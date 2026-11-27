"""Measure and verify the legacy .tex payloads for T01-083, read-only.

Run with SS2_DATA_DIR set or pass the install directory as an argument.
"""

import argparse
from collections import Counter, defaultdict
import os
from pathlib import Path
from struct import unpack_from
import sys
from zipfile import ZipFile
import zlib

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ctse_leftovers_verify import old_files, to_v8
from ctse_values import _natural, merged_files, parse, TreeDec
from tex_verify import block_bytes, mip_chain
from texpng import _dxt


STORED_FORMATS = {0: "BGR", 1: "BGRA", 2: "DXT1", 3: "DXT3", 4: "DXT5"}
FORMATS = {1: "BGRA", 2: "BGR", 4: "BGRA", 5: "BGRA", 6: "BGR",
           7: "DXT1", 8: "DXT3", 9: "DXT5"}
BLOCK_WIDTHS = {"DXT1": 8, "DXT3": 16, "DXT5": 16}
LEGACY_NAMES = {"1": "tx_pixWidth", "2": "tx_pixHeight", "4": "tx_ulFlags",
                "5": "tx_ctFrames", "6": "tx_txqDefault", "8": "tx_acolFrames",
                "17": "tx_txfStored", "18": "tx_aubFrames"}


def frame_pixels(blob, layout_version, flags, size):
    """Unwrap the declared class layout and require exactly the predicted pixel size."""
    if layout_version == 6:
        if len(blob) < 4 or unpack_from("<I", blob)[0] != len(blob) - 4:
            raise ValueError("compressed byte length prefix differs from the stream length")
        storage, payload = "prefixed zlib", blob[4:]
    elif layout_version == 9 and flags & 0x20:
        storage, payload = "zlib", blob
    else:
        storage, payload = "raw", blob
    if storage != "raw":
        inflater = zlib.decompressobj()
        payload = inflater.decompress(payload, size + 1)
        if not inflater.eof or inflater.unused_data:
            raise ValueError("expected one complete, bounded zlib stream")
    if len(payload) != size:
        raise ValueError(f"expected {size} pixel bytes, got {len(payload)}")
    return storage, payload


def effective_versions(install):
    """Read only texture headers, retaining the last case-insensitive path winner."""
    versions = {}
    archives = sorted(install.glob("*.gro"), key=lambda archive: _natural(archive.name))
    for archive in archives:
        with ZipFile(archive) as bundle:
            for entry in bundle.infolist():
                if entry.is_dir() or not entry.filename.casefold().endswith(".tex"):
                    continue
                with bundle.open(entry) as stream:
                    header = stream.read(16)
                version = unpack_from("<I", header, 12)[0] if header.startswith(b"CTSEMETA") else None
                versions[entry.filename.casefold()] = version
    return Counter(v for v in versions.values() if v in (4, 5, 6, 7))


def level_bytes(layout, w, h):
    if layout in BLOCK_WIDTHS:
        return block_bytes(w, h, BLOCK_WIDTHS[layout])
    return w * h * (3 if layout == "BGR" else 4)


def texture_payload(version, source):
    data = to_v8(source, version)
    frame = parse(data)
    objects, end = TreeDec(data, frame).objects()
    if end != frame["edob"] or data[end:] != b"EDOB\0\0\0\0METAEND ":
        raise ValueError("legacy objects must end at zero-record EDOB")
    classes = {oid: obj.type for oid, obj in objects.items()}
    if len(classes) != 2 or classes.get(0) != "CResourceFile" or classes.get(1) not in (
        "CStaticTexture", "CColorTexture"
    ):
        raise ValueError(f"unexpected object classes {classes}")
    tex = objects[1]
    cls = next(t for t in frame["types"].values() if t.name == "CBaseTexture")
    fields = {key: tex.get("CBaseTexture." + (name if version == 4 else key))
              for key, name in LEGACY_NAMES.items()}
    w, h = fields["1"], fields["2"]
    flags, format_field = fields["4"].v, fields["6"]
    if not (0 < w <= 4096 and 0 < h <= 4096):
        raise ValueError("unsupported dimensions")
    if fields["5"] != (None if cls.kind == 9 else 1):
        raise ValueError("frame count disagrees with the class layout")
    if format_field.name == "TexQuality" and cls.kind == 5:
        stored = fields["17"]
        if stored.name != "TexStoredFormat" or fields["8"] != []:
            raise ValueError("expected an explicit stored format and an empty palette")
        selector = (stored.name, stored.v)
        layout = STORED_FORMATS[stored.v]
    elif format_field.name == "TexFormat" and cls.kind in (6, 7, 9):
        if fields["17"] is not None or fields["8"] is not None:
            raise ValueError("unexpected palette or stored-format field")
        selector = (format_field.name, format_field.v)
        layout = FORMATS[format_field.v]
    else:
        raise ValueError(f"unrecognized CBaseTexture layout {cls.kind} with {format_field.name}")
    mipmapped = layout in BLOCK_WIDTHS or (cls.kind == 7 and layout == "BGRA")
    dimensions = mip_chain(w, h) if mipmapped else [(w, h)]
    faces = 6 if flags & 2 else 1
    size = faces * sum(level_bytes(layout, a, b) for a, b in dimensions)
    storage, pixels = frame_pixels(bytes(fields["18"]), cls.kind, flags, size)
    return frame, tex, selector, format_field.v, cls.kind, w, h, layout, dimensions, faces, storage, pixels


