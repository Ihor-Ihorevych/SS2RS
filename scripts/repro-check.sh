#!/usr/bin/env bash
# Is the release build reproducible? Builds the packages twice, in two empty target folders, with the
# release flags (scripts/dist-flags.sh), and compares the SHA-256 of every binary (roadmap T12-008).
#
#   scripts/repro-check.sh                 # ss2-server (quick: about 25 s)
#   scripts/repro-check.sh ss2-app         # the windowed app (several minutes)
#   scripts/repro-check.sh ss2-server ss2-app
#
# Exit status: 0 identical, 1 different, 2 could not build.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
packages=("$@")
[ ${#packages[@]} -gt 0 ] || packages=(ss2-server)
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

exe=""
case "$(uname -s)" in MINGW* | MSYS* | CYGWIN*) exe=".exe" ;; esac

args=()
for p in "${packages[@]}"; do args+=(-p "$p"); done

for run in a b; do
  echo "build $run: ${packages[*]}"
  flags="$(CARGO_TARGET_DIR="$work/$run" "$root/scripts/dist-flags.sh")"
  (cd "$root" && CARGO_TARGET_DIR="$work/$run" RUSTFLAGS="$flags" cargo build --profile dist --locked "${args[@]}") >"$work/$run.log" 2>&1 || {
    tail -n 20 "$work/$run.log"
    echo "repro-check: the build failed" >&2
    exit 2
  }
done

status=0
for p in "${packages[@]}"; do
  a="$(sha256sum "$work/a/dist/$p$exe" | cut -d' ' -f1 | tr -d '\\')"
  b="$(sha256sum "$work/b/dist/$p$exe" | cut -d' ' -f1 | tr -d '\\')"
  if [ "$a" = "$b" ]; then
    echo "reproducible: $p $a"
  else
    echo "DIFFERENT:    $p"
    echo "  build a: $a"
    echo "  build b: $b"
    status=1
  fi
done
exit $status
