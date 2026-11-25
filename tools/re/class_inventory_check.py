"""Re-checks the class inventory of `Sam2Game.dll` (docs/re/class-inventory*.md, T01-113) against the pages of the twelve parts.

Reads ONLY the repository's documents (no game file, no binary); standard library only.

It checks
  1. the two tables hold 932 rows with 932 distinct class names and the sum of their function counts is 10,502;
  2. the per-part class counts (52, 101, 61, 95, 83, 32, 113, 82, 55, 95, 104, 59) and function counts, and the data column's totals
     (178 same name, 249 `S` twin, 11 other name, 494 binary only; 427 + 505 = 932 by the survey's rule);
  3. that every row's covering page belongs to the row's part and exists, and that the page names the class: by its exported name, by its
     `S` twin, or (rows marked `rule`) by the partner class of a family rule (the entity of a properties class, the stem of an inventory item);
  4. that the 59 classes of T01-113 are named on the three companion pages of that part and on no other page of it is missing;
  5. that every page of every part is listed in docs/SUMMARY.md (the book's table of contents).

Usage: python -I tools/re/class_inventory_check.py [REPO_DIR]      (REPO_DIR defaults to the repository containing this script; a few seconds)
"""
import collections, os, re, sys

PAGES = {
    "T01-102": ["players-session-classes"],
    "T01-103": ["puppet-classes", "puppet-entity-classes", "puppet-entity-methods", "puppet-legged-classes", "puppet-vehicle-classes"],
    "T01-104": ["brain-classes"],
    "T01-105": ["behavior-classes", "behavior-action-classes", "behavior-entity-classes", "behavior-mover-classes"],
    "T01-106": ["weapon-classes"],
    "T01-107": ["projectile-classes"],
    "T01-108": ["item-classes", "item-classes-ammo-weapons", "item-classes-vitals"],
    "T01-109": ["mechanism-classes", "mechanism-classes-movers", "mechanism-classes-spawner-tweaks", "mechanism-classes-spawners",
                "mechanism-classes-triggers", "mechanism-classes-world-info"],
    "T01-110": ["worldcontent-classes", "worldcontent-effects", "worldcontent-markers-debug", "worldcontent-models-props", "worldcontent-sound-media"],
    "T01-111": ["hud-classes", "hud-classes-params", "hud-classes-simple", "hud-classes-special"],
    "T01-112": ["menu-classes", "menu-classes-shell", "menu-classes-play", "menu-classes-options", "menu-classes-session", "menu-classes-online",
                "menu-classes-popups", "menu-gadget-members"],
    "T01-113": ["helper-classes", "helper-classes-iterators", "helper-classes-events", "helper-classes-params"],
}
COUNTS = {"T01-102": 52, "T01-103": 101, "T01-104": 61, "T01-105": 95, "T01-106": 83, "T01-107": 32, "T01-108": 113, "T01-109": 82,
          "T01-110": 55, "T01-111": 95, "T01-112": 104, "T01-113": 59}
FUNCS = {"T01-102": 740, "T01-103": 1464, "T01-104": 572, "T01-105": 955, "T01-106": 780, "T01-107": 332, "T01-108": 919, "T01-109": 1206,
         "T01-110": 854, "T01-111": 797, "T01-112": 1412, "T01-113": 471}
SUFFIXES = ["RenderingParams", "Properties", "Params", "InventoryItem"]
ROW = re.compile(r"^\| `([A-Za-z0-9_]+)` \| (T01-\d+) \| \[`([a-z0-9-]+)`\]\(([a-z0-9-]+)\.md\) \| (name|S twin|rule) \| (same|S twin `[A-Za-z0-9_]+`|other name `[A-Za-z0-9_]+`|-) \| (\d+) \| (`[A-Za-z0-9_]+`|-) \|")
WORD = re.compile(r"[A-Za-z0-9_]+")


