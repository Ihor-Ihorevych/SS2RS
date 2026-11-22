"""Re-runs the localization measurements behind docs/re/localization.md on an install (read-only).

Counts the `CTransString` values (a key and a default text) in the `.rsc` and `.ep` files, the key styles, how many have a
non-empty default text, and the files under `Locales/<language>/`. Prints numbers and a few key shapes, no long texts.

Usage: python -I tools/re/l10n_verify.py [DATA_DIR]
"""
import collections, os, re, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import Obj, TreeDec, _natural, merged_files, parse  # noqa: E402


def collect(o, out):
    """Every `CTransString` below `o`: by-value members and elements of lists, recursively."""
    for k, v in o:
        if isinstance(v, Obj):
            if v.type == "CTransString": out.append(v)
            else: collect(v, out)
        elif isinstance(v, list):
            for x in v:
                if isinstance(x, Obj):
                    if x.type == "CTransString": out.append(x)
                    else: collect(x, out)


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); keys = {}
    for ext in (".rsc", ".ep", ".ctl", ".mdl", ".mac", ".wld"):
        if ext == ".wld": continue          # worlds are large; their strings are macro text, see ctsemeta-objects.md
        for name, d in merged_files(install, ext).items():
            r = parse(d)
            try: objs, _ = TreeDec(d, r).objects()
            except Exception: continue
            found = []
            for o in objs.values():
                if isinstance(o, Obj): collect(o, found)
            for ts in found:
                vals = [v for _, v in ts]; key, text = vals[0], (vals[1] if len(vals) > 1 else "")
                st[f"{ext} CTransString values"] += 1
                keys.setdefault(key, text)
                dist[f"{ext} key style"]["hex hash" if re.fullmatch(r"[0-9A-F]{8}", str(key)) else ("dotted name" if "." in str(key) else "other")] += 1
                if text: st[f"{ext} with non-empty default text"] += 1
    st["distinct keys"] = len(keys); st["distinct keys with a non-empty default text"] = sum(1 for t in keys.values() if t)
    langs = collections.Counter(); kinds = collections.Counter()
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            seen = {}
            for i in z.infolist():
                n = i.filename.lower()
                if not i.is_dir() and "/locales/" in n: seen[n] = 1
            for n in seen:
                lang = n.split("/locales/")[1].split("/")[0]; langs[lang] += 1; kinds[(lang, os.path.splitext(n)[1])] += 1
    for k, v in sorted(st.items()): print(f"{v:6d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(5)))
    print("locale files by language (entries over all archives):", dict(langs)); print("by kind:", dict(kinds))


if __name__ == "__main__":
    main()
