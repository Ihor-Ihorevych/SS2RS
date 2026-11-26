"""Identify an install from its files (never from the process).

The identity is a hash of the executable and the four main modules plus the names and sizes of
the patch archives. The numbers go into the capture's sidecar so a result can be tied to the exact
build it was measured on (docs/re/conventions.md: behaviour differs across patch levels).
"""

import hashlib
import os
import struct
from typing import Dict, List, Optional

KEY_FILES = ["Sam2.exe", "Engine.dll", "Core.dll", "Sam2Game.dll"]


def pe_timestamp(path: str) -> Optional[int]:
    """The COFF header time-date stamp of a PE file, or None if it is not one."""
    try:
        with open(path, "rb") as handle:
            head = handle.read(64)
            if len(head) < 64 or head[:2] != b"MZ":
                return None
            (pe_offset,) = struct.unpack_from("<I", head, 0x3C)
            handle.seek(pe_offset)
            # "PE\0\0", machine (2), section count (2), then the time-date stamp (4)
            header = handle.read(12)
            if len(header) < 12 or header[:4] != b"PE\0\0":
                return None
            return struct.unpack_from("<I", header, 8)[0]
    except OSError:
        return None


def _sha256_file(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def install_root_from_exe(exe_path: str) -> str:
    """The install folder for an executable path (the parent of ``Bin`` when it sits in one)."""
    folder = os.path.dirname(os.path.abspath(exe_path))
    if os.path.basename(folder).lower() == "bin":
        return os.path.dirname(folder)
    return folder


def build_id(install: str) -> Dict[str, object]:
    """Describe the install: per-file size, hash and PE time stamp, patch archives, one id."""
    files: Dict[str, object] = {}
    digest = hashlib.sha256()
    for name in KEY_FILES:
        path = os.path.join(install, "Bin", name)
        if not os.path.isfile(path):
            files[name] = None
            digest.update(("%s:missing\n" % name).encode())
            continue
        sha = _sha256_file(path)
        files[name] = {
            "size": os.path.getsize(path),
            "sha256": sha,
            "pe_timestamp": pe_timestamp(path),
        }
        digest.update(("%s:%s\n" % (name, sha)).encode())
    patches: List[Dict[str, object]] = []
    try:
        names = sorted(n for n in os.listdir(install) if n.lower().startswith("patch_") and n.lower().endswith(".gro"))
    except OSError:
        names = []
    for name in names:
        size = os.path.getsize(os.path.join(install, name))
        patches.append({"name": name, "size": size})
        digest.update(("%s:%d\n" % (name, size)).encode())
    return {"id": digest.hexdigest()[:16], "files": files, "patch_archives": patches}