def partner(name):
    for s, repl in (("InventoryItem", ""), ("RenderingParams", "Entity"), ("Properties", "Entity"), ("Params", "Entity")):
        if name.endswith(s): return name[:-len(s)] + repl
    return None


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    repo = sys.argv[1] if len(sys.argv) > 1 else os.path.abspath(os.path.join(here, "..", ".."))
    docs = os.path.join(repo, "docs", "re")
    words = {}
    for p, l in PAGES.items():
        for f in l:
            fp = os.path.join(docs, f + ".md")
            if not os.path.exists(fp): sys.exit(f"missing page {f}.md of {p}")
            words[f] = set(WORD.findall(open(fp, encoding="utf8").read()))
    rows = []
    for t in ("class-inventory-1.md", "class-inventory-2.md"):
        for line in open(os.path.join(docs, t), encoding="utf8"):
            m = ROW.match(line)
            if m: rows.append(m.groups())
    print(f"1. rows: {len(rows)}; distinct class names: {len({r[0] for r in rows})}; functions: {sum(int(r[6]) for r in rows):,}")
    ok = len(rows) == 932 and len({r[0] for r in rows}) == 932 and sum(int(r[6]) for r in rows) == 10502
    part_of = {r[0]: r[1] for r in rows}
    cnt = collections.Counter(r[1] for r in rows); fn = collections.Counter()
    for r in rows: fn[r[1]] += int(r[6])
    print("2. per part (classes, functions):", {p: (cnt[p], fn[p]) for p in COUNTS})
    ok &= all(cnt[p] == COUNTS[p] and fn[p] == FUNCS[p] for p in COUNTS)
    dc = collections.Counter("same" if r[5] == "same" else "S twin" if r[5].startswith("S twin") else "other" if r[5].startswith("other") else "none" for r in rows)
    print("   data column:", dict(dc), "(same 178, S twin 249, other 11, none 494 expected)")
    ok &= dc["same"] == 178 and dc["S twin"] == 249 and dc["other"] == 11 and dc["none"] == 494
    bad = []; nrule = 0; nname = 0
    for name, part, page, link, how, data, nf, table in rows:
        if page != link or page not in PAGES[part]: bad.append((name, "covering page not of its part", page)); continue
        names = {name, "S" + name[1:]} if name[0] == "C" else {name}
        if how == "rule":
            nrule += 1; pr = partner(name)
            if pr is None or not any(pr in words[f] or ("S" + pr[1:]) in words[f] for f in PAGES[part]): bad.append((name, "no partner on the part's pages", pr))
        else:
            nname += 1
            if not (names & words[page]): bad.append((name, "not named on the covering page", page))
            elif how == "name" and name not in words[page]: bad.append((name, "marked name but only the S twin is there", page))
    print(f"3. rows named on their covering page: {nname}; rows covered by a family rule: {nrule}; rows that fail: {len(bad)}")
    for b in bad[:20]: print("   FAIL", b)
    ok &= not bad and nrule == 146
    mine = [r for r in rows if r[1] == "T01-113"]
    miss = [r[0] for r in mine if not any(r[0] in words[f] for f in PAGES["T01-113"])]
    print(f"4. T01-113 classes named on its four pages: {len(mine) - len(miss)} of {len(mine)}")
    ok &= not miss
    summary = open(os.path.join(repo, "docs", "SUMMARY.md"), encoding="utf8").read()
    nolink = [f for l in PAGES.values() for f in l] + ["class-inventory", "class-inventory-1", "class-inventory-2"]
    nolink = [f for f in nolink if f"re/{f}.md" not in summary]
    print("5. pages missing from docs/SUMMARY.md:", nolink or "none")
    ok &= not nolink
    tabs = sum(1 for r in rows if r[7] != "-")
    print(f"   classes with a member table: {tabs} (199 expected)")
    ok &= tabs == 199
    print("RESULT:", "all checks pass" if ok else "FAILED")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
