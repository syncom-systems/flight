#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Installation as systemd user services on a clean Linux machine, following docs/install.md
# (readiness rows 10, 14). Run it on a disposable machine or CI runner: it installs units for the
# invoking user, uses Flight's default tmux socket and default configuration directory, and
# removes the units and the binary again at the end (configuration is kept, as documented).
#
#   tools/smoke/service_linux.sh /path/to/unpacked/flight-<version>-<target>   (the extracted tarball)
set -eu

PKG=${1:?usage: service_linux.sh /path/to/extracted/tarball/dir}
BASE=$(mktemp -d "${SMOKE_BASE:-${TMPDIR:-/tmp}}/flsvc.XXXXXX")
OUTER="flsvc-out-$$"
INNER="flight"
FLIGHT="$HOME/.local/bin/flight"
export XDG_RUNTIME_DIR="/run/user/$(id -u)"
mkdir -p "$BASE/work"

. "$(dirname "$0")/lib.sh"
fake_claude "$BASE"

sc() { systemctl --user "$@"; }
orch() { "$FLIGHT" orchestrator "$@"; }
node_online() { [ "$(orch node list | awk '{print $3}')" = Online ]; }
main_pid() { sc show -p MainPID --value flight-node; }

echo "1. a user manager that outlives the login (loginctl enable-linger)"
sudo loginctl enable-linger "$USER"
wait_for "the user manager" 30 test -S "$XDG_RUNTIME_DIR/systemd/private"
sc --version | head -1

echo "2. install the binary and the units as docs/install.md says"
mkdir -p "$HOME/.local/bin" "$HOME/.config/systemd/user"
install -m 755 "$PKG/flight" "$FLIGHT"
"$FLIGHT" --version
cp "$PKG/contrib/systemd/"flight-*.service "$HOME/.config/systemd/user/"
sed -i 's/ORCHESTRATOR_HOST/127.0.0.1/' "$HOME/.config/systemd/user/flight-orchestrator.service"
if command -v systemd-analyze >/dev/null; then
  systemd-analyze --user verify "$HOME/.config/systemd/user/flight-node.service" \
    "$HOME/.config/systemd/user/flight-orchestrator.service" || fail "unit files do not verify"
fi
sc daemon-reload
sc enable --now flight-orchestrator
wait_for "the orchestrator" 30 test -S "$HOME/.config/flight/orchestrator/admin.sock"

echo "3. enroll the node and a dashboard, start the node service"
"$FLIGHT" node join "$(orch enrollment create)" --name svc >/dev/null
sc enable --now flight-node
wait_for "the node to be Online" 40 node_online
"$FLIGHT" ui join "$(orch enrollment create)" --name ui1 >/dev/null
outer new-session -d -x 120 -y 36 -s s "PATH=$BASE/bin:\$PATH '$FLIGHT' ui run --refresh 1"
outer set-option -g remain-on-exit on
wait_screen 'svc' 30
# The node service has to find the stand-in agent: give it the PATH entry and restart.
sc set-environment PATH="$BASE/bin:$PATH"
sc restart flight-node
wait_for "the node to be Online" 40 node_online

echo "4. a workspace made through the service survives killing the node"
outer send-keys -t s n
wait_screen 'New workspace'
outer send-keys -t s svcws Tab BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace "$BASE/work" Tab Tab Enter
wait_screen 'Created workspace svcws|svcws +idle'
wait_windows 1
before=$(main_pid)
kill -9 "$before"
wait_for "the service to restart the node" 60 sh -c "[ \"\$(systemctl --user show -p MainPID --value flight-node)\" != $before ] && [ \"\$(systemctl --user show -p MainPID --value flight-node)\" != 0 ]"
wait_for "the node to be Online again" 60 node_online
[ "$(windows)" = 1 ] || fail "the session did not survive the node being killed"

echo "5. stopping and restarting the services leaves the sessions alone"
sc stop flight-node
sleep 2
[ "$(windows)" = 1 ] || fail "stopping the node service ended the session"
sc start flight-node
wait_for "the node to be Online" 40 node_online
sc restart flight-orchestrator
wait_for "the node to reconnect to a restarted orchestrator" 90 node_online
quit_flight

echo "6. uninstall leaves the data and the sessions"
sc disable --now flight-node flight-orchestrator
rm -f "$HOME/.config/systemd/user/flight-node.service" "$HOME/.config/systemd/user/flight-orchestrator.service" "$FLIGHT"
sc daemon-reload
[ -f "$HOME/.config/flight/node/workspaces.toml" ] || fail "the saved workspaces were removed"
[ "$(windows)" = 1 ] || fail "uninstalling ended the session"
CLEANUP='tmux -L flight kill-server; rm -rf "$HOME/.config/flight"'

echo "service journey OK"
