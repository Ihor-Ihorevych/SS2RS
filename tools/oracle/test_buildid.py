import os
import struct
import tempfile
import unittest

from ss2_oracle.buildid import build_id, install_root_from_exe, pe_timestamp


def fake_pe(stamp):
    head = bytearray(64)
    head[:2] = b"MZ"
    struct.pack_into("<I", head, 0x3C, 64)
    return bytes(head) + b"PE\0\0" + struct.pack("<HHI", 0x14C, 3, stamp) + b"\0" * 32


class BuildIdTests(unittest.TestCase):
    def make_install(self, root, exe_stamp=1000):
        os.makedirs(os.path.join(root, "Bin"))
        for name in ("Sam2.exe", "Engine.dll", "Core.dll", "Sam2Game.dll"):
            with open(os.path.join(root, "Bin", name), "wb") as f:
                f.write(fake_pe(exe_stamp if name == "Sam2.exe" else 5))
        with open(os.path.join(root, "Patch_02_068.gro"), "wb") as f:
            f.write(b"x" * 10)
        with open(os.path.join(root, "All_PC_01.gro"), "wb") as f:
            f.write(b"y")

    def test_pe_timestamp(self):
        with tempfile.TemporaryDirectory() as d:
            path = os.path.join(d, "a.dll")
            with open(path, "wb") as f:
                f.write(fake_pe(0x5F000000))
            self.assertEqual(pe_timestamp(path), 0x5F000000)
            with open(path, "wb") as f:
                f.write(b"not a pe at all" * 10)
            self.assertIsNone(pe_timestamp(path))
            self.assertIsNone(pe_timestamp(os.path.join(d, "missing")))

    def test_id_is_stable_and_changes_with_a_file(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
            self.make_install(os.path.join(a, "i"))
            self.make_install(os.path.join(b, "i"))
            one, two = build_id(os.path.join(a, "i")), build_id(os.path.join(b, "i"))
            self.assertEqual(one["id"], two["id"])
            self.assertEqual(len(one["id"]), 16)
            self.assertEqual(one["files"]["Sam2.exe"]["pe_timestamp"], 1000)
            self.assertEqual(one["patch_archives"], [{"name": "Patch_02_068.gro", "size": 10}])
            with open(os.path.join(b, "i", "Bin", "Engine.dll"), "ab") as f:
                f.write(b"!")
            self.assertNotEqual(build_id(os.path.join(b, "i"))["id"], one["id"])

    def test_a_new_patch_archive_changes_the_id(self):
        with tempfile.TemporaryDirectory() as a:
            root = os.path.join(a, "i")
            self.make_install(root)
            before = build_id(root)["id"]
            with open(os.path.join(root, "Patch_02_100_800000.gro"), "wb") as f:
                f.write(b"z")
            self.assertNotEqual(build_id(root)["id"], before)

    def test_missing_files_are_reported_not_fatal(self):
        with tempfile.TemporaryDirectory() as a:
            info = build_id(a)
            self.assertIsNone(info["files"]["Sam2.exe"])
            self.assertEqual(info["patch_archives"], [])

    def test_install_root_from_exe(self):
        self.assertEqual(install_root_from_exe(os.path.join("X", "Game", "Bin", "Sam2.exe")), os.path.abspath(os.path.join("X", "Game")))
        self.assertEqual(install_root_from_exe(os.path.join("X", "Game", "Sam2.exe")), os.path.abspath(os.path.join("X", "Game")))


if __name__ == "__main__":
    unittest.main()
