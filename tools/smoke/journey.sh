#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Clean-machine journey for an installed Flight binary (docs/releases/v1.0-readiness.md rows 1, 3, 4).
# Drives the real dashboard through a private outer tmux server, with a private Flight tmux
# server and config directory; it never touches your default tmux server or ~/.config/flight.
#
#   tools/smoke/journey.sh /path/to/flight
#
# Needs: tmux, a POSIX shell, mktemp. The stand-in `claude` is a program that only waits, so no
# agent account is needed.
set -eu

FLIGHT=${1:?usage: journey.sh /path/to/flight}
BASE=$(mktemp -d "${SMOKE_BASE:-${TMPDIR:-/tmp}}/flsmoke.XXXXXX")
OUTER="flsmoke-out-$$"
INNER="flsmoke-in-$$"
mkdir -p "$BASE/work" "$BASE/bin"
printf '#!/bin/sh\nexec sleep 3600\n' > "$BASE/bin/claude"
chmod +x "$BASE/bin/claude"

outer() { tmux -L "$OUTER" "$@"; }
inner() { tmux -L "$INNER" "$@"; }
cleanup() {
  outer kill-server 2>/dev/null || true
  inner kill-server 2>/dev/null || true
  rm -rf "$BASE"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1" >&2
  echo "--- screen ---" >&2
  outer capture-pane -p -t s 2>/dev/null >&2 || echo "(no outer session)" >&2
  echo "--- inner windows ---" >&2
  inner list-windows -a 2>&1 >&2 || true
  [ -f "$BASE/cfg/solo.log" ] && { echo "--- solo.log ---" >&2; tail -20 "$BASE/cfg/solo.log" >&2; }
  exit 1
}

screen() { outer capture-pane -p -t s 2>/dev/null || true; }
wait_screen() { # regex [seconds]
  local i=0 limit=$(( ${2:-30} * 5 ))
  until screen | grep -Eq -- "$1"; do
    i=$((i + 1)); [ "$i" -le "$limit" ] || fail "screen never showed: $1"
    sleep 0.2
  done
}
windows() { inner list-windows -a -F '#{session_name}' 2>/dev/null | grep -vc '^flight-view' || true; }
wait_windows() { # count
  local i=0
  until [ "$(windows)" = "$1" ]; do
    i=$((i + 1)); [ "$i" -le 150 ] || fail "expected $1 windows, have $(windows)"
    sleep 0.2
  done
}
start_flight() {
  outer kill-server 2>/dev/null || true
  outer new-session -d -x 120 -y 36 -s s \
    "PATH=$BASE/bin:\$PATH FLIGHT_CONFIG_DIR=$BASE/cfg '$FLIGHT' solo --socket $INNER"
}
quit_flight() {
  outer send-keys -t s q
  local i=0
  while outer has-session -t s 2>/dev/null; do
    i=$((i + 1)); [ "$i" -le 100 ] || fail "flight did not quit"
    sleep 0.2
  done
}

echo "1. version"
"$FLIGHT" --version | grep -Eq '^flight [0-9]+\.[0-9]+\.[0-9]+' || fail "--version"

echo "2. first run reaches the dashboard"
start_flight
wait_screen 'No Flight workspaces yet' 60

echo "3. create a workspace"
outer send-keys -t s n
wait_screen 'New workspace'
outer send-keys -t s smoke Tab BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace "$BASE/work" Tab Tab Enter
wait_screen 'QUIET|smoke +idle'
wait_windows 1

echo "4. companion shell, then another"
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

echo "5. name a shell"
outer send-keys -t s R
wait_screen 'Name a shell'
outer send-keys -t s logs Enter
wait_screen 's logs +ready'

echo "6. quitting the dashboard keeps the sessions; coming back finds them"
quit_flight
[ "$(windows)" = 3 ] || fail "sessions did not survive the dashboard"
start_flight
wait_screen 's logs +ready'
wait_screen 'Shell 2 +ready'

echo "7. a lost tmux server leaves the workspace saved; one restore brings back everything"
inner kill-server
wait_screen 'SAVED'
outer send-keys -t s f
wait_screen 'Start a new conversation'
outer send-keys -t s y
wait_windows 3
wait_screen 's logs +ready'
quit_flight

echo "journey OK"
