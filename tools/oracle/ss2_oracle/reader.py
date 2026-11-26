"""Resolve configured fields to numbers: pointer chains, type decoding, number formatting."""

import struct
import time
from typing import Callable, Dict, List, Optional, Tuple

from .backend import Backend, ModuleInfo, ProcessGone, ReadError
from .config import TYPES, Config, FieldSpec


def format_number(value, ftype: str, scale: Optional[float] = None) -> str:
    """Text of one decoded value for the CSV.

    Integers are written exactly. Floats use the shortest text that reads back to the same value
    of the field's own width (a 32-bit float is not printed with 17 digits).
    """
    _fmt, _size, is_float = TYPES[ftype]
    if scale is not None:
        return _format_double(float(value) * scale)
    if not is_float:
        return str(int(value))
    if ftype == "f32":
        return _format_f32(value)
    return _format_double(value)


def _format_double(value: float) -> str:
    if value != value:
        return "nan"
    if value in (float("inf"), float("-inf")):
        return "inf" if value > 0 else "-inf"
    return repr(float(value))


def _format_f32(value: float) -> str:
    if value != value:
        return "nan"
    if value in (float("inf"), float("-inf")):
        return "inf" if value > 0 else "-inf"
    packed = struct.pack("<f", value)
    for digits in range(1, 10):
        text = "%.*g" % (digits, value)
        try:
            if struct.pack("<f", float(text)) == packed:
                return repr(float(text))
        except OverflowError:  # a rounded-up text just past the largest float32
            continue
    return "%.9g" % value


class Sample:
    """One reading of every column. ``cells[i]`` is a CSV string or '' when invalid."""

    __slots__ = ("cells", "n_invalid")

    def __init__(self, cells: List[str], n_invalid: int):
        self.cells = cells
        self.n_invalid = n_invalid

    @property
    def valid(self) -> bool:
        return self.n_invalid == 0


