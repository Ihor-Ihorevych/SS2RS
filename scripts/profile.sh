#!/usr/bin/env sh
# Run ss2-app with the puffin profiler built in and switched on (docs/PROFILING.md).
# Extra arguments go to the app. Set SS2_PROFILE to another address to change the port.
set -eu
cd "$(dirname "$0")/.."
SS2_PROFILE="${SS2_PROFILE:-1}"
export SS2_PROFILE
if command -v puffin_viewer >/dev/null 2>&1; then
    puffin_viewer --url 127.0.0.1:8585 &
    viewer=$!
    trap 'kill "$viewer" 2>/dev/null || true' EXIT
else
    echo "profile: puffin_viewer is not installed (cargo install --locked puffin_viewer); serving anyway" >&2
fi
cargo run -p ss2-app --features profile -- "$@"
