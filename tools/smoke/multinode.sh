#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Several machines, scripted (docs/releases/v1.0-readiness.md, rows 2b and 3): an orchestrator, two
# nodes and a dashboard, each in its own container on a private network, the way
# docs/two-machines.md sets them up. Then the faults an operator meets: one node's network is
# cut and restored, a node process is killed, the orchestrator is restarted, a node is revoked.
# Sessions must survive all of it, and the other node must not notice. Needs docker and a
# Linux x86_64 flight binary; touches nothing outside its own containers and network.
#
#   tools/smoke/multinode.sh /path/to/linux/flight
set -eu

FLIGHT=$(cd "$(dirname "${1:?usage: multinode.sh /path/to/linux/flight}")" && pwd)/$(basename "$1")
ID=$$
NET="flmn-$ID"
IMG="flmn-img-$ID"
BASE=$(mktemp -d "${TMPDIR:-/tmp}/flmn.XXXXXX")
c() { echo "flmn-$ID-$1"; }
dx() { local who=$1; shift; docker exec "$(c "$who")" "$@"; }
dxd() { local who=$1; shift; docker exec -d "$(c "$who")" "$@"; }

cleanup() {
  for n in orch n1 n2 ui; do docker rm -f "$(c $n)" >/dev/null 2>&1 || true; done
  docker network rm "$NET" >/dev/null 2>&1 || true
  docker rmi -f "$IMG" >/dev/null 2>&1 || true
  rm -rf "$BASE"
}
trap cleanup EXIT

fail() {
  echo "FAIL: $1" >&2
  for n in orch n1 n2; do
    echo "--- $n ---" >&2
    dx $n sh -c 'tail -n 15 /var/log/flight.log 2>/dev/null' >&2 || true
  done
  echo "--- orchestrator view ---" >&2
  dx orch flight orchestrator node list --config-dir /cfg >&2 || true
  echo "--- dashboard screen ---" >&2
  dx ui tmux -L out capture-pane -p -t s >&2 || true
  exit 1
}

wait_for() { # description seconds command...
  local what=$1 secs=$2 i=0
  shift 2
  until "$@" >/dev/null 2>&1; do
    i=$((i + 1)); [ "$i" -le $((secs * 2)) ] || fail "timed out waiting for: $what"
    sleep 0.5
  done
}
nodes() { dx orch flight orchestrator node list --config-dir /cfg; }
state_of() { nodes | awk -v n="$1" '$NF == n {print $3}'; }
is() { [ "$(state_of "$1")" = "$2" ]; }
not_online() { [ "$(state_of "$1")" != Online ]; }
sessions() { dx "$1" sh -c 'tmux -L flight list-sessions 2>/dev/null | grep -vc "^flight-view" || true'; }
screen() { dx ui tmux -L out capture-pane -p -t s 2>/dev/null || true; }
wait_screen() { local i=0; until screen | grep -Eq -- "$1"; do i=$((i + 1)); [ "$i" -le 80 ] || fail "screen never showed: $1"; sleep 0.5; done; }
start_orch() { dxd orch sh -c 'exec flight orchestrator run --listen 0.0.0.0:7676 --advertise orch:7676 --config-dir /cfg >> /var/log/flight.log 2>&1'; wait_for "the orchestrator" 30 dx orch test -S /cfg/orchestrator/admin.sock; }
start_node() { dxd "$1" sh -c 'exec flight node run --interval 1 --config-dir /cfg >> /var/log/flight.log 2>&1'; }
stop_node() { dx "$1" pkill -9 -f "flight node run" || true; }

