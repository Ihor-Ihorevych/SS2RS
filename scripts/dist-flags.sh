#!/usr/bin/env bash
# Prints the RUSTFLAGS that make a `dist` build reproducible (roadmap T12-008, docs/RELEASING.md):
#   RUSTFLAGS="$(scripts/dist-flags.sh)" cargo build --profile dist --locked -p ss2-app
#
#  - --remap-path-prefix: the checkout and the cargo home no longer appear in the binary (panic
#    messages and debug info name /src/... and /cargo/... instead), so building in another folder
#    gives the same bytes. The target folder is remapped too when CARGO_TARGET_DIR is set.
#  - /Brepro (Windows, MSVC): the linker puts a hash, not the build time, in the PE header.
# Linux and macOS linkers already leave no time stamp.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo_home="${CARGO_HOME:-$HOME/.cargo}"
rustup_home="${RUSTUP_HOME:-$HOME/.rustup}"

# On Windows under Git Bash the Rust compiler wants the native form (C:\...), not /c/...
native() {
  if command -v cygpath >/dev/null 2>&1; then cygpath -w "$1"; else printf '%s' "$1"; fi
}

flags="--remap-path-prefix=$(native "$root")=/src"
flags="$flags --remap-path-prefix=$(native "$cargo_home")=/cargo"
flags="$flags --remap-path-prefix=$(native "$rustup_home")=/rustup"
# A build script that writes code into OUT_DIR leaves that path in the binary (the windowed app's
# glutin crate does); a target folder outside the checkout has to be remapped too.
if [ -n "${CARGO_TARGET_DIR:-}" ]; then
  flags="$flags --remap-path-prefix=$(native "$CARGO_TARGET_DIR")=/target"
fi
case "$(uname -s)" in
  MINGW* | MSYS* | CYGWIN*) flags="$flags -C link-arg=/Brepro" ;;
esac
printf '%s' "$flags"
