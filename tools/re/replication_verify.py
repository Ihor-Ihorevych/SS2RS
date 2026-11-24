"""Prints the data-side numbers quoted in docs/re/replication.md (read-only).

The replication rules themselves come from the binaries (no script can reproduce that). What the install's data can
check is (1) the framing of the three shipped demo recordings, which store the stream of network messages a client
receives (decoded here to the last byte of each file, so the message layouts in the spec are checked three times),
(2) the make-up of the opening snapshot, the per-step traffic and the share of empty updates, and (3) that no shipped
configuration script sets a `net_`, `ser_` or `cli_` variable (so the defaults in the spec are what an unmodified
install runs with).

Usage: python -I tools/re/replication_verify.py [DATA_DIR]      (DATA_DIR defaults to $SS2_DATA_DIR; runs in a few seconds)
"""
import collections
import os
import re
import statistics
import struct
import sys
import zipfile

NAMES = {1: "REQ_CONNECT", 2: "REQ_CHALLENGE", 3: "REP_CHALLENGE", 4: "REP_CONNECT", 5: "DISCONNECT",
         6: "SESSIONPARAMS", 7: "REMAPLIST", 8: "MODULEINDEX", 9: "STRINGINDEX", 10: "CREATE_ENTITY",
         11: "DELETE_ENTITY", 12: "UPDATE", 13: "INITIAL_UPDATE", 14: "RELIABLE_RPC", 15: "UNRELIABLE_RPC",
         16: "KEEPALIVE", 17: "RELIABLE_KEEPALIVE", 18: "PRE_WORLD_CHANGE", 19: "POST_WORLD_CHANGE"}
UPDATE, INITIAL = 12, 13


def string_at(d, p):
    n = struct.unpack_from("<H", d, p)[0]
    return d[p + 2:p + 2 + n], p + 2 + n


def parse_demo(path):
    """Decodes a whole demo. Returns (slot, blocks, updates). Raises on any byte it cannot account for."""
    d = open(path, "rb").read()
    if d[:8] != b"DMSTAVTR":
        raise ValueError("not a demo")
    slot = struct.unpack_from("<I", d, 8)[0]
    p = 12
    blocks = []          # one per TIME chunk: dict(t, bytes, count)
    updates = []         # (block index, entity handle, payload bytes, message type)
    cur = None
    while True:
        tag = d[p:p + 4]
        if tag == b"TIME":
            t = struct.unpack_from("<q", d, p + 4)[0]
            p += 12
            cur = {"t": t, "bytes": 0, "count": collections.Counter()}
            blocks.append(cur)
            continue
        if tag == b"MSGS":
            p += 4
            continue
        if tag == b"DMEN":
            p += 4
            break
        ty = d[p]
        start = p
        p += 1
        if ty == 4:                                   # connect response: 3 strings, accepted, client index, time
            for _ in range(3):
                _s, p = string_at(d, p)
            p += 2 + 8
        elif ty == 7:                                 # remap list: u16 count, then 8 bytes per entry
            p += 2 + 8 * struct.unpack_from("<H", d, p)[0]
        elif ty == 8:                                 # module index: string, u8
            _s, p = string_at(d, p)
            p += 1
        elif ty == 9:                                 # string index: string, u32
            _s, p = string_at(d, p)
            p += 4
        elif ty == 6:                                 # session parameters: u32, 12 bytes, 4 x u8
            p += 4 + 12 + 4
        elif ty == 10:                                # create entity: u32, u32, u8, u32, u16 size, data
            p += 4 + 4 + 1 + 4 + 2 + struct.unpack_from("<H", d, p + 13)[0]
        elif ty == 11:                                # delete entity: u32
            p += 4
        elif ty in (UPDATE, INITIAL):                 # u32 handle, u8 size, data
            h = struct.unpack_from("<I", d, p)[0]
            n = d[p + 4]
            updates.append((len(blocks), h, n, ty, d[p + 5:p + 5 + n]))
            p += 5 + n
        elif ty in (14, 15):                          # rpc: u8, u32, u32, u16 size, data
            p += 1 + 4 + 4 + 2 + struct.unpack_from("<H", d, p + 9)[0]
        elif ty in (16, 17):                          # keep-alives carry nothing
            pass
        elif ty == 18:                                # pre world change: string
            _s, p = string_at(d, p)
        elif ty == 19:                                # post world change: u8
            p += 1
        elif ty == 5:                                 # disconnect: string
            _s, p = string_at(d, p)
        else:
            raise ValueError(f"unknown message type {ty} at byte {start}")
        cur["bytes"] += p - start
        cur["count"][ty] += 1
    if p != len(d):
        raise ValueError(f"{len(d) - p} trailing bytes")
    return slot, blocks, updates, len(d)


