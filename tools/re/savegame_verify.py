"""Prints the data-side numbers quoted in docs/re/save-game.md (read-only).

A saved game cannot be reproduced from the install (there is none in it), so the spec's rules come from the
binaries. What the *data* can check is everything a save game is built on:

  1. the world root: `CWorld.18` (the pointer the type table declares as `CWorldSaveData*`) is null in every
     version of every `.wld`, and `CWorld.19` (the editor's save counter) never decreases from one patch level
     of a world to the next (the rule the engine uses to refuse a save from a newer world);
  2. the classes a save game adds (`CWorldSaveData`, `CPlayerSave`, `CStoredLevelStatus`, `CGameInfoEntity`,
     `CPlayerProfile`, ...) are *not* defined by any data file: only the pointer and handle types that name
     them are;
  3. the `.preload` files (the text lists a save game copies): how many, how many lines, and whether the line
     for the world file itself is present;
  4. the three demo recordings of the install (`Content/SeriousSam2/Demos`): header, build string, level path
     and the `.nfo` text next to each.

Usage: python -I tools/re/savegame_verify.py [DATA_DIR] [--skip-worlds]    (DATA_DIR defaults to $SS2_DATA_DIR)
       a full run takes about 6 minutes; --skip-worlds leaves out part 1 and takes about 3.
"""
import collections
import os
import re
import struct
import sys
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, _natural, parse, u32  # noqa: E402

NULL = 0xFFFFFFFF
SAVE_CLASSES = ("CWorldSaveData", "CPlayerSave", "CStoredLevelStatus", "CStoredChapterStatus", "CGameInfoEntity",
                "CGameStats", "CPlayerProfile", "CPlayerPuppetEntity", "CPlayerBrainEntity", "CPlayerInventory",
                "CSecretEntityData", "CMissionObjectiveData", "CNavigationalBeaconData")


def archives(install):
    return sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural)


def world_root(d, r):
    """The `CWorld` object, decoded without decoding the objects after it (it is the second object of a file)."""
    dec = TreeDec(d, r)
    pos = r["data_start"]
    for oid, tid in r["obty"]:
        assert u32(d, pos) == oid and u32(d, pos + 4) == tid
        value, pos = dec.val(tid, pos + 8)
        if isinstance(value, Obj) and value.type == "CWorld":
            return value
    return None


