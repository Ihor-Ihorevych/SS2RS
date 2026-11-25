"""Prints the data-side numbers quoted in docs/re/main-loop.md (read-only).

The main loop itself is specified from the binaries (no script can reproduce that). What the install's *data*
can say is whether any shipped configuration script changes a timing variable, so that the defaults in the
spec are the values the game runs with. This script reads every `.cfg` (text) in the `.gro` archives (later
archives override earlier ones) and in the loose `Content/**/Config` folders, and reports

  * how many config scripts there are and which of them exist;
  * every line that sets one of the timing variables the spec names (sim_*, phy_*, net_bClientsMayPause,
    sam_iDedicatedFPS, gfx_iWaitVSyncs, gfx_iMaxFPS*, cli_i*BPS, gfx_iFinish);
  * every line that mentions an FPS display or a pause.

Usage: python -I tools/re/mainloop_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR)
"""
import os
import re
import sys
import zipfile

VARS = re.compile(r"\b(sim_\w+|phy_\w+|net_bClientsMayPause|sam_iDedicatedFPS|gfx_iWaitVSyncs|gfx_iMaxFPS\w*|"
                  r"cli_i\w*BPS|gfx_iFinish)\b")
OTHER = re.compile(r"FPS|pause|real.?time", re.I)


def natural(name):
    lower = name.lower()
    return (1 if lower.startswith("patch") else 0, [int(t) if t.isdigit() else t for t in re.split(r"(\d+)", lower)])


def collect(install):
    """{normalised path: (source, text)} of every .cfg; archives in patch order, then loose files."""
    out = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower().endswith(".cfg"):
                    out[i.filename.lower()] = (f, z.read(i).decode("latin1"))
    for root, _dirs, files in os.walk(os.path.join(install, "Content")):
        for n in files:
            if n.lower().endswith(".cfg"):
                p = os.path.join(root, n)
                rel = os.path.relpath(p, install).replace("\\", "/").lower()
                with open(p, "rb") as fh:
                    out[rel] = ("loose", fh.read().decode("latin1"))
    return out


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    cfgs = collect(install)
    print(f"config scripts (.cfg): {len(cfgs)}")
    for name in sorted(cfgs):
        print(f"  {name}  [{cfgs[name][0]}]  {len(cfgs[name][1].splitlines())} lines")
    setting = []
    other = []
    for name in sorted(cfgs):
        for no, line in enumerate(cfgs[name][1].splitlines(), 1):
            code = line.split("//", 1)[0]
            if VARS.search(code):
                setting.append((name, no, line.strip()))
            elif OTHER.search(line):
                other.append((name, no, line.strip()))
    print(f"lines that set a timing variable named in the spec: {len(setting)}")
    for name, no, line in setting:
        print(f"  {name}:{no}: {line}")
    sim = [s for s in setting if re.search(r"\b(sim_|phy_|net_bClientsMayPause|sam_iDedicatedFPS)", s[2])]
    print(f"  of which set sim_*, phy_*, net_bClientsMayPause or sam_iDedicatedFPS: {len(sim)}")
    print(f"other lines that mention FPS, pause or real time: {len(other)}")
    for name, no, line in other:
        print(f"  {name}:{no}: {line}")


if __name__ == "__main__":
    main()
