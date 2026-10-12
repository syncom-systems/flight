#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# The node as a launchd service on macOS (docs/releases/v1.0-readiness.md, row 14b), scripted, following
# contrib/launchd/README.md: the template is filled in, loaded, killed with SIGKILL and must come
# back as a new process, reconnect, and leave the tmux session untouched. The job has a unique
# label and a private socket and configuration directory, and is removed at the end.
#
#   tools/smoke/service_macos.sh /path/to/flight
set -eu

FLIGHT=$(cd "$(dirname "${1:?usage: service_macos.sh /path/to/flight}")" && pwd)/$(basename "$1")
REPO=$(cd "$(dirname "$0")/../.." && pwd)
ID=$$
BASE=$(mktemp -d "${SMOKE_BASE:-/tmp}/flsvc.XXXXXX")
LABEL="name.flight.smoke$ID"
SOCK="flsvc-$ID"
PORT=$((20000 + RANDOM % 20000))
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
DOMAIN=""

cleanup() {
  [ -n "$DOMAIN" ] && launchctl bootout "$DOMAIN/$LABEL" >/dev/null 2>&1 || true
  rm -f "$PLIST"
  tmux -L "$SOCK" kill-server >/dev/null 2>&1 || true
  pkill -f "$BASE" >/dev/null 2>&1 || true
  rm -rf "$BASE"
}
trap cleanup EXIT
fail() {
  echo "FAIL: $1" >&2
  [ -f "$BASE/logs/node.out.log" ] && { echo "--- node log ---" >&2; tail -n 20 "$BASE/logs/node.out.log" >&2; }
  [ -f "$BASE/logs/node.err.log" ] && { echo "--- node err ---" >&2; tail -n 10 "$BASE/logs/node.err.log" >&2; }
  [ -n "$DOMAIN" ] && launchctl print "$DOMAIN/$LABEL" >&2 2>&1 || true
  exit 1
}
wait_for() { local what=$1 secs=$2 i=0; shift 2; until "$@" >/dev/null 2>&1; do i=$((i + 1)); [ "$i" -le $((secs * 2)) ] || fail "timed out waiting for: $what"; sleep 0.5; done; }
orch() { "$FLIGHT" orchestrator "$@" --config-dir "$BASE/orch"; }
online() { orch node list | grep -q ' Online '; }
job_pid() { launchctl print "$DOMAIN/$LABEL" | awk '/^\tpid =/{print $3}'; }
job_runs() { launchctl print "$DOMAIN/$LABEL" | awk '/^\truns =/{print $3}'; }
running() { [ -n "$(job_pid)" ]; }

mkdir -p "$BASE/logs" "$HOME/Library/LaunchAgents"

echo "1. an orchestrator, and a node enrolled with it"
"$FLIGHT" orchestrator run --listen "127.0.0.1:$PORT" --advertise "127.0.0.1:$PORT" --config-dir "$BASE/orch" > "$BASE/orch.log" 2>&1 &
wait_for "the orchestrator" 30 test -S "$BASE/orch/orchestrator/admin.sock"
# shellcheck disable=SC2046
"$FLIGHT" node join $(orch enrollment create) --name svc --config-dir "$BASE/node" >/dev/null || fail "node join"

echo "2. the template of contrib/launchd is filled in and passes plutil"
tmux_dir=$(dirname "$(command -v tmux)")
sed -e "s|@FLIGHT_BIN@|$FLIGHT|" -e "s|@NODE_CONFIG_DIR@|$BASE/node|" -e "s|@LOG_DIR@|$BASE/logs|" \
    -e "s|<string>name.flight.node</string>|<string>$LABEL</string>|" \
    -e "s|<string>flight</string>|<string>$SOCK</string>|" \
    -e "s|/opt/homebrew/bin:|$tmux_dir:/opt/homebrew/bin:|" \
    "$REPO/contrib/launchd/name.flight.node.plist" > "$PLIST"
plutil -lint "$PLIST" >/dev/null || fail "plutil -lint"

echo "3. launchctl bootstrap: the node runs and connects"
for d in "gui/$(id -u)" "user/$(id -u)"; do
  if launchctl bootstrap "$d" "$PLIST" 2>"$BASE/bootstrap.err"; then DOMAIN=$d; break; fi
  echo "   bootstrap into $d failed: $(cat "$BASE/bootstrap.err")"
done
[ -n "$DOMAIN" ] || fail "launchctl could not load the job in any domain"
echo "   loaded in $DOMAIN"
wait_for "the job to run" 30 running
wait_for "the node to be Online" 60 online

echo "4. a session in the node's tmux, then kill -9 of the node"
tmux -L "$SOCK" new-session -d -s keep "sleep 3600"
P0=$(job_pid); R0=$(job_runs)
kill -9 "$P0"
wait_for "launchd to start a new process" 60 sh -c "[ \"\$(launchctl print $DOMAIN/$LABEL | awk '/^\\tpid =/{print \$3}')\" != \"$P0\" ] && [ -n \"\$(launchctl print $DOMAIN/$LABEL | awk '/^\\tpid =/{print \$3}')\" ]"
P1=$(job_pid)
[ "$P1" != "$P0" ] || fail "the pid did not change"
[ "$(job_runs)" -gt "$R0" ] || fail "launchd did not count a new run"
wait_for "the node to be Online again" 60 online
tmux -L "$SOCK" has-session -t keep || fail "the session did not survive the node"
echo "   pid $P0 -> $P1"

echo "5. bootout stops it, and the session still survives"
launchctl bootout "$DOMAIN/$LABEL"
wait_for "the node to stop" 30 sh -c "! pgrep -f 'node run --socket $SOCK' >/dev/null"
tmux -L "$SOCK" has-session -t keep || fail "the session did not survive the service stopping"

echo "launchd journey OK"
