"""Decode a CStaticTexture (docs/re/tex.md) to RGBA and write PNG files (read-only helper for the RE scripts).

Formats (tex.md): 0/2/6 = 24-bit BGR, 1/3/4/5/11 = 32-bit BGRA, 7 = DXT1, 8 = DXT3, 9 = DXT5; only the top
mip level (or the first cube face) is decoded. Rows run top to bottom.
"""
import struct, zlib


def write_png(path, w, h, rgba):
    raw = b"".join(b"\x00" + bytes(rgba[y * w * 4:(y + 1) * w * 4]) for y in range(h))

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    with open(path, "wb") as f:
        f.write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 6)) + chunk(b"IEND", b""))


def _rgb565(v): return (((v >> 11) & 31) * 255 // 31, ((v >> 5) & 63) * 255 // 63, (v & 31) * 255 // 31)


def _bc1(block, alpha_ok):
    c0, c1 = struct.unpack_from("<HH", block, 0); idx = struct.unpack_from("<I", block, 4)[0]; p0, p1 = _rgb565(c0), _rgb565(c1)
    if c0 > c1 or not alpha_ok:
        pal = [p0 + (255,), p1 + (255,), tuple((2 * a + b) // 3 for a, b in zip(p0, p1)) + (255,), tuple((a + 2 * b) // 3 for a, b in zip(p0, p1)) + (255,)]
    else:
        pal = [p0 + (255,), p1 + (255,), tuple((a + b) // 2 for a, b in zip(p0, p1)) + (255,), (0, 0, 0, 0)]
    return [pal[(idx >> (2 * i)) & 3] for i in range(16)]


def _dxt(raw, w, h, kind):
    out = bytearray(w * h * 4); size = 8 if kind == 1 else 16; p = 0
    for by in range(max(1, (h + 3) // 4)):
        for bx in range(max(1, (w + 3) // 4)):
            blk = raw[p:p + size]; p += size
            px = _bc1(blk if kind == 1 else blk[8:], kind == 1)
            if kind == 3:
                a = [((blk[i // 2] >> (4 * (i % 2))) & 15) * 17 for i in range(16)]
            elif kind == 5:
                a0, a1 = blk[0], blk[1]; bits = int.from_bytes(blk[2:8], "little")
                tab = [a0, a1] + ([((6 - i) * a0 + i * a1) // 7 for i in range(1, 7)] if a0 > a1 else [((5 - i) * a0 + i * a1) // 5 for i in range(1, 5)] + [0, 255])
                a = [tab[(bits >> (3 * i)) & 7] for i in range(16)]
            if kind != 1: px = [(r, g, b, a[i]) for i, (r, g, b, _) in enumerate(px)]
            for i in range(16):
                x, y = bx * 4 + i % 4, by * 4 + i // 4
                if x < w and y < h: out[(y * w + x) * 4:(y * w + x) * 4 + 4] = bytes(px[i])
    return out


def decode_static_texture(tex):
    """`tex` is a decoded CStaticTexture object (ctse_values.Obj); returns (width, height, RGBA bytearray)."""
    w, h, fmt = tex.get("CBaseTexture.1"), tex.get("CBaseTexture.2"), tex.get("CBaseTexture.6").v
    raw = zlib.decompress(bytes(tex.get("CBaseTexture.18")))
    if fmt in (0, 2, 6):
        out = bytearray(w * h * 4)
        for i in range(w * h): out[i * 4:i * 4 + 4] = bytes((raw[i * 3 + 2], raw[i * 3 + 1], raw[i * 3], 255))
    elif fmt in (1, 3, 4, 5, 11):
        out = bytearray(w * h * 4)
        for i in range(w * h): out[i * 4:i * 4 + 4] = bytes((raw[i * 4 + 2], raw[i * 4 + 1], raw[i * 4], raw[i * 4 + 3]))
    elif fmt in (7, 8, 9):
        out = _dxt(raw, w, h, {7: 1, 8: 3, 9: 5}[fmt])
    else:
        raise ValueError(f"unknown texture format {fmt}")
    return w, h, out
