# SPDX-License-Identifier: MIT
#
# Shared helpers for the smoke scripts (sourced, not run). The caller sets FLIGHT, BASE, OUTER
# and INNER before sourcing and may append to CLEANUP.

outer() { tmux -L "$OUTER" "$@"; }
inner() { tmux -L "$INNER" "$@"; }

CLEANUP=""
cleanup() {
  outer kill-server 2>/dev/null || true
  inner kill-server 2>/dev/null || true
  eval "$CLEANUP" 2>/dev/null || true
  rm -rf "$BASE"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1" >&2
  echo "--- screen ---" >&2
  outer capture-pane -p -t s >&2 2>&1 || echo "(no outer session)" >&2
  echo "--- inner windows ---" >&2
  inner list-windows -a >&2 2>&1 || true
  echo "--- outer pane state ---" >&2
  outer display-message -p -t s '#{pane_dead} alt=#{alternate_on} cmd=#{pane_current_command} #{pane_width}x#{pane_height}' >&2 2>&1 || true
  outer capture-pane -p -t s -e 2>&1 | od -c | head -20 >&2 || true
  echo "--- processes and sockets ---" >&2
  ps -eo pid,args 2>/dev/null | grep -E 'flight|tmux' | grep -v grep >&2 || true
  ls -la "/tmp/tmux-$(id -u)" >&2 2>&1 || true
  if command -v journalctl >/dev/null 2>&1 && [ -n "${XDG_RUNTIME_DIR:-}" ]; then
    echo "--- journal (user units) ---" >&2
    journalctl --user -n 40 --no-pager >&2 2>&1 || true
  fi
  for log in "$BASE"/*.log "$BASE"/cfg/solo.log; do
    [ -f "$log" ] && { echo "--- $log ---" >&2; tail -15 "$log" >&2; }
  done
  exit 1
}

screen() { outer capture-pane -p -t s 2>/dev/null || true; }

wait_for() { # description seconds command...
  local what=$1 secs=$2 i=0
  shift 2
  until "$@" >/dev/null 2>&1; do
    i=$((i + 1)); [ "$i" -le $((secs * 5)) ] || fail "timed out waiting for: $what"
    sleep 0.2
  done
}

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

quit_flight() {
  outer send-keys -t s q
  local i=0
  while outer has-session -t s 2>/dev/null; do
    i=$((i + 1)); [ "$i" -le 100 ] || fail "flight did not quit"
    sleep 0.2
  done
}

fake_claude() { # dir
  mkdir -p "$1/bin"
  printf '#!/bin/sh\nexec sleep 3600\n' > "$1/bin/claude"
  chmod +x "$1/bin/claude"
}