def part1_worlds(install):
    """CWorld.18 / CWorld.19 of every version of every world, in archive order."""
    versions = collections.defaultdict(list)             # path -> [(archive, save counter, save-data pointer null?)]
    types_seen = collections.Counter()
    for f in archives(install):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or not i.filename.lower().endswith(".wld"):
                    continue
                d = z.read(i)
                if d[:8] != b"CTSEMETA" or u32(d, 12) != 8:
                    continue
                r = parse(d)
                w = world_root(d, r)
                assert w is not None, i.filename
                versions[i.filename.lower()].append((f, w.get("CWorld.19"), w.get("CWorld.18").oid == NULL, i.file_size))
                for t in r["types"].values():
                    types_seen[t.name] += 1
    total = sum(len(v) for v in versions.values())
    multi = {k: v for k, v in versions.items() if len(v) > 1}
    mono = sum(1 for v in multi.values() if all(a[1] <= b[1] for a, b in zip(v, v[1:])))
    strict = sum(1 for v in multi.values() if all(a[1] < b[1] for a, b in zip(v, v[1:])))
    allnull = sum(1 for v in versions.values() for x in v if x[2])
    counters = [x[1] for v in versions.values() for x in v]
    print("worlds: %d paths, %d versions in all archives" % (len(versions), total))
    print("CWorld.18 (declared CWorldSaveData*) is null in %d of %d versions" % (allnull, total))
    print("CWorld.19 (save counter): min %d, max %d over all versions" % (min(counters), max(counters)))
    print("worlds present in more than one archive: %d; counter never decreases in %d of them; strictly rises in %d"
          % (len(multi), mono, strict))
    first = next(iter(sorted(multi)))
    print("example %s: %s" % (first, [(a, c) for a, c, _, _ in multi[first]]))
    final = [v[-1][3] for v in versions.values()]
    print("size of the final version of each of the %d worlds: smallest %d bytes, largest %d bytes, median %d bytes"
          % (len(final), min(final), max(final), sorted(final)[len(final) // 2]))
    for name in ("CWorldSaveData*", "Handle<CGameInfoEntity>"):
        print("type name %-26s present in the type tables of %d world versions" % (name, types_seen[name]))
    return types_seen


def part2_classes(install, types_seen):
    """Which of the classes a save adds are defined (have members) in any format-8 file's type table."""
    defined = collections.Counter()
    files = 0
    for f in archives(install):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.is_dir() or i.file_size < 64:
                    continue
                with z.open(i) as fh:
                    head = fh.read(16)
                if head[:8] != b"CTSEMETA" or u32(head, 12) != 8:
                    continue
                r = parse(z.read(i))
                files += 1
                for t in r["types"].values():
                    if t.members is not None and t.name in SAVE_CLASSES:
                        defined[t.name] += 1
    print("format-8 files read: %d (all archives, all versions)" % files)
    for c in SAVE_CLASSES:
        print("  class %-24s defined with members in %d files" % (c, defined[c]))


def part3_preload(install):
    n = lines = bad = other = with_world = 0
    for f in archives(install):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            names = {i.filename.lower(): i for i in z.infolist()}
            for lname, i in names.items():
                if not lname.endswith(".wld.preload"):
                    continue
                n += 1
                text = z.read(i).decode("latin1")
                rows = [x for x in text.split("\r\n") if x]
                lines += len(rows)
                bad += sum(1 for x in rows if x == "-1:")
                other += sum(1 for x in rows if x != "-1:" and not re.fullmatch(r"\d+:.+", x))
                world = lname[:-len(".preload")]
                with_world += any(x.split(":", 1)[1].lower() == world for x in rows)
    print(".wld.preload files: %d, lines %d (of which the bare line '-1:' %d, other lines not of the form "
          "<size>:<path> %d), files that list their own world: %d" % (n, lines, bad, other, with_world))


def part4_demos(install):
    folder = os.path.join(install, "Content", "SeriousSam2", "Demos")
    if not os.path.isdir(folder):
        print("no demo folder")
        return
    for name in sorted(os.listdir(folder)):
        if not name.lower().endswith(".dem"):
            continue
        d = open(os.path.join(folder, name), "rb").read()
        magic, hlen = d[:8], u32(d, 8)
        t1 = d[12:16] == b"TIME" and struct.unpack_from("<Q", d, 16)[0]
        t2 = d[24:28] == b"TIME" and struct.unpack_from("<Q", d, 28)[0]
        ver = re.search(rb"\$(Version: [^$]*)\$", d[:400])
        world = re.search(rb"Content/[^\x00]*\.wld", d[:400])
        nfo = open(os.path.join(folder, name[:-4] + ".nfo"), "rb").read().decode("utf-8-sig")
        print("%s: %d bytes, magic %r, header length %d, start time %.3f s (second stamp equal: %s), ends with %r"
              % (name, len(d), magic, hlen, t1 / 2 ** 32, t1 == t2, d[-4:]))
        print("   build string %r" % (ver.group(1).decode() if ver else None))
        print("   level %r" % (world.group(0).decode() if world else None))
        print("   .nfo lines: %r" % [x.split("=")[0] for x in nfo.splitlines() if x])


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR")
    if not install:
        sys.exit("usage: savegame_verify.py DATA_DIR")
    types_seen = {}
    if "--skip-worlds" not in sys.argv:
        types_seen = part1_worlds(install)
    part2_classes(install, types_seen)
    part3_preload(install)
    part4_demos(install)


if __name__ == "__main__":
    main()
