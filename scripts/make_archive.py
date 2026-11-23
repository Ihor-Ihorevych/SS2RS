#!/usr/bin/env python3
"""Make a release archive whose bytes depend only on the files in it (roadmap T12-008).

    SOURCE_DATE_EPOCH=<unix seconds> python scripts/make_archive.py <folder> <archive.tar.gz|archive.zip>

The folder is archived under its own name. Entries are sorted, carry the given time and no owner,
modes are normalised (0755 for programs and folders, 0644 for the rest), the gzip header has no time
and no file name, and the zip says it was made on Unix. So two builds of the same files give the same
archive, whatever the file times, the user or the directory they were built in. The time is read from
SOURCE_DATE_EPOCH and never from the clock: without it the script refuses.

The compressed bytes also depend on the zlib that Python links, so compare archives made on the same
kind of machine (the release workflow does).
"""

import gzip
import io
import os
import stat
import sys
import tarfile
import time
import zipfile
from pathlib import Path

ZIP_MIN_EPOCH = 315_532_800  # 1980-01-01, the first date a zip can hold
PROGRAM_MAGIC = (b"\x7fELF", b"MZ", b"\xcf\xfa\xed\xfe", b"\xce\xfa\xed\xfe", b"\xca\xfe\xba\xbe")


def fail(message: str):
    sys.exit(f"make_archive: {message}")


def is_program(path: Path) -> bool:
    if path.suffix.lower() == ".exe" or path.stat().st_mode & 0o111:
        return True
    with path.open("rb") as handle:
        head = handle.read(4)
    return any(head.startswith(magic) for magic in PROGRAM_MAGIC)


def entries(folder: Path):
    """(archive name, path, is_dir) for the folder and everything under it, sorted by name."""
    top = folder.name
    found = [(top, folder, True)]
    for path in folder.rglob("*"):
        if path.is_symlink():
            fail(f"{path} is a symbolic link; an archive of files only is made")
        relative = path.relative_to(folder).as_posix()
        found.append((f"{top}/{relative}", path, path.is_dir()))
    return sorted(found, key=lambda e: e[0])


def write_tar_gz(items, out: Path, epoch: int):
    buffer = io.BytesIO()
    with tarfile.open(fileobj=buffer, mode="w", format=tarfile.GNU_FORMAT) as tar:
        for name, path, is_dir in items:
            info = tarfile.TarInfo(name)
            info.mtime = epoch
            info.uid = info.gid = 0
            info.uname = info.gname = ""
            if is_dir:
                info.type = tarfile.DIRTYPE
                info.mode = 0o755
                tar.addfile(info)
            else:
                data = path.read_bytes()
                info.size = len(data)
                info.mode = 0o755 if is_program(path) else 0o644
                tar.addfile(info, io.BytesIO(data))
    with out.open("wb") as handle:
        # filename="" leaves the name out of the header, mtime=0 the time.
        with gzip.GzipFile(filename="", mode="wb", fileobj=handle, compresslevel=9, mtime=0) as gz:
            gz.write(buffer.getvalue())


def write_zip(items, out: Path, epoch: int):
    date_time = time.gmtime(max(epoch, ZIP_MIN_EPOCH))[:6]
    with zipfile.ZipFile(out, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as z:
        for name, path, is_dir in items:
            if is_dir:
                info = zipfile.ZipInfo(name + "/", date_time)
                info.external_attr = (stat.S_IFDIR | 0o755) << 16 | 0x10
                info.compress_type = zipfile.ZIP_STORED
                data = b""
            else:
                info = zipfile.ZipInfo(name, date_time)
                info.external_attr = (stat.S_IFREG | (0o755 if is_program(path) else 0o644)) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                data = path.read_bytes()
            info.create_system = 3  # Unix, whatever the machine that made it
            z.writestr(info, data)


def make_archive(folder: Path, out: Path, epoch: int):
    if not folder.is_dir():
        fail(f"{folder} is not a folder")
    name = out.name.lower()
    if not (name.endswith(".tar.gz") or name.endswith(".zip")):
        fail(f"{out.name}: the archive must end in .tar.gz or .zip")
    items = entries(folder)
    if name.endswith(".zip"):
        write_zip(items, out, epoch)
    else:
        write_tar_gz(items, out, epoch)


def main(argv):
    if len(argv) != 3:
        fail("usage: make_archive.py <folder> <archive.tar.gz|archive.zip>  (with SOURCE_DATE_EPOCH set)")
    epoch_text = os.environ.get("SOURCE_DATE_EPOCH")
    if not epoch_text or not epoch_text.isdigit():
        fail("SOURCE_DATE_EPOCH (unix seconds) must be set; the clock is never used")
    make_archive(Path(argv[1]), Path(argv[2]), int(epoch_text))


if __name__ == "__main__":
    main(sys.argv)
