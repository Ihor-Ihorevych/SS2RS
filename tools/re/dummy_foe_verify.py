"""Evidence for docs/re/puppet-dummy-foe.md (T01-150): what the install says about the dummy foe puppet.

Read-only on the install (CLEANROOM.md: this script reads DATA files only, never the game's binaries).

It prints
  1. the files of the install whose name contains `dummy` (the built-in parameter file, model and editor thumbnail the class names are
     expected under `Puppets/Objects`, `Models/Enemies/DummyFoe` and `SeriousEditor2/Thumbnails`: none is there) and the `.ep`
     files of the `Puppets/Objects` folder;
  2. the editor's entity palette (`SeriousEditor2/CustomTrees/entities.ctr`): the number of entries, whether `DummyFoe` is one, and the
     puppet-like entries around it;
  3. with `--grep-all` (about 70 seconds): every file of the install whose bytes contain the text `DummyFoe` (any case). The placed
     entities of the worlds are counted by `puppet_params_verify.py` (section 4: none).

Usage: python -I tools/re/dummy_foe_verify.py [DATA_DIR] [--grep-all]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import os, re, sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import merged_files  # noqa: E402


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    everything = merged_files(install, "")
    names = sorted(everything)
    print(f"1. {len(names)} files in the install")
    print("   names containing 'dummy':", [n for n in names if "dummy" in n.lower()])
    print("   Puppets/Objects:", [n for n in names if "/databases/puppets/objects/" in n.lower()])
    print("   Models/Enemies folders with 'foe' in the name:", sorted({n.split("/")[3] for n in names if n.lower().startswith("content/serioussam2/models/enemies/") and "foe" in n.lower().split("/")[3]}))
    print("   thumbnails:", sorted(n for n in names if "/thumbnails/" in n.lower())[:8], "...")
    d = everything.get("content/seriouseditor2/customtrees/entities.ctr")
    print("\n2. the editor's entity palette")
    if d is None:
        print("   entities.ctr not found")
    else:
        prefix = b"Bin/Sam2Game.module|"        # a length-prefixed string "<module>|<entry name>": a 4-byte length, then the text
        found = [d[m.start():m.start() - 4 + 4 + int.from_bytes(d[m.start() - 4:m.start()], "little")][len(prefix):].decode("latin-1")
                 for m in re.finditer(re.escape(prefix), d)]
        print(f"   entities.ctr: {len(found)} entries from the game module; DummyFoe is an entry: {'DummyFoe' in found}")
        print("   puppet-like entries:", [n.strip() for n in found if any(w in n.lower() for w in ("character", "puppet", "dummy", "test "))])
    if "--grep-all" in sys.argv:
        pat = re.compile(rb"dummyfoe", re.I)
        print("\n3. files whose bytes contain DummyFoe:", [k for k, v in everything.items() if pat.search(v)])


if __name__ == "__main__":
    main()
