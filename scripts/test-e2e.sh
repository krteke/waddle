#!/usr/bin/env bash
set -euo pipefail

export DAE_WING_BIN
DAE_WING_BIN="$(realpath "${1:?Usage: scripts/test-e2e.sh /path/to/dae-wing}")"
test_dir="$(mktemp -d)"
trap 'rm -rf "$test_dir"' EXIT
export XDG_DATA_HOME="$test_dir/data"
export XDG_CONFIG_HOME="$test_dir/config"
export WADDLE_TEST_KEYRING="$test_dir/keyring"
mkdir -m 700 -p "$WADDLE_TEST_KEYRING"
export GTK_A11Y=none
export GIO_USE_VFS=local

# Keep both the test credentials and the Secret Service off the user's session bus.
dbus-run-session -- bash -euo pipefail -c '
    printf "waddle-e2e-keyring" |
        gnome-keyring-daemon --unlock --components=secrets \
            --control-directory "$WADDLE_TEST_KEYRING" >/dev/null
    # Latency probes use SO_MARK. Grant it only inside a private network namespace.
    # Keep the keyring outside: gnome-keyring cannot drop capabilities in a user namespace.
    unshare --user --map-current-user --keep-caps --net sh -ec "
        ip link set lo up
        cargo test --offline -- --include-ignored --test-threads=1
    "
'
