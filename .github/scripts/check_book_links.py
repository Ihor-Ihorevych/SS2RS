"""Fail if a built mdBook contains a broken internal link.

mdBook does not validate links. Usage: python3 check_book_links.py <built book dir>

Only relative links to files are checked (external URLs and in-page anchors are skipped).
"""

import os
import re
import sys

SKIP_PAGES = {"print.html", "toc.html"}
EXTERNAL = re.compile(r"^(https?:|mailto:|//)")
HREF = re.compile(r'<a [^>]*href="([^"#]+)(#[^"]*)?"')


def main(book_dir: str) -> int:
    root = os.path.abspath(book_dir)
    checked = 0
    broken = set()
    for dirpath, _, files in os.walk(root):
        for name in files:
            if not name.endswith(".html") or name in SKIP_PAGES:
                continue
            page = os.path.join(dirpath, name)
            with open(page, encoding="utf-8", errors="replace") as handle:
                text = handle.read()
            for match in HREF.finditer(text):
                href = match.group(1)
                if EXTERNAL.match(href):
                    continue
                checked += 1
                target = os.path.normpath(os.path.join(dirpath, href))
                if not os.path.exists(target):
                    broken.add((os.path.relpath(page, root).replace(os.sep, "/"), href))
    print(f"internal links checked: {checked}, broken: {len(broken)}")
    for page, href in sorted(broken):
        print(f"  {page}: {href}")
    return 1 if broken else 0


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit("usage: check_book_links.py <built book dir>")
    sys.exit(main(sys.argv[1]))
