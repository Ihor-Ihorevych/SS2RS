"""Re-derives T01-424's `CBloomProperties`/`CBloomRenderable` member mapping from the shipped `.wld`
data alone (read-only; no binary/decompiler input).

Both classes' own CTSEMETA type tables are stored in every world file (`docs/re/ctsemeta-types.md`):
each class's `STMB` member list is read in **file declaration order**, which is the same order the
`Sam2Game.dll` record-to-renderable copy routine reads/writes (`docs/re/environment.md`, "Bloom").
`CBloomProperties`' own table happens to declare a member literally named "21" **twice** — a genuine
quirk of the shipped data, not a bug in this script or in `environment-schema.md` (whose aggregation
by name alone merges the two into one misleadingly-wide "CBloomProperties.21" row). This script reads
each `CBloomProperties`/`CBloomRenderable` instance **by STMB position**, so the two "21"s (and every
other member) are told apart, and prints, per world, every bloom entity's record and renderable values
side by side with the position-correct role labels.

Usage: python -I tools/re/wld_bloom_members_verify.py [DATA_DIR] [--worlds "dm_hole,1_3_village,..."]
DATA_DIR defaults to $SS2_DATA_DIR; --worlds limits output to worlds whose filename contains one of the
given (comma-separated, case-insensitive) substrings (default: all worlds with a bloom entity).
"""
import os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import TreeDec, merged_files, parse  # noqa: E402

# Position (0-based) -> (role, which struct this belongs to) established in docs/re/environment.md "Bloom".
RECORD_ROLE = {
    0: "colour (.1)", 1: "box.x (.2)", 2: "box.y (.3)", 3: "box.z (.4)", 4: "edge-fade (.5)",
    5: "threshold-source, 1st '21' (always 3)", 6: "high-pass-amount-source (.16)",
    7: "STRENGTH-source (.6)", 8: "ramp-selector-source, 2nd '21'", 9: "blur-spread-source (.8)",
    10: "blur-asymmetry-source (.13)", 11: "mask ptr (.9)", 12: "const (.10)", 13: "const (.11)",
    14: "const (.18)", 15: "const (.19)", 16: "const (.20)",
}
RENDERABLE_ROLE = {
    0: "box (.1)", 1: "edge-fade (.2)", 2: "colour (.3)", 3: "THRESHOLD (.19, always 3, HDR-only)",
    4: "high-pass amount (.13)", 5: "STRENGTH multiplier (.4)", 6: "COLOUR-RAMP selector (.18)",
    7: "blur spread (.6)", 8: "blur asymmetry (.12)", 9: "mask texture (.7)", 10: "const (.8)",
    11: "const (.9)", 12: "const (.15)", 13: "const (.16)", 14: "const (.17)",
}


def main():
    args = sys.argv[1:]
    install = next((a for a in args if os.path.isdir(a)), os.environ.get("SS2_DATA_DIR", ""))
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    only = None
    if "--worlds" in args:
        only = [s.strip().lower() for s in args[args.index("--worlds") + 1].split(",")]
    files = sorted(merged_files(install, ".wld").items(), key=lambda kv: len(kv[1]))
    n_worlds = n_record = n_renderable = 0
    first21 = set()
    for wname, d in files:
        base = wname.rsplit("/", 1)[-1].lower()
        if only and not any(s in base for s in only):
            continue
        r = parse(d)
        dec = TreeDec(d, r)
        objs, edob = dec.objects()
        try:
            extra, _ = dec.extra_objects(edob)
        except Exception:
            extra = {}
        allobjs = {**objs, **extra}
        records = {oid: o for oid, o in allobjs.items() if getattr(o, "type", None) == "CBloomProperties"}
        renderables = {oid: o for oid, o in allobjs.items() if getattr(o, "type", None) == "CBloomRenderable"}
        if not records and not renderables:
            continue
        n_worlds += 1
        print(f"\n=== {wname.rsplit('/', 1)[-1]} ===")
        for oid, o in records.items():
            n_record += 1
            print(f"  CBloomProperties obj{oid}:")
            own = [(k, v) for k, v in o if k.startswith("CBloomProperties.")]
            for i, (k, v) in enumerate(own):
                role = RECORD_ROLE.get(i, "?")
                print(f"    [{i}] {k} = {v!r}  -- {role}")
                if i == 5 and isinstance(v, (int, float)):
                    first21.add(round(float(v), 3))
        for oid, o in renderables.items():
            n_renderable += 1
            print(f"  CBloomRenderable obj{oid}:")
            own = [(k, v) for k, v in o if k.startswith("CBloomRenderable.")]
            for i, (k, v) in enumerate(own):
                role = RENDERABLE_ROLE.get(i, "?")
                print(f"    [{i}] {k} = {v!r}  -- {role}")
    print(f"\n{n_worlds} world(s), {n_record} CBloomProperties instance(s), {n_renderable} CBloomRenderable "
          f"instance(s). First-occurrence '21' (threshold-source) values seen: {sorted(first21)} "
          "(expect only {3.0} if T01-424's 'always 3' finding holds for this subset).")


if __name__ == "__main__":
    main()
