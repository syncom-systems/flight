#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Distributed journey for an installed Flight binary (docs/releases/v1.0-readiness.md rows 2, 3, 8).
# One orchestrator, one node and one dashboard, each a separate process with its own identity and
# configuration, talking over mutual TLS on TCP, driven the way an operator would. Private tmux
# servers and a private configuration directory; the default tmux server is never touched.
#
#   tools/smoke/distributed.sh /path/to/flight
#
# ORCH_HOST (default 127.0.0.1) is the address the orchestrator listens on and advertises.
set -eu

FLIGHT=${1:?usage: distributed.sh /path/to/flight}
BASE=$(mktemp -d "${SMOKE_BASE:-${TMPDIR:-/tmp}}/fldist.XXXXXX")
OUTER="fldist-out-$$"
INNER="fldist-in-$$"
HOST=${ORCH_HOST:-127.0.0.1}
PORT=$((20000 + RANDOM % 20000))
mkdir -p "$BASE/work"

. "$(dirname "$0")/lib.sh"
fake_claude "$BASE"
CLEANUP='pkill -f "$BASE" || true'

orch() { "$FLIGHT" orchestrator "$@" --config-dir "$BASE/orch"; }
start_orchestrator() {
  "$FLIGHT" orchestrator run --listen "$HOST:$PORT" --advertise "$HOST:$PORT" \
    --config-dir "$BASE/orch" >> "$BASE/orchestrator.log" 2>&1 &
  wait_for "the orchestrator" 30 test -S "$BASE/orch/orchestrator/admin.sock"
}
start_node() {
  PATH="$BASE/bin:$PATH" "$FLIGHT" node run --socket "$INNER" --interval 1 \
    --config-dir "$BASE/node" >> "$BASE/node.log" 2>&1 &
}
node_state() { orch node list | awk '{print $3}'; }
node_is() { [ "$(node_state)" = "$1" ]; }
node_not_online() { ! node_is Online; }
stop_node() { pkill -9 -f "node run --socket $INNER" || true; }
start_dashboard() {
  outer kill-server 2>/dev/null || true
  outer new-session -d -x 120 -y 36 -s s \
    "'$FLIGHT' ui run --refresh 1 --config-dir '$BASE/ui'"
}

echo "1. version"
"$FLIGHT" --version | grep -Eq '^flight [0-9]+\.[0-9]+\.[0-9]+' || fail "--version"

echo "2. orchestrator serves; a node enrolls with a one-time bundle"
start_orchestrator
BUNDLE=$(orch enrollment create)
# shellcheck disable=SC2086  # the bundle is several words, passed as separate arguments
"$FLIGHT" node join $BUNDLE --name n1 --config-dir "$BASE/node" >/dev/null || fail "node join"
# shellcheck disable=SC2086
if "$FLIGHT" node join $BUNDLE --name again --config-dir "$BASE/node-again" >/dev/null 2>&1; then
  fail "a bundle was accepted twice"
fi
orch enrollment create --ttl 1 > "$BASE/short.bundle"
sleep 3
# shellcheck disable=SC2046
if "$FLIGHT" node join $(cat "$BASE/short.bundle") --name late --config-dir "$BASE/node-late" >/dev/null 2>&1; then
  fail "an expired bundle was accepted"
fi
[ ! -e "$BASE/node-again" ] && [ ! -e "$BASE/node-late" ] || fail "a refused join left files behind"

echo "3. the node reports in; a dashboard enrolls and sees it"
start_node
wait_for "the node to be Online" 40 node_is Online
UIBUNDLE=$(orch enrollment create)
# shellcheck disable=SC2086
"$FLIGHT" ui join $UIBUNDLE --name ui1 --config-dir "$BASE/ui" >/dev/null || fail "ui join"
start_dashboard
wait_screen 'n1' 30

echo "4. create a workspace on the node from the dashboard"
outer send-keys -t s n
wait_screen 'New workspace'
outer send-keys -t s dist Tab BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace "$BASE/work" Tab Tab Enter
wait_screen 'Created workspace dist|dist +idle'
wait_windows 1

echo "5. the node dies and returns: the session survives and is shown again"
stop_node
wait_for "the node to be Disconnected" 60 node_is Disconnected
[ "$(windows)" = 1 ] || fail "the session did not survive the node"
start_node
wait_for "the node to be Online again" 60 node_is Online
wait_screen 'dist +idle' 30

echo "6. the orchestrator dies and returns: everything reconnects"
pkill -9 -f "orchestrator run --listen $HOST:$PORT" || true
sleep 1
start_orchestrator
wait_for "the node to be Online after an orchestrator restart" 90 node_is Online
wait_screen 'dist +idle' 60
[ "$(windows)" = 1 ] || fail "the session did not survive the orchestrator"

echo "7. the dashboard goes away and comes back to the same session"
quit_flight
start_dashboard
wait_screen 'dist +idle' 30
quit_flight

echo "8. revoking the node's trust cuts it off at once"
ID=$(orch trust list | awk '/ n1$/ {print $2}')
[ -n "$ID" ] || fail "trust list does not show the node: $(orch trust list)"
orch trust revoke "$ID" >/dev/null
wait_for "the revoked node to be cut off" 60 node_not_online
stop_node
start_node
sleep 8
node_is Online && fail "a revoked node came back"
[ "$(windows)" = 1 ] || fail "revocation ended the node's session"

echo "distributed journey OK"
