"""Validate PNG files with Pillow (an independent decoder).

    python -I validate_png.py <file-or-folder>... [--quiet]

Each file is fully decoded (`Image.load()` after `verify()`); the summary lists the counts by mode
and the files that failed. Exit status 0 if every file decodes, 1 otherwise.
"""
import json
import os
import sys

from PIL import Image


def walk(path):
    if os.path.isdir(path):
        for name in sorted(os.listdir(path)):
            yield from walk(os.path.join(path, name))
    elif path.lower().endswith(".png"):
        yield path


def main(argv):
    quiet = "--quiet" in argv
    paths = [a for a in argv if not a.startswith("--")]
    modes, failures, files, pixels = {}, [], 0, 0
    for arg in paths:
        for f in walk(arg):
            files += 1
            try:
                with Image.open(f) as im:
                    im.verify()
                with Image.open(f) as im:
                    im.load()
                    modes[im.mode] = modes.get(im.mode, 0) + 1
                    pixels += im.width * im.height
            except Exception as e:  # noqa: BLE001 - report any decoder failure
                failures.append({"file": f, "error": str(e)})
                if not quiet:
                    print("ERR ", f, e)
    print(json.dumps({"pillow": getattr(__import__("PIL"), "__version__", "?"), "files": files,
                      "pixels": pixels, "modes": modes, "failures": failures[:20],
                      "failed": len(failures)}, indent=1))
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
