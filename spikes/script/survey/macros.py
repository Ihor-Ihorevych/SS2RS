"""Which macro actions do the `.mac` programs of the two shipped mods call, and how many are outside the base
vocabulary of docs/re/wld-macro-vocab.md? Reads the mod archives directly (SS2_DATA_DIR); nothing is extracted.

    SS2_DATA_DIR=<install> python -I macros.py
"""

import collections
import glob
import os
import re
import zipfile

here = os.path.dirname(os.path.abspath(__file__))
doc = open(os.path.join(here, "../../../docs/re/wld-macro-vocab.md"), encoding="utf-8").read()
sections = {}
cur = None
for line in doc.splitlines():
    m = re.match(r"## (\w[\w ]*?) \((\d+) distinct\)", line)
    if m:
        cur = m.group(1)
        sections[cur] = set()
        continue
    if cur and line.startswith("| `"):
        sections[cur].add(re.match(r"\| `([^`]+)`", line).group(1))
print({k: len(v) for k, v in sections.items()})

root = os.environ["SS2_DATA_DIR"]
MODS = {
    "InSamnity2": [root + "/Content/InSamnity2/InSamnity2.gro"] + sorted(glob.glob(root + "/Content/InSamnity2/z_*.gro")),
    "Sam2Renovation": [root + "/Content/Sam2Renovation/Sam2Renovation.gro"],
}
ACTION = re.compile(r"\bAction(?:Async)?\s+[\w.]+?\.(\w+)\s*\(")
EVENT = re.compile(r"\bevent\(\s*[\w.]+?\.(\w+)\s*\)")
SEND = re.compile(r"\bSend event\s+[\w.]+?\.(\w+)")
PRINTABLE = re.compile(rb"[\x09\x0a\x0d\x20-\x7e]{80,}")

for mod, gros in MODS.items():
    acts, evs = collections.Counter(), collections.Counter()
    files = lines = 0
    for g in gros:
        with zipfile.ZipFile(g) as z:
            for i in z.infolist():
                if not i.filename.lower().endswith(".mac"):
                    continue
                runs = PRINTABLE.findall(z.read(i))
                txt = "\n".join(r.decode() for r in runs if b"Macro" in r or b"Action" in r or b"On (" in r)
                files += 1
                lines += txt.count("\n")
                acts.update(ACTION.findall(txt))
                evs.update(EVENT.findall(txt))
                evs.update(SEND.findall(txt))
    unk = {a: c for a, c in acts.items() if a not in sections.get("action", set())}
    print("==", mod, "files", files, "text lines", lines)
    print(" action names used", len(acts), "uses", sum(acts.values()), "| names outside the base vocabulary:", len(unk), "uses:", sum(unk.values()), unk)
    print(" event names used", len(evs))
