"""Re-runs the measurement behind docs/re/projectile-classes.md section 6.4 on an install (read-only).

Walks every `.ep` file's `CShootingProcess.sp_csbBlasts` -> `CSingleBlast.sb_bfcBulletFireing`
chain and reports, for the blasts that actually fire bullets (`bf_ctBullets` above 0), their
`bf_btBulletType` (`EBulletType`) distribution and which file/attack uses `BT_PHYSICAL` (0): the
retail-data answer to "which enemy attack descriptions fire flying bullets" (`CBulletEntity`).

Usage: python -I tools/re/bulletfireing_type_verify.py [DATA_DIR]   (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import collections, os, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, Ref, TreeDec, merged_files, parse  # noqa: E402

NULL = 0xFFFFFFFF


def main():
    install = sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR")
    if not install:
        print("set SS2_DATA_DIR or pass the install directory")
        return 2

    rows = []
    for path, d in merged_files(install, ".ep").items():
        r = parse(d)
        dec = TreeDec(d, r)
        objs, _ = dec.objects()
        for o in objs.values():
            if not (isinstance(o, Obj) and o.type == "CShootingProcess"):
                continue
            name = o.get("CShootingProcess.sp_idName")
            for bref in o.get("CShootingProcess.sp_csbBlasts") or []:
                if not (isinstance(bref, Ref) and bref.oid != NULL):
                    continue
                blast = objs.get(bref.oid)
                if not isinstance(blast, Obj):
                    continue
                bf = blast.get("CSingleBlast.sb_bfcBulletFireing")
                if bf is None:
                    continue
                ctb = getattr(bf.get("CBulletFireing.bf_ctBullets"), "v", bf.get("CBulletFireing.bf_ctBullets"))
                bt = getattr(bf.get("CBulletFireing.bf_btBulletType"), "v", bf.get("CBulletFireing.bf_btBulletType"))
                if ctb and ctb > 0:
                    rows.append((path, name, ctb, bt))

    print("blasts with bullets (bf_ctBullets > 0):", len(rows))
    print("bullet type distribution among them:", dict(collections.Counter(r[3] for r in rows)))
    print()
    for path, name, ctb, bt in rows:
        if bt == 0:
            print(f"BT_PHYSICAL(0): {os.path.basename(path)} attack={name} bullets={ctb}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
