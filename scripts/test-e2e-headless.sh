#!/usr/bin/env bash
set -euo pipefail

binary="$(realpath "${1:?Usage: scripts/test-e2e-headless.sh /path/to/dae-wing}")"
command -v kwin_wayland >/dev/null
test_runtime="$(mktemp -d)"
trap 'rm -rf "$test_runtime"' EXIT
export XDG_RUNTIME_DIR="$test_runtime"
export XDG_CONFIG_HOME="$test_runtime/config"
export XDG_DATA_HOME="$test_runtime/data"
export WAYLAND_DISPLAY=waddle-tests
export GDK_BACKEND=wayland

dbus-run-session -- bash -euo pipefail -s -- "$binary" <<'SESSION'
QT_QPA_PLATFORM=offscreen kwin_wayland --virtual --width 1280 --height 1024 \
    --socket "$WAYLAND_DISPLAY" --no-lockscreen --no-global-shortcuts --no-kactivities \
    > "$XDG_RUNTIME_DIR/compositor.log" 2>&1 &
compositor_pid=$!
trap 'kill "$compositor_pid" 2>/dev/null || true; wait "$compositor_pid" 2>/dev/null || true' EXIT
for attempt in {1..100}; do
    if test -S "$XDG_RUNTIME_DIR/$WAYLAND_DISPLAY"; then
        bash scripts/test-e2e.sh "$1"
        exit
    fi
    kill -0 "$compositor_pid" 2>/dev/null || break
    sleep 0.05
done
cat "$XDG_RUNTIME_DIR/compositor.log" >&2
exit 1
SESSION
