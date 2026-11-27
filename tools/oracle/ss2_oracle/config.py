"""The local target map: field name -> (module, offset chain, value type).

The file is machine-local and git-ignored (``targets.json`` / ``*.local.json``); the repository
holds only the format and fake examples. See docs/ORACLE_HARNESS.md section "Config format".
"""

import json
import math
import os
import re
from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple

FORMAT_VERSION = 1

# type name -> (struct format, size in bytes, is_float)
TYPES: Dict[str, Tuple[str, int, bool]] = {
    "f32": ("<f", 4, True),
    "f64": ("<d", 8, True),
    "i8": ("<b", 1, False),
    "u8": ("<B", 1, False),
    "i16": ("<h", 2, False),
    "u16": ("<H", 2, False),
    "i32": ("<i", 4, False),
    "u32": ("<I", 4, False),
    "i64": ("<q", 8, False),
    "u64": ("<Q", 8, False),
}

# CSV columns the tool writes itself; a field may not use these names.
RESERVED_NAMES = frozenset(["sample", "t_wall_s", "jitter_s", "read_s", "valid", "n_invalid"])

NAME_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_.]*$")
MODULE_RE = re.compile(r"^[A-Za-z0-9_.\-]+$")
MAX_CHAIN = 16
MAX_OFFSET = 2**32 - 1
MAX_REPEAT = 4096


class ConfigError(ValueError):
    """The config file is malformed. The message names the field and the rule."""


@dataclass(frozen=True)
class Repeat:
    """Replicate one field ``count`` times; element i adds ``i * stride`` to chain step ``at``."""

    count: int
    stride: int
    at: int


@dataclass(frozen=True)
class FieldSpec:
    name: str
    module: str
    offsets: Tuple[int, ...]
    type: str
    scale: Optional[float] = None
    repeat: Optional[Repeat] = None
    # handles[i] is True when the value found at the address before step i is a 32-bit handle
    # (resolved through Config.handle_table) rather than a pointer. handles[0] is always False.
    handles: Tuple[bool, ...] = ()


@dataclass(frozen=True)
class HandleTable:
    """How a 32-bit object handle is turned into an object address.

    A handle is ``(generation << index_bits) | index``. The table is an array of ``record_size``-byte
    records; record ``index`` holds the generation at ``generation_at`` and the object pointer at
    ``object_at``. ``table`` and ``count`` are offsets from the module base of the variables that
    hold the table address and the number of records.
    """

    module: str
    table: int
    count: int
    record_size: int
    generation_at: int
    object_at: int
    index_bits: int = 24


@dataclass(frozen=True)
class Config:
    fields: Tuple[FieldSpec, ...]
    pointer_size: Optional[int] = None  # None: use what the process is
    handle_table: Optional[HandleTable] = None

    def column_names(self) -> List[str]:
        out = []
        for f in self.fields:
            if f.repeat is None:
                out.append(f.name)
            else:
                out.extend("%s[%d]" % (f.name, i) for i in range(f.repeat.count))
        return out


def _reject_duplicates(pairs):
    seen = {}
    for key, value in pairs:
        if key in seen:
            raise ConfigError("duplicate key %r in the config file" % key)
        seen[key] = value
    return seen


def _parse_offset(raw, where: str) -> int:
    if isinstance(raw, bool):
        raise ConfigError("%s: offset must be a number or a hex string, not a boolean" % where)
    if isinstance(raw, int):
        value = raw
    elif isinstance(raw, str):
        text = raw.strip().lower()
        try:
            value = int(text, 16) if text.startswith(("0x", "-0x")) else int(text, 10)
        except ValueError:
            raise ConfigError("%s: %r is not a number" % (where, raw)) from None
    else:
        raise ConfigError('%s: offset must be an integer or a string like "0x10"' % where)
    if value < 0:
        raise ConfigError("%s: offsets must not be negative (got %d)" % (where, value))
    if value > MAX_OFFSET:
        raise ConfigError("%s: offset %d is larger than 32 bits" % (where, value))
    return value


