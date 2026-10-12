#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# The real Claude CLI in a workspace (docs/releases/v1.0-readiness.md, row 1b), scripted: create a
# workspace, talk to Claude, quit and return (the session survives), lose the private tmux server,
# restore, and show that the earlier conversation was resumed. Needs a logged-in `claude` and uses
# a few tokens of your account. Private tmux servers and a private configuration directory; your
# own tmux server and ~/.config/flight are never touched.
#
#   tools/smoke/real-claude.sh /path/to/flight
set -eu

FLIGHT=${1:?usage: real-claude.sh /path/to/flight}
command -v claude >/dev/null || { echo "claude is not installed or not on PATH; nothing to check" >&2; exit 2; }
BASE=$(mktemp -d "${SMOKE_BASE:-${TMPDIR:-/tmp}}/flclaude.XXXXXX")
OUTER="flclaude-out-$$"
INNER="flclaude-in-$$"
mkdir -p "$BASE/work"

. "$(dirname "$0")/lib.sh"

start_flight() {
  outer kill-server 2>/dev/null || true
  outer new-session -d -x 120 -y 40 -s s \
    "FLIGHT_CONFIG_DIR=$BASE/cfg '$FLIGHT' solo --socket $INNER"
}
agent_screen() { inner capture-pane -p -S -200 -t "$(inner list-panes -a -F '#{session_name}:#{window_name}' | grep ':agent$' | grep -v '^flight-view' | head -1)" 2>/dev/null || true; }
wait_agent() { # regex [seconds]
  local i=0 limit=$(( ${2:-60} * 5 ))
  until agent_screen | grep -Eq -- "$1"; do
    i=$((i + 1)); [ "$i" -le "$limit" ] || fail "the agent never showed: $1"
    # A first run in a new folder asks whether to trust it: accept.
    if agent_screen | grep -qiE 'trust (the files|this folder)|Do you trust'; then
      inner send-keys -t "$(inner list-panes -a -F '#{session_name}:#{window_name}' | grep ':agent$' | grep -v '^flight-view' | head -1)" Enter
    fi
    sleep 0.2
  done
}
say() { # text: type into the agent and send it
  local pane
  pane=$(inner list-panes -a -F '#{session_name}:#{window_name}' | grep ':agent$' | grep -v '^flight-view' | head -1)
  inner send-keys -t "$pane" -l "$1"
  sleep 1
  inner send-keys -t "$pane" Enter
}

echo "1. version, and the agent is the real one"
"$FLIGHT" --version | grep -Eq '^flight [0-9]+\.[0-9]+\.[0-9]+' || fail "--version"
echo "   $(claude --version 2>&1 | head -1)"

echo "2. create a workspace with the real Claude"
start_flight
wait_screen 'No Flight workspaces yet' 60
outer send-keys -t s n
wait_screen 'New workspace'
outer send-keys -t s proj Tab BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace "$BASE/work" Tab Tab Enter
wait_screen 'Created workspace proj|proj +(idle|ready|working)'
wait_windows 1
wait_agent '[Cc]laude' 90

echo "3. a conversation, held in the session"
say "Remember the word PINEAPPLE. Reply with OK only."
wait_agent '^. ?OK|⏺ OK|● OK' 120

echo "4. quitting the dashboard keeps the session; coming back finds it"
quit_flight
[ "$(windows)" = 1 ] || fail "the session did not survive the dashboard"
start_flight
wait_screen 'proj +(idle|ready|working)' 60

echo "5. the tmux server is lost: the workspace is saved, not running"
inner kill-server
wait_screen 'SAVED'

echo "6. restore: Enter starts it again, and Claude resumes the earlier conversation"
outer send-keys -t s Enter
wait_windows 1
wait_agent 'PINEAPPLE' 120
say "Reply with the word I asked you to remember, spelled backwards, nothing else."
wait_agent 'ELPPAENIP' 120

quit_flight
echo "real Claude journey OK"
