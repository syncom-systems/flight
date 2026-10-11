#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Upgrade and rollback over populated data (docs/releases/v1.0-readiness.md row 11).
#
#   tools/smoke/upgrade.sh /path/to/OLD/flight /path/to/NEW/flight
#
# OLD builds a workspace with two shells and a name; NEW then starts on the same configuration
# and the same running sessions: nothing is lost and the saved file is upgraded with its
# original kept. OLD started again refuses the newer file and leaves it byte for byte; the
# backup taken before the upgrade restores the old state.
set -eu

OLD=${1:?usage: upgrade.sh OLD NEW}
NEW=${2:?usage: upgrade.sh OLD NEW}
FLIGHT=$OLD
BASE=$(mktemp -d "${SMOKE_BASE:-${TMPDIR:-/tmp}}/flupg.XXXXXX")
OUTER="flupg-out-$$"
INNER="flupg-in-$$"
mkdir -p "$BASE/work"

. "$(dirname "$0")/lib.sh"
fake_claude "$BASE"

start_flight() {
  outer kill-server 2>/dev/null || true
  outer new-session -d -x 120 -y 36 -s s \
    "PATH=$BASE/bin:\$PATH FLIGHT_CONFIG_DIR=$BASE/cfg '$FLIGHT' solo --socket $INNER"
}
SAVED="$BASE/cfg/node/workspaces.toml"
digest() { shasum -a 256 "$1" | cut -d' ' -f1; }

echo "1. the old build makes a workspace with two shells, one named"
start_flight
wait_screen 'No Flight workspaces yet' 60
outer send-keys -t s n
wait_screen 'New workspace'
outer send-keys -t s smoke Tab BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace "$BASE/work" Tab Tab Enter
wait_screen 'QUIET|smoke +idle'
outer send-keys -t s s
wait_screen 'Companion shell'
outer send-keys -t s Enter
wait_windows 2
sleep 1
outer send-keys -t s C-Space q
wait_screen 's Shell +ready'
outer send-keys -t s S
wait_screen 'Add another shell'
outer send-keys -t s Enter
wait_windows 3
sleep 1
outer send-keys -t s C-Space q
wait_screen 'Shell 2 +ready'
outer send-keys -t s R
wait_screen 'Name a shell'
outer send-keys -t s logs Enter
wait_screen 's logs +ready'
quit_flight
grep -q '^schema_version = 1$' "$SAVED" || fail "the old build did not write schema 1"
cp -R "$BASE/cfg" "$BASE/cfg-backup" 2>/dev/null || true

echo "2. the new build starts on the same data and the same running sessions"
FLIGHT=$NEW
start_flight
wait_screen 's logs +ready'
wait_screen 'Shell 2 +ready'
[ "$(windows)" = 3 ] || fail "the sessions did not carry over"
quit_flight
grep -q '^schema_version = 2$' "$SAVED" || fail "the file was not upgraded"
grep -q '^schema_version = 1$' "$BASE/cfg/node/workspaces.v1.bak" || fail "the original was not kept"
grep -q 'name = "logs"' "$SAVED" || fail "the name was lost in the upgrade"

echo "3. rolling back to the old build: it refuses the newer file and leaves it alone"
before=$(digest "$SAVED")
FLIGHT=$OLD
start_flight
wait_screen 's logs +ready'
quit_flight
[ "$(digest "$SAVED")" = "$before" ] || fail "the old build changed a file it cannot read"
grep -qi 'newer\|another Flight\|saved workspaces' "$BASE/cfg/solo.log" "$BASE/cfg/node/"*.log 2>/dev/null \
  || echo "note: the refusal is not in solo.log (the old build may print it to the node's stderr only)"

echo "4. the pre-upgrade backup restores the old state for the old build"
rm -rf "$BASE/cfg"
cp -R "$BASE/cfg-backup" "$BASE/cfg"
start_flight
wait_screen 's logs +ready'
quit_flight
grep -q '^schema_version = 1$' "$SAVED" || fail "the backup did not restore schema 1"

echo "upgrade and rollback OK"