def mip_delta(first, second, w, h):
    """Mean absolute channel distance from averaging each corresponding 2×2 pixel block."""
    total = 0
    for y in range(h // 2):
        for x in range(w // 2):
            for channel in range(4):
                top = (2 * y * w + 2 * x) * 4 + channel
                index = (y * (w // 2) + x) * 4 + channel
                average = (first[top] + first[top + 4]
                           + first[top + w * 4] + first[top + w * 4 + 4]) / 4
                total += abs(average - second[index])
    return total / (w * h)


def compare_v8(source, layout, faces, w, h, pixels):
    """Compare same-layout copies of a texture across container versions."""
    texture = TreeDec(source, parse(source)).objects()[0][1]
    ref_width = texture.get("CBaseTexture.1")
    ref_height = texture.get("CBaseTexture.2")
    ref_format = texture.get("CBaseTexture.6").v
    ref_faces = 6 if texture.get("CBaseTexture.4").v & 2 else 1
    # The version-8 mapping also has format codes not seen in these older files.
    ref_layout = {0: "BGR", 3: "BGRA", 11: "BGRA"}.get(ref_format, FORMATS.get(ref_format))
    if (ref_width, ref_height, ref_layout, ref_faces) != (w, h, layout, faces):
        return None
    reference = zlib.decompress(bytes(texture.get("CBaseTexture.18")))
    if reference == pixels:
        return "identical"
    if layout == "BGRA" and reference == pixels[:w * h * 4]:
        return "identical base level"
    return "different"


def scan(install):
    storage_stats = Counter()
    formats = Counter()
    alpha = defaultdict(Counter)
    mip_samples = defaultdict(list)
    grayscale = {}
    failures = 0
    v8 = merged_files(install, ".tex")
    overlaps = Counter()
    for name, version, source in old_files(install):
        if version not in (4, 5, 6, 7) or not name.casefold().endswith(".tex"):
            continue
        try:
            (frame, tex, selector, quality_or_format, layout_version, w, h,
             layout, dimensions, faces, storage, pixels) = texture_payload(version, source)
            storage_stats[(version, layout_version, storage)] += 1
            formats[(*selector, quality_or_format, layout, len(dimensions) > 1, faces)] += 1
            if name.lower() in v8:
                comparison = compare_v8(v8[name.lower()], layout, faces, w, h, pixels)
                if comparison is not None:
                    overlaps[(layout, faces, comparison)] += 1
            if layout in ("DXT3", "DXT5"):
                for offset in range(0, level_bytes(layout, w, h), 16):
                    half = pixels[offset:offset + 8]
                    alpha[selector]["blocks"] += 1
                    alpha[selector]["all ff"] += half == b"\xff" * 8
                    alpha[selector]["ff ff followed by zeros"] += half == b"\xff\xff" + b"\0" * 6
            if layout == "BGRA" and len(dimensions) > 1:
                start = level_bytes(layout, w, h)
                mip_samples[layout].append(mip_delta(pixels[:start], pixels[start:], w, h))
            if layout == "DXT1" and "thumbnail" in name.casefold() and len(mip_samples[layout]) < 3:
                start = level_bytes(layout, w, h)
                low_w, low_h = dimensions[1]
                low_end = start + level_bytes(layout, low_w, low_h)
                upper = _dxt(pixels[:start], w, h, 1)
                lower = _dxt(pixels[start:low_end], low_w, low_h, 1)
                mip_samples[layout].append(mip_delta(upper, lower, w, h))
            leaf = name.casefold().rsplit("/", 1)[-1]
            if leaf in ("noise_01.tex", "noise_02.tex") and layout == "BGR":
                if pixels[0::3] != pixels[1::3] or pixels[0::3] != pixels[2::3]:
                    raise ValueError("expected equal channels for the legacy noise texture")
                grayscale[leaf] = pixels[0::3]
            if tex.type == "CColorTexture":
                ref = tex.get("CColorTexture.1")
                references = {oid: file_id for oid, file_id, _, _ in frame["exob"]}
                path = frame["rfil"][references[ref.oid]][2]
                print(f"CColorTexture: {w}x{h} {storage} {layout}, source {path}")
                print("CColorTexture auxiliary fields:", [(k, v) for k, v in tex if
                                                           k.startswith("CColorTexture.")])
        except (ValueError, KeyError, TypeError, StopIteration, zlib.error) as error:
            print(f"{name}: {error}")
            failures += 1

    print("verified (container version, class layout version, storage):")
    for key, count in sorted(storage_stats.items()):
        print(" ", key, count)
    print("formats (enum, value, quality/format, layout, mips, faces):")
    for key, count in sorted(formats.items()):
        print(" ", key, count)
    for key, values in sorted(alpha.items()):
        print("alpha signatures", key, dict(values))
    for layout, deltas in sorted(mip_samples.items()):
        for delta in deltas:
            print(f"{layout} level-one mean absolute error from a 2x2 average: {delta:.3f}")
    print("same-layout version-8 overlaps (layout, faces, comparison):", dict(sorted(overlaps.items())))
    if len(grayscale) == 2:
        mapping = Counter(zip(grayscale["noise_01.tex"], grayscale["noise_02.tex"], strict=True))
        print("source and baked gray pairs:", sorted(mapping.items()))
    effective = effective_versions(install)
    print("effective legacy paths:", effective.total(), dict(sorted(effective.items())))
    print("verified legacy archive entries:", storage_stats.total(), "failures:", failures)
    return int(failures != 0 or storage_stats.total() != 155 or effective.total() != 125)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("install", nargs="?", type=Path, default=os.environ.get("SS2_DATA_DIR"))
    args = parser.parse_args()
    if args.install is None or not args.install.is_dir():
        parser.error("pass a directory containing the retail .gro archives or set SS2_DATA_DIR")
    return scan(args.install)


if __name__ == "__main__":
    raise SystemExit(main())