def report_demo(path):
    slot, blocks, updates, size = parse_demo(path)
    print(f"{os.path.basename(path)}: {size} bytes, u32 after the magic = {slot}, {len(blocks)} TIME chunks")
    totals = collections.Counter()
    for b in blocks:
        totals.update(b["count"])
    print("  messages: " + ", ".join(f"{NAMES[k]} {v}" for k, v in sorted(totals.items())))
    t = [b["t"] for b in blocks]
    dts = [(t[i + 1] - t[i]) / 2 ** 32 for i in range(2, len(t) - 1)]
    dur = (t[-1] - t[2]) / 2 ** 32
    print(f"  duration {dur:.2f} s; block interval s: min {min(dts):.4f} median {statistics.median(dts):.4f} "
          f"mean {statistics.mean(dts):.4f} max {max(dts):.4f}")
    steady = blocks[12:]
    print(f"  steady blocks (after the 12th): {len(steady)}; bytes per block median "
          f"{statistics.median(b['bytes'] for b in steady):.0f} (max {max(b['bytes'] for b in steady)}); "
          f"updates per block mean {statistics.mean(b['count'][UPDATE] for b in steady):.1f}; "
          f"bytes per second {sum(b['bytes'] for b in steady) / ((t[-1] - t[12]) / 2 ** 32):.0f}")
    first = {NAMES[k]: v for k, v in sorted(blocks[1]["count"].items())}
    print(f"  first block after the two start stamps: {blocks[1]['bytes']} bytes {first}")
    ups = [u for u in updates if u[3] == UPDATE]
    empty = sum(1 for u in ups if u[2] == 0)
    nz = sorted(u[2] for u in ups if u[2] > 0)
    print(f"  UPDATE messages {len(ups)}, empty (no member changed) {empty} = {100 * empty / len(ups):.1f} %; "
          f"non-empty payload median {statistics.median(nz)}, 90th percentile {nz[int(0.9 * len(nz))]}, max {nz[-1]}")
    last = {}
    repeats = total = 0
    runs = collections.defaultdict(collections.Counter)
    for u in ups:
        if u[2] == 0:
            continue
        total += 1
        repeats += last.get(u[1]) == u[4]
        last[u[1]] = u[4]
        runs[u[1]][u[4]] += 1
    top = max(runs.values(), key=lambda c: sum(c.values()))
    print(f"  non-empty updates identical to the previous non-empty update of the same entity: {repeats} of {total} "
          f"= {100 * repeats / total:.1f} %; the busiest entity sent {sum(top.values())} non-empty updates, "
          f"{top.most_common(1)[0][1]} of them with one payload ({len(top)} distinct)")
    per = collections.Counter(u[1] for u in ups if u[0] >= 12)
    n_steady = len(steady)
    print(f"  distinct entities updated {len(set(u[1] for u in ups))}; in at least 90 % of the steady blocks: "
          f"{sum(1 for v in per.values() if v >= 0.9 * n_steady)}; median share of blocks an entity is updated in: "
          f"{100 * statistics.median(v / n_steady for v in per.values()):.1f} %")


def scan_configs(install):
    pat = re.compile(r"\b(?:net_\w+|ser_\w+|cli_\w+)\b")
    n_files = 0
    hits = []
    for f in sorted(os.listdir(install)):
        if f.lower().endswith(".gro"):
            with zipfile.ZipFile(os.path.join(install, f)) as z:
                for i in z.infolist():
                    if not i.is_dir() and i.filename.lower().endswith((".cfg", ".ini")):
                        n_files += 1
                        for no, line in enumerate(z.read(i).decode("latin1").splitlines(), 1):
                            if pat.search(line):
                                hits.append((f, i.filename, no))
    for root, _dirs, files in os.walk(os.path.join(install, "Content")):
        for name in files:
            if name.lower().endswith((".cfg", ".ini")):
                n_files += 1
                with open(os.path.join(root, name), "rb") as fh:
                    for no, line in enumerate(fh.read().decode("latin1").splitlines(), 1):
                        if pat.search(line):
                            hits.append(("loose", os.path.join(root, name), no))
    print(f"configuration scripts (.cfg/.ini) in the archives and loose: {n_files}; lines naming a net_, ser_ or cli_ variable: {len(hits)}")


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install):
        sys.exit("pass the install folder or set SS2_DATA_DIR")
    scan_configs(install)
    demos = sorted(os.path.join(r, n) for r, _d, fs in os.walk(os.path.join(install, "Content")) for n in fs
                   if n.lower().endswith(".dem"))
    print(f"demo recordings: {len(demos)}")
    for path in demos:
        report_demo(path)


if __name__ == "__main__":
    main()