def _parse_field(name: str, raw) -> FieldSpec:
    where = "field %r" % name
    if not isinstance(name, str) or not NAME_RE.match(name):
        raise ConfigError("%s: name must match %s" % (where, NAME_RE.pattern))
    if name in RESERVED_NAMES:
        raise ConfigError("%s: name is reserved for a column the tool writes itself" % where)
    if not isinstance(raw, dict):
        raise ConfigError("%s: must be an object" % where)
    unknown = set(raw) - {"module", "offsets", "type", "scale", "repeat"}
    if unknown:
        raise ConfigError("%s: unknown key(s) %s" % (where, ", ".join(sorted(unknown))))

    module = raw.get("module")
    if not isinstance(module, str) or not module:
        raise ConfigError('%s: "module" (a module file name such as "Engine.dll") is required' % where)
    if not MODULE_RE.match(module):
        raise ConfigError("%s: 'module' must be a bare file name, not a path (got %r)" % (where, module))

    offsets_raw = raw.get("offsets")
    if not isinstance(offsets_raw, list) or not offsets_raw:
        raise ConfigError("%s: 'offsets' must be a non-empty list" % where)
    if len(offsets_raw) > MAX_CHAIN:
        raise ConfigError("%s: 'offsets' has more than %d steps" % (where, MAX_CHAIN))
    offsets_list = []
    handles_list = []
    for position, item in enumerate(offsets_raw):
        if isinstance(item, dict):
            if set(item) != {"offset", "handle"} or item["handle"] is not True:
                raise ConfigError('%s: a step object must be exactly {"offset": N, "handle": true}' % where)
            if position == 0:
                raise ConfigError("%s: the first step is added to the module base, so it cannot be a handle step" % where)
            offsets_list.append(_parse_offset(item["offset"], where))
            handles_list.append(True)
        else:
            offsets_list.append(_parse_offset(item, where))
            handles_list.append(False)
    offsets = tuple(offsets_list)
    handles = tuple(handles_list)

    ftype = raw.get("type")
    if ftype not in TYPES:
        raise ConfigError(
            "%s: 'type' must be one of %s (got %r)" % (where, ", ".join(sorted(TYPES)), ftype)
        )

    scale = raw.get("scale")
    if scale is not None:
        if isinstance(scale, bool) or not isinstance(scale, (int, float)) or not math.isfinite(scale):
            raise ConfigError("%s: 'scale' must be a finite number" % where)
        scale = float(scale)

    repeat = None
    if "repeat" in raw:
        r = raw["repeat"]
        if not isinstance(r, dict) or set(r) != {"count", "stride", "at"}:
            raise ConfigError("%s: 'repeat' needs exactly count, stride and at" % where)
        for key in ("count", "stride", "at"):
            if isinstance(r[key], bool) or not isinstance(r[key], int):
                raise ConfigError("%s: repeat.%s must be an integer" % (where, key))
        if not 1 <= r["count"] <= MAX_REPEAT:
            raise ConfigError("%s: repeat.count must be 1..%d" % (where, MAX_REPEAT))
        if not 0 <= r["stride"] <= MAX_OFFSET:
            raise ConfigError("%s: repeat.stride must be 0..2^32-1" % where)
        if not 0 <= r["at"] < len(offsets):
            raise ConfigError("%s: repeat.at must index a step of 'offsets'" % where)
        repeat = Repeat(r["count"], r["stride"], r["at"])
    return FieldSpec(name, module, offsets, ftype, scale, repeat, handles)


def _parse_handle_table(raw) -> HandleTable:
    where = "handle_table"
    if not isinstance(raw, dict):
        raise ConfigError("%s: must be an object" % where)
    keys = {"module", "table", "count", "record_size", "generation_at", "object_at", "index_bits"}
    unknown = set(raw) - keys
    if unknown:
        raise ConfigError("%s: unknown key(s) %s" % (where, ", ".join(sorted(unknown))))
    missing = (keys - {"index_bits"}) - set(raw)
    if missing:
        raise ConfigError("%s: missing key(s) %s" % (where, ", ".join(sorted(missing))))
    module = raw["module"]
    if not isinstance(module, str) or not MODULE_RE.match(module):
        raise ConfigError("%s: 'module' must be a bare module file name" % where)
    record_size = _parse_offset(raw["record_size"], where)
    if not 4 <= record_size <= 4096:
        raise ConfigError("%s: record_size must be 4..4096" % where)
    generation_at = _parse_offset(raw["generation_at"], where)
    object_at = _parse_offset(raw["object_at"], where)
    if generation_at + 4 > record_size or object_at + 4 > record_size:
        raise ConfigError("%s: generation_at and object_at must fit in a record" % where)
    index_bits = raw.get("index_bits", 24)
    if isinstance(index_bits, bool) or not isinstance(index_bits, int) or not 1 <= index_bits <= 31:
        raise ConfigError("%s: index_bits must be an integer 1..31" % where)
    return HandleTable(
        module,
        _parse_offset(raw["table"], where),
        _parse_offset(raw["count"], where),
        record_size,
        generation_at,
        object_at,
        index_bits,
    )


def parse_config(text: str) -> Config:
    """Parse and validate the JSON text of a config file."""
    try:
        doc = json.loads(text, object_pairs_hook=_reject_duplicates)
    except json.JSONDecodeError as exc:
        raise ConfigError("not valid JSON: %s" % exc) from None
    if not isinstance(doc, dict):
        raise ConfigError("the top level must be an object")
    unknown = set(doc) - {"format", "pointer_size", "fields", "note", "handle_table"}
    if unknown:
        raise ConfigError("unknown top-level key(s) %s" % ", ".join(sorted(unknown)))
    if doc.get("format") != FORMAT_VERSION:
        raise ConfigError("'format' must be %d (got %r)" % (FORMAT_VERSION, doc.get("format")))
    pointer_size = doc.get("pointer_size")
    if pointer_size is not None and pointer_size not in (4, 8):
        raise ConfigError("'pointer_size' must be 4 or 8 when present")
    fields_raw = doc.get("fields")
    if not isinstance(fields_raw, dict) or not fields_raw:
        raise ConfigError("'fields' must be a non-empty object")
    fields = tuple(_parse_field(name, raw) for name, raw in fields_raw.items())
    handle_table = _parse_handle_table(doc["handle_table"]) if "handle_table" in doc else None
    if handle_table is None:
        for spec in fields:
            if any(spec.handles):
                raise ConfigError("field %r uses a handle step but the config has no 'handle_table'" % spec.name)
    # Column names cannot collide: JSON object keys are unique (duplicates are rejected above) and
    # "name[i]" cannot be a plain field name because the name pattern excludes brackets.
    return Config(fields, pointer_size, handle_table)


def default_config_path() -> str:
    base = os.environ.get("LOCALAPPDATA") or os.path.join(os.path.expanduser("~"), ".local", "share")
    return os.path.join(base, "ss2rs-oracle", "targets.json")


def load_config(path: str) -> Config:
    try:
        with open(path, "r", encoding="utf-8") as handle:
            text = handle.read()
    except OSError as exc:
        raise ConfigError("cannot read the config file %s: %s" % (path, exc)) from None
    try:
        return parse_config(text)
    except ConfigError as exc:
        raise ConfigError("%s: %s" % (path, exc)) from None