echo "0. machines: an orchestrator, two nodes and a dashboard on a private network"
mkdir "$BASE/ctx"
cp "$FLIGHT" "$BASE/ctx/flight"
printf '#!/bin/sh\nexec sleep 3600\n' > "$BASE/ctx/claude"
cat > "$BASE/ctx/Dockerfile" <<'DOCKER'
FROM debian:12-slim
RUN apt-get update -qq && apt-get install -y -qq tmux procps && rm -rf /var/lib/apt/lists/*
COPY flight claude /usr/local/bin/
RUN chmod 755 /usr/local/bin/flight /usr/local/bin/claude && mkdir -p /work
DOCKER
docker build -q -t "$IMG" "$BASE/ctx" >/dev/null
docker network create "$NET" >/dev/null
for n in orch n1 n2 ui; do
  docker run -d --name "$(c $n)" --hostname "$n" --network "$NET" --network-alias "$n" "$IMG" sleep infinity >/dev/null
done
dx orch flight --version | grep -Eq '^flight [0-9]+\.[0-9]+\.[0-9]+' || fail "--version"

echo "1. the orchestrator serves; two nodes and a dashboard enroll with one-time bundles"
start_orch
for n in n1 n2; do
  bundle=$(dx orch flight orchestrator enrollment create --config-dir /cfg)
  # shellcheck disable=SC2086  # the bundle is several words
  dx $n flight node join $bundle --name $n --config-dir /cfg >/dev/null || fail "$n join"
  start_node $n
done
bundle=$(dx orch flight orchestrator enrollment create --config-dir /cfg)
# shellcheck disable=SC2086
dx ui flight ui join $bundle --name ui1 --config-dir /cfg >/dev/null || fail "ui join"
wait_for "n1 Online" 60 is n1 Online
wait_for "n2 Online" 60 is n2 Online

echo "2. the dashboard sees both and starts a workspace on each"
dx ui tmux -L out new-session -d -x 120 -y 36 -s s "flight ui run --refresh 1 --config-dir /cfg"
wait_screen 'n1'
wait_screen 'n2'
for k in 0 1; do
  dx ui tmux -L out send-keys -t s n
  wait_screen 'New workspace'
  dx ui tmux -L out send-keys -t s "ws$k" Up
  i=0; while [ "$i" -lt "$k" ]; do dx ui tmux -L out send-keys -t s Right; i=$((i + 1)); done
  dx ui tmux -L out send-keys -t s Tab Tab
  dx ui tmux -L out send-keys -t s BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace /work Tab Tab Enter
  wait_screen "ws$k +idle|Created workspace ws$k"
done
wait_for "one session on each node" 30 sh -c "[ \"\$(docker exec $(c n1) sh -c 'tmux -L flight list-sessions 2>/dev/null | grep -vc ^flight-view || true')\" = 1 ] && [ \"\$(docker exec $(c n2) sh -c 'tmux -L flight list-sessions 2>/dev/null | grep -vc ^flight-view || true')\" = 1 ]"

echo "3. n2's network is cut: n2 is lost, n1 is unaffected, n2 returns by itself, its session survived"
docker network disconnect "$NET" "$(c n2)"
wait_for "n2 to leave Online" 60 not_online n2
is n1 Online || fail "n1 noticed n2's network"
[ "$(sessions n2)" = 1 ] || fail "n2's session did not survive the network loss"
docker network connect --alias n2 "$NET" "$(c n2)"
wait_for "n2 Online again" 90 is n2 Online
[ "$(sessions n2)" = 1 ] || fail "n2's session is gone after the network returned"

echo "4. a node process is killed and started again: the session survives and is picked up"
stop_node n1
wait_for "n1 to leave Online" 60 not_online n1
[ "$(sessions n1)" = 1 ] || fail "n1's session did not survive the node"
start_node n1
wait_for "n1 Online again" 60 is n1 Online

echo "5. the orchestrator is restarted: both nodes reconnect without enrolling again"
dx orch pkill -9 -f "orchestrator run" || true
sleep 1
start_orch
wait_for "n1 Online after the orchestrator restart" 90 is n1 Online
wait_for "n2 Online after the orchestrator restart" 90 is n2 Online
[ "$(sessions n1)" = 1 ] && [ "$(sessions n2)" = 1 ] || fail "a session was lost with the orchestrator"

echo "6. n2 is revoked: it is cut off at once, n1 is not, and n2's session keeps running"
id=$(dx orch flight orchestrator trust list --config-dir /cfg | awk '/ n2$/ {print $2}')
[ -n "$id" ] || fail "trust list does not show n2"
dx orch flight orchestrator trust revoke "$id" --config-dir /cfg >/dev/null
wait_for "n2 to be cut off" 60 not_online n2
is n1 Online || fail "revoking n2 affected n1"
stop_node n2
start_node n2
sleep 8
is n2 Online && fail "a revoked node came back"
[ "$(sessions n2)" = 1 ] || fail "revocation ended n2's session"

echo "multi-node journey OK"
