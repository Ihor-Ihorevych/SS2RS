"""Synthetic checks of the T01-083 legacy texture evidence helper."""

import struct
import unittest
import zlib

from tools.re.tex_legacy_verify import frame_pixels


class LegacyTextureTests(unittest.TestCase):
    def test_prefixed_stream_length_counts_compressed_bytes_only(self):
        pixels = bytes(range(12))
        stream = zlib.compress(pixels)
        blob = struct.pack("<I", len(stream)) + stream
        self.assertEqual(frame_pixels(blob, 6, 0, len(pixels)), ("prefixed zlib", pixels))
        with self.assertRaisesRegex(ValueError, "length prefix"):
            frame_pixels(struct.pack("<I", len(blob)) + stream, 6, 0, len(pixels))

    def test_zlib_requires_the_full_stream_and_exact_output(self):
        pixels = b"\0" * 12
        stream = zlib.compress(pixels)
        self.assertEqual(frame_pixels(stream, 9, 0x20, 12), ("zlib", pixels))
        for malformed in (stream[:-1], stream + b"extra", zlib.compress(pixels + b"\0")):
            with self.subTest(malformed=malformed), self.assertRaises(ValueError):
                frame_pixels(malformed, 9, 0x20, 12)

    def test_raw_frame_is_not_inflated_based_on_its_first_bytes(self):
        pixels = b"\x78\x9c\0\0\0\0"
        self.assertEqual(frame_pixels(pixels, 5, 0, 6), ("raw", pixels))


if __name__ == "__main__":
    unittest.main()
