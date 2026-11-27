"""Evidence for docs/re/behavior-classes.md open question 8 (T01-159): data leftovers of the AI
behaviour family.

Read-only on the install and on any installed mods (CLEANROOM.md: this script reads DATA files
only, never the game's binaries).

It answers three things from the data:
  1. which object class owns the world-stored `CAIMomentHandler`s (564 counted, holding the 614
     `SBAShoot` and 16 `SBAAnimation` of 23 worlds): for every `.wld` top-level object whose type
     declares a `Handle<CAIMomentHandler>` member, how many are non-null and what owns them.
  2. whether any mod's data creates the two obsolete classes of the family
     (`CTurretBehaviorEntity`, `CRideFoeManagerEntity`; `behavior-classes.md` Invariant 5): scans
     the base game's own `.gro` archives plus every installed mod's archives (`Content/<Mod>/*.gro`,
     recursively one level, patches included) for the type appearing at all (declared in a file's
     type table) and for any stored instance.
  3. how many tactics (`STacticEntity`) have no marker (`en_henTacticMarker` null) broken down by
     their mover's class, so a reader can see whether any marker-walking mover (`CMCMarkersMoving*`)
     is ever started without one.

Usage: python -I tools/re/behavior_leftovers_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, parse  # noqa: E402
from walk_objects import u32  # noqa: E402

NULL = 0xFFFFFFFF
OBSOLETE = ("CTurretBehaviorEntity", "CRideFoeManagerEntity")


def _natural(name):
    lower = name.lower()
    return (1 if "patch" in lower else 0, [int(t) if t.isdigit() else t for t in __import__("re").split(r"(\d+)", lower)])


def gro_files(root):
    """Every `.gro` directly under `root`, patches last (same order rule as ctse_values.merged_files)."""
    if not os.path.isdir(root):
        return []
    return sorted((os.path.join(root, f) for f in os.listdir(root) if f.lower().endswith(".gro")), key=lambda p: _natural(os.path.basename(p)))


def mod_roots(install):
    """`Content/<Mod>` directories that hold their own `.gro` archives (installed mods)."""
    content = os.path.join(install, "Content")
    if not os.path.isdir(content):
        return []
    return [os.path.join(content, name) for name in sorted(os.listdir(content))
            if os.path.isdir(os.path.join(content, name)) and gro_files(os.path.join(content, name))]


def scan_source(label, gro_paths, suffix):
    """{path: bytes} of every `suffix` file across `gro_paths` (later ones override earlier), tagged with `label`."""
    out = {}
    for path in gro_paths:
        with zipfile.ZipFile(path) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower().endswith(suffix):
                    d = z.read(i)
                    if d[:8] == b"CTSEMETA" and u32(d, 12) == 8:
                        out[(label, i.filename.lower())] = d
    return out


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not install:
        print("usage: behavior_leftovers_verify.py <install>  (or set SS2_DATA_DIR)")
        return 1

    sources = [("base", gro_files(install))]
    for root in mod_roots(install):
        sources.append((os.path.basename(root), gro_files(root)))
    print("Sources:", ", ".join(f"{label} ({len(paths)} .gro)" for label, paths in sources))

    # ---- 1 & 3: .wld analysis (base only; mods do not ship their own .wld in this install) ----
    wld = {}
    for label, paths in sources:
        wld.update(scan_source(label, paths, ".wld"))

    handler_owner_members = collections.defaultdict(collections.Counter)
    handler_nonnull = collections.defaultdict(collections.Counter)
    moverclass_vs_marker = collections.defaultdict(lambda: collections.defaultdict(collections.Counter))
    tactic_total = collections.Counter()
    handler_types_seen = set()

    for (label, name), d in sorted(wld.items()):
        r = parse(d)
        objs, _ = TreeDec(d, r).objects()
        for oid, o in objs.items():
            if not isinstance(o, Obj):
                continue
            for key, val in o:
                cls, _, mem = key.partition(".")
                if isinstance(val, Ref) and mem.lower().find("momenthandler") != -1:
                    handler_owner_members[label][f"{o.type}.{mem}"] += 1
                    if val.oid != NULL:
                        handler_nonnull[label][f"{o.type}.{mem}"] += 1
                        target = objs.get(val.oid)
                        if isinstance(target, Obj):
                            handler_types_seen.add(target.type)
            if o.type == "STacticEntity":
                tactic_total[label] += 1
                marker = o.get("STacticEntity.en_henTacticMarker")
                mover = o.get("STacticEntity.en_hmcCharacterMover")
                mover_cls = "<none>"
                if isinstance(mover, Ref) and mover.oid != NULL:
                    target = objs.get(mover.oid)
                    if isinstance(target, Obj):
                        mover_cls = target.type
                elif isinstance(mover, Obj):
                    mover_cls = mover.type
                has_marker = isinstance(marker, Ref) and marker.oid != NULL
                moverclass_vs_marker[label][mover_cls]["marker" if has_marker else "no marker"] += 1

    for label in [s[0] for s in sources]:
        print(f"\n-- 1. CAIMomentHandler ownership (.wld, {label}) --")
        print("Member paths holding Handle<CAIMomentHandler>, non-null / total:")
        for key, total in sorted(handler_owner_members[label].items()):
            print(f"  {key}: {handler_nonnull[label][key]} / {total}")

        print(f"\n-- 3. STacticEntity marker presence by mover class ({label}) --")
        print(f"Total STacticEntity objects: {tactic_total[label]}")
        for mover_cls, c in sorted(moverclass_vs_marker[label].items()):
            print(f"  {mover_cls}: marker {c['marker']}, no marker {c['no marker']}")
    print("\nResolved target object types of the non-null CAIMomentHandler handles (any source):", sorted(handler_types_seen) or "(none resolved)")

    # ---- 2: obsolete-class presence, base + every installed mod ----
    print("\n-- 2. Obsolete classes (%s) across base + mods --" % ", ".join(OBSOLETE))
    for label, paths in sources:
        for suffix in (".wld", ".cb", ".ep"):
            files = scan_source(label, paths, suffix)
            type_hits = collections.Counter()
            inst_hits = collections.Counter()
            for (lbl, name), d in files.items():
                r = parse(d)
                names_here = {t.name for t in r["types"].values()}
                for cls in OBSOLETE:
                    if cls in names_here:
                        type_hits[cls] += 1
                if type_hits:
                    objs, _ = TreeDec(d, r).objects()
                    for o in objs.values():
                        if isinstance(o, Obj) and o.type in OBSOLETE:
                            inst_hits[o.type] += 1
            if files:
                print(f"  {label} {suffix}: {len(files)} files; type declared: {dict(type_hits)}; instances: {dict(inst_hits)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
