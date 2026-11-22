"""Re-runs the measurements behind docs/re/shaders.md on an install (read-only).

Reads `Sources/Shaders/GPU-Programs.asm` from the archives (the patched version wins; `Bin/GPU-Programs.asm`
on disk is the same file) and reports how the file is organised: program labels, doc comments, version
tags, instruction counts, registers and families. It prints numbers only, never program text.

Usage: python -I tools/re/shader_verify.py [DATA_DIR]
"""
import collections, os, re, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import _natural  # noqa: E402

NAME = "Sources/Shaders/GPU-Programs.asm"


def read_asm(install):
    data = None; versions = []
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if i.filename.lower() == NAME.lower(): data = z.read(i); versions.append((f, i.file_size))
    return data, versions


def split_programs(text):
    """[(label, doc, body_lines)]: a label is a line `Name:` outside comments; the doc is the first block comment after it."""
    progs = []; in_block = False; cur = None
    lines = text.replace("\r", "").split("\n")
    for l in lines:
        s = l.strip()
        m = re.fullmatch(r"([A-Za-z_]\w*):\s*(//.*)?", s)
        if not in_block and m:
            cur = [m.group(1), [], []]; progs.append(cur); continue
        if cur is None: continue
        if in_block:
            cur[1].append(l)
            if "*/" in l: in_block = False
            continue
        if s.startswith("/*"):
            cur[1].append(l); in_block = "*/" not in l
            continue
        cur[2].append(l)
    return progs


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    data, versions = read_asm(install)
    if data is None: sys.exit("GPU-Programs.asm not found in the archives")
    text = data.decode("latin1").replace(chr(13), ""); progs = split_programs(text)
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter)
    st["file bytes (patched version)"] = len(data); print("versions in archives:", versions)
    disk = os.path.join(install, "Bin", "GPU-Programs.asm")
    if os.path.exists(disk): st["Bin/GPU-Programs.asm equals the archive version"] = int(open(disk, "rb").read() == data)
    st["lines"] = text.count("\n") + 1; st["programs"] = len(progs)
    st["programs with a block comment (the IN/OUT register notes)"] = sum(1 for p in progs if p[1])
    st["every program ends with END"] = sum(1 for p in progs if any(x.strip() == "END" for x in p[2]))
    st["labels are unique"] = int(len({p[0] for p in progs}) == len(progs))
    insn = []
    for label, doc, body in progs:
        code = [re.sub(r"//.*", "", x).strip() for x in body]; code = [x for x in code if x and x != "END"]
        tag = next((x for x in code if re.fullmatch(r"(CT)?(vs|ps)\.\d\.\d", x)), None)
        dist["version tags"][tag] += 1
        ops = [x for x in code if x != tag and not x.startswith("def")]
        insn.append(len(ops)); dist["instruction count ranges"][(len(ops) // 10) * 10] += 1
        m = re.fullmatch(r"(\w+?)_(VP|PP)(\d*)(?:_(\w+))?", label)
        fam = m.group(1) if m else label
        kind = m.group(2) if m else "?"
        dist["digits after VP/PP (model hint)"][m.group(2) + m.group(3)] += 1
        dist[f"family {fam}"][kind + m.group(3)] += 1
        dist["families"][fam] += 1; dist[f"programs by kind"][kind] += 1
        dist["label shape matches Family_(VP|PP)[n][_variant]"][bool(m)] += 1
        used = {int(x) for x in re.findall(r"\bc(\d+)\b", " ".join(code))}
        dist["highest constant register c<n> used"][max(used) if used else -1] += 1
        dist["defs (inline constants) per program"][sum(1 for x in code if x.startswith("def"))] += 1
        dist["uses texture-load op"][any(re.match(r"tex", x) for x in code)] += 1
    st["longest program (instructions)"] = max(insn); st["shortest program (instructions)"] = min(insn)
    secs = re.findall(r"(?m)^//\s*-[ -]+(.+?)\s*-[ -]*$", text); st["section banner comments"] = len(secs)
    for k, v in sorted(st.items()): print(f"{v:8d}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(sorted(v.items(), key=lambda kv: -kv[1])[:20]))
    for s in secs: print("  section:", s)


if __name__ == "__main__":
    main()
