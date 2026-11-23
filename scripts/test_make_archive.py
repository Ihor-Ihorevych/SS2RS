"""Tests for scripts/make_archive.py (run: python -m unittest discover -s scripts)."""

import gzip
import hashlib
import os
import subprocess
import sys
import tarfile
import tempfile
import time
import unittest
import zipfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import make_archive  # noqa: E402

EPOCH = 1_700_000_000


def make_tree(root: Path, files: dict[str, bytes]) -> Path:
    top = root / "ss2rs-0.0.0-x"
    for name, data in files.items():
        path = top / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
    return top


FILES = {"b.txt": b"bee", "a.txt": b"ay", "sub/c.txt": b"cee", "ss2-app": b"\x7fELF"}


class DeterministicArchiveTests(unittest.TestCase):
    def build(self, tmp: Path, ext: str, files=None, epoch=EPOCH, touch=None) -> bytes:
        top = make_tree(tmp, files or FILES)
        if touch is not None:
            for p in top.rglob("*"):
                os.utime(p, (touch, touch))
        out = tmp / ("out" + ext)
        make_archive.make_archive(top, out, epoch)
        return out.read_bytes()

    def test_the_same_tree_gives_identical_bytes_whatever_the_file_times_and_the_directory(self):
        for ext in (".tar.gz", ".zip"):
            with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
                first = self.build(Path(a), ext, touch=1_000_000_000)
                second = self.build(Path(b), ext, touch=int(time.time()))
                self.assertEqual(hashlib.sha256(first).digest(), hashlib.sha256(second).digest(), ext)

    def test_the_order_of_the_file_system_listing_does_not_matter(self):
        from unittest import mock

        real_rglob = Path.rglob
        for ext in (".tar.gz", ".zip"):
            with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b:
                normal = self.build(Path(a), ext)
                with mock.patch.object(Path, "rglob", lambda self, pattern: reversed(list(real_rglob(self, pattern)))):
                    reversed_listing = self.build(Path(b), ext)
                self.assertEqual(normal, reversed_listing, ext)

    def test_a_different_epoch_or_content_changes_the_bytes(self):
        with tempfile.TemporaryDirectory() as a, tempfile.TemporaryDirectory() as b, tempfile.TemporaryDirectory() as c:
            base = self.build(Path(a), ".tar.gz")
            self.assertNotEqual(base, self.build(Path(b), ".tar.gz", epoch=EPOCH + 1))
            changed = dict(FILES, **{"a.txt": b"different"})
            self.assertNotEqual(base, self.build(Path(c), ".tar.gz", files=changed))

    def test_tar_entries_are_sorted_have_no_owner_and_the_given_time(self):
        with tempfile.TemporaryDirectory() as a:
            data = self.build(Path(a), ".tar.gz")
        out = Path(tempfile.mkdtemp()) / "x.tar.gz"
        out.write_bytes(data)
        with tarfile.open(out) as tar:
            members = tar.getmembers()
        names = [m.name for m in members]
        self.assertEqual(names, sorted(names), "sorted")
        self.assertEqual(names[0], "ss2rs-0.0.0-x", "the top folder comes first")
        for m in members:
            self.assertEqual((m.uid, m.gid, m.uname, m.gname), (0, 0, "", ""), m.name)
            self.assertEqual(m.mtime, EPOCH, m.name)
            self.assertTrue(m.name.startswith("ss2rs-0.0.0-x"), "only the one folder")
        modes = {m.name.split("/")[-1]: m.mode for m in members if m.isfile()}
        self.assertEqual(modes["a.txt"], 0o644)
        self.assertEqual(modes["ss2-app"], 0o755, "binaries are executable")

    def test_the_gzip_header_has_no_time_and_no_file_name(self):
        with tempfile.TemporaryDirectory() as a:
            data = self.build(Path(a), ".tar.gz")
        self.assertEqual(data[:2], b"\x1f\x8b")
        self.assertEqual(data[4:8], b"\x00\x00\x00\x00", "gzip mtime is zero")
        self.assertEqual(data[3] & 0x08, 0, "no FNAME flag")
        gzip.decompress(data)  # it is a valid gzip stream

    def test_zip_entries_are_sorted_with_the_given_time(self):
        with tempfile.TemporaryDirectory() as a:
            data = self.build(Path(a), ".zip")
        out = Path(tempfile.mkdtemp()) / "x.zip"
        out.write_bytes(data)
        with zipfile.ZipFile(out) as z:
            infos = z.infolist()
            self.assertIsNone(z.testzip())
        names = [i.filename for i in infos]
        self.assertEqual(names, sorted(names))
        for i in infos:
            self.assertEqual(i.date_time, (2023, 11, 14, 22, 13, 20), i.filename)
            self.assertEqual(i.create_system, 3, "made on Unix, whatever the machine")
        self.assertTrue(all(n.startswith("ss2rs-0.0.0-x/") for n in names))

    def test_an_epoch_before_1980_is_clamped_for_zip_not_an_error(self):
        with tempfile.TemporaryDirectory() as a:
            top = make_tree(Path(a), FILES)
            out = Path(a) / "o.zip"
            make_archive.make_archive(top, out, 0)
            with zipfile.ZipFile(out) as z:
                self.assertEqual(z.infolist()[0].date_time, (1980, 1, 1, 0, 0, 0))

    def test_unknown_extension_and_missing_folder_are_errors(self):
        with tempfile.TemporaryDirectory() as a:
            top = make_tree(Path(a), FILES)
            with self.assertRaises(SystemExit):
                make_archive.make_archive(top, Path(a) / "o.rar", EPOCH)
            with self.assertRaises(SystemExit):
                make_archive.make_archive(Path(a) / "nope", Path(a) / "o.zip", EPOCH)

    def test_symlinks_are_refused(self):
        with tempfile.TemporaryDirectory() as a:
            top = make_tree(Path(a), FILES)
            try:
                os.symlink(top / "a.txt", top / "link")
            except (OSError, NotImplementedError):
                self.skipTest("symlinks are not available here")
            with self.assertRaises(SystemExit):
                make_archive.make_archive(top, Path(a) / "o.zip", EPOCH)

    def test_the_command_line_reads_the_epoch_from_source_date_epoch(self):
        with tempfile.TemporaryDirectory() as a:
            top = make_tree(Path(a), FILES)
            out = Path(a) / "o.tar.gz"
            env = dict(os.environ, SOURCE_DATE_EPOCH=str(EPOCH))
            subprocess.run([sys.executable, str(HERE / "make_archive.py"), str(top), str(out)], check=True, env=env)
            with tarfile.open(out) as tar:
                self.assertEqual(tar.getmembers()[0].mtime, EPOCH)
            bad = subprocess.run([sys.executable, str(HERE / "make_archive.py"), str(top), str(out)], env={k: v for k, v in env.items() if k != "SOURCE_DATE_EPOCH"}, capture_output=True)
            self.assertNotEqual(bad.returncode, 0, "no epoch: refuse rather than use the clock")


if __name__ == "__main__":
    unittest.main()
