"""Generates the archive/file-type inventory (docs/re/inventory.md): paths, sizes and counts only.

Usage: python -I inventory.py [DATA_DIR] [OUT.md]   (DATA_DIR defaults to $SS2_DATA_DIR; OUT.md defaults to docs/re/inventory.md of this
repository; read-only on the install, no game content is written)
"""
import os, sys
import zipfile, os, collections, struct, re
arch_dir = (sys.argv[1] if len(sys.argv) > 1 else os.environ.get("SS2_DATA_DIR", ""))
if not arch_dir or not os.path.isdir(arch_dir):
    sys.exit("pass the install folder (the one containing All_PC_01.gro) or set SS2_DATA_DIR")
def natkey(n):
    return (0 if not n.lower().startswith("patch") else 1, [int(t) if t.isdigit() else t.lower() for t in re.split(r"(\d+)", n)])
archives = sorted([f for f in os.listdir(arch_dir) if f.lower().endswith(".gro")], key=natkey)
rows = []; by_path = collections.defaultdict(list); ext_info = collections.defaultdict(lambda: [0,0,0,0])  # count, bytes, meta, nonmeta
versions = collections.Counter(); dirs = collections.Counter()
for f in archives:
    with zipfile.ZipFile(os.path.join(arch_dir, f)) as z:
        infos = [i for i in z.infolist() if not i.is_dir()]
        rows.append((f, len(infos), sum(i.compress_size for i in infos), sum(i.file_size for i in infos), os.path.getsize(os.path.join(arch_dir, f))))
        for i in infos:
            by_path[i.filename.lower().replace("\\","/")].append(f)
            ext = os.path.splitext(i.filename.lower())[1] or "(none)"
            head = z.open(i).read(64) if i.file_size else b""
            e = ext_info[ext]; e[0] += 1; e[1] += i.file_size
            if head[:8] == b"CTSEMETA":
                e[2] += 1
                L = struct.unpack_from("<I", head, 16)[0]
                if L < 48: pass
            else: e[3] += 1
            parts = i.filename.split("/")
            dirs["/".join(parts[:3]) if len(parts) > 3 else "/".join(parts[:-1])] += 1
total = sum(r[1] for r in rows); unique = len(by_path); overridden = sum(1 for v in by_path.values() if len(v) > 1)
out = []
w = out.append
w("# Inventory of a Serious Sam 2 install (retail, patched to 1.0)\n")
w("- **Roadmap task:** T01-003 · **Confidence:** high (mechanically generated) · **Author:** @Ihor-Ihorevych/agent")
w("- **Method:** every `.gro` archive was opened as a ZIP and its entry table (names, sizes) and the first 8 bytes of each file were read. Only **paths, sizes and counts** are recorded here — no file contents. The full path list is generated locally by `gro-tool list` (T11-001) and is intentionally not committed.\n")
w("## Archives and mount order\n")
w("`.gro` files are standard ZIP archives. Order below is the proposed mount order (base archives first, then `Patch_*` by natural numeric order; **later archives override earlier ones** for the same case-insensitive path — see T02-002).\n")
w("| # | Archive | Entries | Packed (MB) | Unpacked (MB) | On disk (MB) |\n|---|---|---:|---:|---:|---:|")
for n, (f, c, cs, us, ds) in enumerate(rows, 1):
    w(f"| {n} | `{f}` | {c:,} | {cs/1048576:,.1f} | {us/1048576:,.1f} | {ds/1048576:,.1f} |")
w(f"\n**Totals:** {total:,} file entries across {len(rows)} archives; **{unique:,} distinct paths** (case-insensitive); **{overridden:,} paths exist in more than one archive** (patch overrides).\n")
w("## File types\n")
w("`CTSEMETA` = file starts with the 8 bytes `CTSEMETA` (see `ctsemeta-container.md`).\n")
w("| Extension | Files | Size (MB) | CTSEMETA | other |\n|---|---:|---:|---:|---:|")
for ext, (c, b, m, o) in sorted(ext_info.items(), key=lambda kv: -kv[1][1])[:40]:
    w(f"| `{ext}` | {c:,} | {b/1048576:,.1f} | {m:,} | {o:,} |")
rest = sorted(ext_info.items(), key=lambda kv: -kv[1][1])[40:]
if rest: w(f"\n(+{len(rest)} rarer extensions, {sum(v[0] for k,v in rest)} files.)")
w("\n## Directory layout (depth 3, entries)\n")
w("| Folder | Entries |\n|---|---:|")
for d, c in dirs.most_common(25): w(f"| `{d}` | {c:,} |")
out_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "docs", "re", "inventory.md")
open(out_path, "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n")
print("written; archives", len(rows), "total", total, "unique", unique, "overridden", overridden)
