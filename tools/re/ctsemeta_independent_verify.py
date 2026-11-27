"""Walk retail format-8 OBJS independently for T01-050d.

Sources: docs/re/ctsemeta-{container,types,objects}.md.
Usage: python tools/re/ctsemeta_independent_verify.py [install directory]
       The install directory defaults to SS2_DATA_DIR.
"""

import argparse
from collections import Counter
from functools import lru_cache
import os
from pathlib import Path
from struct import Struct
from zipfile import ZipFile


MAGIC = b"CTSEMETA"
FOOTER = b"METAEND "
EMPTY_EDOB = b"EDOB\0\0\0\0" + FOOTER
NO_PARENT = 0xFFFFFFFF
U32 = Struct("<I")
PAIR = Struct("<II")
WIDTHS = {
    b"UBYTE": 1, b"SBYTE": 1, b"UWORD": 2, b"SWORD": 2,
    b"ULONG": 4, b"SLONG": 4, b"FLOAT": 4, b"IDENT": 4,
}
RETAIL_ARCHIVES = 14
RETAIL_VERSION_8 = 9269
RETAIL_ELIGIBLE = 9090


class WalkError(ValueError):
    """The first point where the documented framing or encoding failed."""

    def __init__(self, offset, reason, object_type="<frame>"):
        self.object_type = object_type
        super().__init__(f"{object_type} at 0x{offset:x}: {reason}")


class Cursor:
    def __init__(self, data):
        self.data = data
        self.pos = 0
        self.limit = len(data)
        self.object_type = "<frame>"

    def fail(self, reason):
        raise WalkError(self.pos, reason, self.object_type)

    def advance(self, size):
        start = self.pos
        if size > self.limit - start:
            self.fail(f"{size} bytes cross the section end")
        self.pos += size
        return start

    def uint(self):
        return U32.unpack_from(self.data, self.advance(4))[0]

    def pair(self):
        return PAIR.unpack_from(self.data, self.advance(8))

    def text(self):
        size = self.uint()
        start = self.advance(size)
        return self.data[start:self.pos]

    def skip_text(self):
        self.advance(self.uint())

    def tag(self, expected):
        start = self.pos
        if not self.data.startswith(expected, start, self.limit):
            self.fail(f"expected {expected!r}, found {self.data[start:start + len(expected)]!r}")
        self.advance(len(expected))

    def count(self, minimum):
        count = self.uint()
        if count * minimum > self.limit - self.pos:
            self.fail(f"count {count} cannot fit in the section")
        return count


def type_table(cursor):
    cursor.tag(b"EXTY")
    external = {}
    for _ in range(cursor.count(8)):
        type_id = cursor.uint()
        external[type_id] = cursor.text()
    cursor.tag(b"INTY")
    expected = cursor.count(24)
    internal = {}
    while not cursor.data.startswith(b"EXOB", cursor.pos):
        if cursor.data.startswith(b"ADIM", cursor.pos):
            cursor.tag(b"ADIM")
            cursor.advance(cursor.uint() * 4)
            continue
        cursor.tag(b"DTTY")
        type_id = cursor.uint()
        if type_id in external or type_id in internal or len(internal) == expected:
            cursor.fail("unexpected or duplicate DTTY id")
        name = cursor.text()
        kind = cursor.uint()
        first, second = cursor.pair()
        if cursor.data.startswith(b"STMB", cursor.pos):
            cursor.tag(b"STMB")
            members = []
            for _ in range(cursor.count(8)):
                cursor.skip_text()
                members.append(cursor.uint())
            internal[type_id] = (name, "class", second, tuple(members))
        else:
            if (kind, first) == (1, 0):
                cursor.uint()  # Primitive alignment, absent from short forms.
            internal[type_id] = (name, kind, first, second)
    if len(internal) != expected:
        cursor.fail("DTTY count differs from INTY")
    return external, internal


def skip_values(cursor, external, internal, records):
    @lru_cache(maxsize=None)
    def encoding(type_id):
        if type_id in external:
            name = external[type_id]
            if name in WIDTHS:
                return ("bytes", WIDTHS[name])
            cursor.fail(f"no inline width for external type {name!r}")
        if type_id not in internal:
            cursor.fail(f"undefined type {type_id}")
        name, kind, first, second = internal[type_id]
        if kind == "class":
            if name == b"CMetaHandle":
                return ("bytes", 4)
            if name == b"CTransString":
                return ("translated string",)
            return ("class", first, second)
        if (kind, first) == (1, 13):
            return ("alias", second)
        if (kind, first) == (1, 0) and second in (1, 2, 4, 8):
            return ("string",) if name == b"CString" else ("bytes", second)
        if kind == 0:
            if first == 1 and second in (1, 2, 4):
                return ("bytes", second)
            if first in (2, 11, 12):
                return ("bytes", 4)
            if first == 8:
                return ("references",)
            if first in (6, 7):
                return ("inline", b"STAR" if first == 6 else b"SSAR", second)
            if first == 4:
                _, bracket, length = name.rpartition(b"[")
                if bracket and length.endswith(b"]") and length[:-1].isdigit():
                    return ("fixed", int(length[:-1]), second)
        cursor.fail(f"unsupported type {name!r} with shape {(kind, first, second)}")

    def repeated(count, element, depth):
        if count == 0:
            return
        shape = encoding(element)
        if shape[0] == "bytes":
            cursor.advance(count * shape[1])
            return
        for _ in range(count):
            value(element, depth)

    # Work on this retail corpus is proportional to the object region. The allowance
    # also covers empty containers and classes that do not advance the cursor.
    budget = 8 * (cursor.limit - cursor.pos) + 1024

    def value(type_id, depth=0):
        nonlocal budget
        budget -= 1
        if depth > 96 or budget < 0:
            cursor.fail("value traversal limit exceeded")
        shape, *args = encoding(type_id)
        if shape == "bytes":
            cursor.advance(args[0])
        elif shape == "alias":
            value(args[0], depth + 1)
        elif shape == "class":
            parent, members = args
            if parent != NO_PARENT:
                value(parent, depth + 1)
            for member in members:
                value(member, depth + 1)
        elif shape == "string":
            cursor.skip_text()
        elif shape == "translated string":
            cursor.tag(b"DTRS")
            cursor.skip_text()
            cursor.skip_text()
        elif shape == "references":
            cursor.tag(b"DCON")
            cursor.advance(cursor.uint() * 4)
        elif shape == "inline":
            marker, element = args
            cursor.tag(marker)
            repeated(cursor.uint(), element, depth + 1)
        else:
            count, element = args
            repeated(count, element, depth + 1)

    for object_id, type_id in records:
        name = internal[type_id][0] if type_id in internal else external.get(type_id)
        cursor.object_type = name.decode("latin-1") if name else f"type {type_id}"
        if cursor.pair() != (object_id, type_id):
            cursor.fail(f"OBJS header differs from OBTY for object {object_id}")
        value(type_id)


