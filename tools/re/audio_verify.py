"""Re-runs the audio asset measurements behind docs/re/audio.md on an install (read-only).

Reads the headers (first bytes) of every `.wav` and `.ogg` in the archives (later archives override earlier ones):
format tag, channels, sample rate, bit depth, data size and duration for WAV; channels and rate for Ogg Vorbis.
Prints counts per folder and per format, never audio data.

Usage: python -I tools/re/audio_verify.py [DATA_DIR]
"""
import collections, os, struct, sys, zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from ctse_values import _natural  # noqa: E402


def wav_info(d):
    if d[:4] != b"RIFF" or d[8:12] != b"WAVE": return None
    p = 12; fmt = None; data = None
    while p + 8 <= len(d):
        tag = d[p:p + 4]; ln = struct.unpack_from("<I", d, p + 4)[0]
        if tag == b"fmt " and ln >= 16: fmt = struct.unpack_from("<HHIIHH", d, p + 8)
        if tag == b"data": data = ln; break
        p += 8 + ln + (ln & 1)
    return (fmt, data) if fmt and data is not None else None


def ogg_info(d):
    i = d.find(b"\x01vorbis")
    if i < 0 or i + 16 > len(d): return None
    ver, ch, rate = struct.unpack_from("<IBI", d, i + 7)
    return ver, ch, rate


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    install = args[0] if args else os.environ.get("SS2_DATA_DIR", "")
    if not install or not os.path.isdir(install): sys.exit("pass the install folder or set SS2_DATA_DIR")
    entries = {}
    for f in sorted((f for f in os.listdir(install) if f.lower().endswith(".gro")), key=_natural):
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for i in z.infolist():
                if not i.is_dir() and i.filename.lower().endswith((".wav", ".ogg")): entries[i.filename.lower()] = (f, i.filename, i.file_size)
    st = collections.Counter(); dist = collections.defaultdict(collections.Counter); secs = collections.defaultdict(float)
    by_arch = collections.defaultdict(list)
    for k, (f, name, size) in entries.items(): by_arch[f].append((k, name, size))
    for f, items in by_arch.items():
        with zipfile.ZipFile(os.path.join(install, f)) as z:
            for k, name, size in items:
                folder = "/".join(k.split("/")[:4]); ext = k[-4:]
                with z.open(name) as fh: head = fh.read(4096 if ext == ".ogg" else 1 << 16)
                st[f"{ext} files"] += 1; dist[f"{ext} by folder"][folder] += 1
                if ext == ".wav":
                    info = wav_info(head)
                    if not info: st["wav not parsed"] += 1; continue
                    (tag, ch, rate, byterate, align, bits), data = info
                    dist["wav format tag"][tag] += 1; dist["wav channels"][ch] += 1; dist["wav sample rate"][rate] += 1; dist["wav bits"][bits] += 1
                    secs[folder] += data / byterate if byterate else 0; st["wav seconds total"] += data / byterate if byterate else 0
                else:
                    info = ogg_info(head)
                    if not info: st["ogg not parsed"] += 1; continue
                    dist["ogg channels"][info[1]] += 1; dist["ogg sample rate"][info[2]] += 1; dist["ogg size bucket (KB)"][(size // 100000) * 100] += 1
    for k, v in sorted(st.items()): print(f"{v:10.0f}  {k}")
    for k, v in sorted(dist.items()): print(k, dict(v.most_common(10)))
    print("wav seconds by folder:", {k: round(v) for k, v in sorted(secs.items(), key=lambda kv: -kv[1])[:8]})


if __name__ == "__main__":
    main()