class FieldReader:
    """Reads every field of a config from one backend.

    * The module base is looked up by name at run time (ASLR) and kept; a module that is not
      loaded yet is retried at most every ``module_retry_s`` seconds, and its fields read as invalid.
    * A pointer chain step that reads 0, or memory that cannot be read, makes only that field
      invalid for that sample (an empty cell); it never raises.
    * One sample reads each address at most once (a shared pointer is the same value in
      every field of the row).
    """

    def __init__(
        self,
        backend: Backend,
        config: Config,
        clock: Callable[[], float] = time.monotonic,
        module_retry_s: float = 0.5,
    ):
        self.backend = backend
        self.config = config
        self.pointer_size = backend.pointer_size
        if config.pointer_size is not None and config.pointer_size != self.pointer_size:
            raise ValueError(
                "the config says pointer_size %d but the process uses %d"
                % (config.pointer_size, self.pointer_size)
            )
        self._clock = clock
        self._retry = module_retry_s
        self._bases: Dict[str, int] = {}
        self._last_lookup = -1e18
        self._limit = 1 << (8 * self.pointer_size)
        self._cache: Dict[Tuple[int, int], Optional[bytes]] = {}

    # -- modules --------------------------------------------------------------------------

    def _refresh_modules(self) -> None:
        now = self._clock()
        if now - self._last_lookup < self._retry:
            return
        self._last_lookup = now
        for module in self.backend.list_modules():
            self._bases.setdefault(module.name.lower(), module.base)

    def module_base(self, name: str) -> Optional[int]:
        key = name.lower()
        if key not in self._bases:
            self._refresh_modules()
        return self._bases.get(key)

    # -- memory ---------------------------------------------------------------------------

    def _read(self, address: int, size: int) -> Optional[bytes]:
        key = (address, size)
        if key in self._cache:
            return self._cache[key]
        try:
            data = self.backend.read(address, size)
        except ReadError:
            if not self.backend.is_alive():
                raise ProcessGone("the process exited") from None
            data = None
        else:
            if len(data) != size:
                data = None
        self._cache[key] = data
        return data

    def _resolve(self, spec: FieldSpec, index: int) -> Optional[int]:
        """Address of the value for element ``index`` of ``spec`` (0 for a plain field)."""
        base = self.module_base(spec.module)
        if base is None:
            return None
        offsets = list(spec.offsets)
        if spec.repeat is not None:
            offsets[spec.repeat.at] += index * spec.repeat.stride
        address = base + offsets[0]
        for position in range(1, len(offsets)):
            if address >= self._limit:
                return None
            if spec.handles[position]:
                pointer = self._resolve_handle(address)
            else:
                raw = self._read(address, self.pointer_size)
                pointer = None if raw is None else int.from_bytes(raw, "little")
            if not pointer:
                return None
            address = pointer + offsets[position]
        if address >= self._limit:
            return None
        return address

    def _resolve_handle(self, address: int) -> Optional[int]:
        """The object that the 32-bit handle stored at ``address`` refers to, or None.

        None when the handle is 0, its index is out of range, the record generation does not match
        (a stale handle), or the table cannot be read. The table is read without the game lock, so a
        row can occasionally mix two frames; such a read comes out invalid or stale, never a crash.
        """
        table = self.config.handle_table
        raw = self._read(address, 4)
        if raw is None:
            return None
        handle = int.from_bytes(raw, "little")
        if handle == 0:
            return None
        index = handle & ((1 << table.index_bits) - 1)
        generation = handle >> table.index_bits
        base = self.module_base(table.module)
        if base is None:
            return None
        count_raw = self._read(base + table.count, 4)
        table_raw = self._read(base + table.table, self.pointer_size)
        if count_raw is None or table_raw is None:
            return None
        if index >= int.from_bytes(count_raw, "little"):
            return None
        records = int.from_bytes(table_raw, "little")
        if records == 0:
            return None
        record = records + index * table.record_size
        if record + table.record_size > self._limit:
            return None
        gen_raw = self._read(record + table.generation_at, 4)
        obj_raw = self._read(record + table.object_at, self.pointer_size)
        if gen_raw is None or obj_raw is None:
            return None
        if int.from_bytes(gen_raw, "little") != generation:
            return None
        return int.from_bytes(obj_raw, "little") or None

    def _read_value(self, spec: FieldSpec, index: int) -> str:
        address = self._resolve(spec, index)
        if address is None:
            return ""
        fmt, size, _ = TYPES[spec.type]
        raw = self._read(address, size)
        if raw is None:
            return ""
        (value,) = struct.unpack(fmt, raw)
        return format_number(value, spec.type, spec.scale)

    def sample(self) -> Sample:
        """Read every column once. Raises ProcessGone when the process has exited."""
        self._cache = {}
        cells: List[str] = []
        invalid = 0
        for spec in self.config.fields:
            count = 1 if spec.repeat is None else spec.repeat.count
            for index in range(count):
                cell = self._read_value(spec, index)
                if cell == "":
                    invalid += 1
                cells.append(cell)
        return Sample(cells, invalid)

    def describe(self) -> List[Tuple[str, str]]:
        """(column, type) pairs, for the sidecar."""
        out: List[Tuple[str, str]] = []
        for spec, name_count in ((s, 1 if s.repeat is None else s.repeat.count) for s in self.config.fields):
            if spec.repeat is None:
                out.append((spec.name, spec.type))
            else:
                out.extend(("%s[%d]" % (spec.name, i), spec.type) for i in range(name_count))
        return out


def missing_modules(config: Config, modules: List[ModuleInfo]) -> List[str]:
    """Names the config uses that the process has not loaded."""
    loaded = {m.name.lower() for m in modules}
    wanted = []
    for spec in config.fields:
        if spec.module.lower() not in loaded and spec.module not in wanted:
            wanted.append(spec.module)
    return wanted