def walk(data):
    """Return (main-record count, EDOB offset), or None for a nonempty EDOB."""
    cursor = Cursor(data)
    cursor.tag(MAGIC)
    if cursor.pair() != (0x1234ABCD, 8):
        cursor.fail("expected the little-endian version 8 header")
    if not data.endswith(FOOTER):
        cursor.fail("missing METAEND footer")
    cursor.skip_text()
    cursor.tag(b"INFO")
    cursor.advance(20)
    for section, words in ((b"RFIL", 2), (b"IDNT", 1)):
        cursor.tag(section)
        for _ in range(cursor.count(4 * (words + 1))):
            cursor.advance(4 * words)
            cursor.skip_text()
    external, internal = type_table(cursor)
    cursor.tag(b"EXOB")
    cursor.advance(cursor.uint() * 16)
    cursor.tag(b"OBTY")
    length = cursor.count(8)
    offset = cursor.advance(length * 8)
    records = PAIR.iter_unpack(memoryview(data)[offset:cursor.pos])
    cursor.tag(b"EDTY")
    extra = cursor.count(8)
    if extra:
        if data.endswith(EMPTY_EDOB):
            cursor.fail("nonempty EDTY conflicts with the zero-record EDOB footer")
        return None
    cursor.tag(b"OBJS")
    if cursor.uint() != length:
        cursor.fail("OBJS count differs from OBTY")
    end = len(data) - len(EMPTY_EDOB)
    cursor.limit = end
    skip_values(cursor, external, internal, records)
    cursor.limit = len(data)
    if not data.endswith(EMPTY_EDOB):
        cursor.object_type = "<frame>"
        cursor.fail("missing zero-record EDOB footer")
    if cursor.pos != end:
        cursor.fail(f"OBJS did not end at the expected EDOB offset 0x{end:x}")
    return length, end


def scan(install):
    archives = sorted(install.glob("*.gro"))
    if not archives:
        raise SystemExit(f"No .gro archives found in {install}")
    counts = Counter()
    failures = Counter()
    for archive in archives:
        with ZipFile(archive) as bundle:
            for entry in bundle.infolist():
                if entry.is_dir():
                    continue
                counts["entries"] += 1
                with bundle.open(entry) as stream:
                    header = stream.read(16)
                    if not header.startswith(MAGIC):
                        continue
                    if len(header) >= 16 and U32.unpack_from(header, 12)[0] in (4, 5, 6, 7):
                        counts["legacy containers"] += 1
                        continue
                    counts["format-8 candidates"] += 1
                    try:
                        result = walk(header + stream.read())
                    except WalkError as error:
                        failures[error.object_type] += 1
                        print(f"{archive.name} / {entry.filename}: {error}", flush=True)
                        continue
                    if result is None:
                        counts["nonempty EDTY excluded"] += 1
                    else:
                        counts["OBJS to zero-record EDOB"] += 1
                        counts["records followed"] += result[0]
        print(f"{archive.name}: {counts['OBJS to zero-record EDOB']} passed cumulatively",
              flush=True)
    for label, total in sorted(counts.items()):
        print(f"{label}: {total}")
    print(f"archives: {len(archives)}")
    for object_type, total in sorted(failures.items()):
        print(f"failures for {object_type}: {total}")
    print(f"total failures: {failures.total()}")
    coverage = (len(archives), counts["format-8 candidates"], counts["OBJS to zero-record EDOB"])
    expected = (RETAIL_ARCHIVES, RETAIL_VERSION_8, RETAIL_ELIGIBLE)
    if coverage != expected:
        print(f"retail coverage incomplete or different: got {coverage}, expected {expected}")
    return int(bool(failures) or coverage != expected)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("install", nargs="?", type=Path, default=os.getenv("SS2_DATA_DIR"))
    args = parser.parse_args()
    if args.install is None:
        parser.error("supply the install directory or set SS2_DATA_DIR")
    return scan(args.install)


if __name__ == "__main__":
    raise SystemExit(main())
