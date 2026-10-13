#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# The real Codex CLI in a workspace (ADR-013, scenario 6 of the equivalence suite), scripted:
# choose Codex in setup's settings, create a workspace from the dashboard, quit and return (the
# session survives), lose the private tmux server, restore. Codex cannot be asked to continue a
# conversation Flight did not identify (ADR-010), so the restore must start a NEW Codex and must
# not ask it to resume. No model is asked anything, so no tokens are spent; Codex shows its own
# screen (or its login prompt) either way. Private tmux servers and configuration directory.
#
#   tools/smoke/real-codex.sh /path/to/flight
set -eu

FLIGHT=${1:?usage: real-codex.sh /path/to/flight}
command -v codex >/dev/null || { echo "codex is not installed or not on PATH; nothing to check" >&2; exit 2; }
BASE=$(mktemp -d "${SMOKE_BASE:-${TMPDIR:-/tmp}}/flcodex.XXXXXX")
OUTER="flcodex-out-$$"
INNER="flcodex-in-$$"
mkdir -p "$BASE/work" "$BASE/cfg"

. "$(dirname "$0")/lib.sh"

# What setup would have saved for a person who works with Codex.
cat > "$BASE/cfg/settings.toml" <<'TOML'
version = 1

[agents]
enabled = ["codex"]
default = "codex"

[setup]
completed = true
declined = false
TOML

start_flight() {
  outer kill-server 2>/dev/null || true
  outer new-session -d -x 120 -y 40 -s s \
    "FLIGHT_NO_SETUP=1 FLIGHT_CONFIG_DIR=$BASE/cfg '$FLIGHT' solo --socket $INNER"
}
agent_pane() { inner list-panes -a -F '#{session_name}:#{window_name}' | grep ':agent$' | grep -v '^flight-view' | head -1; }
agent_screen() { inner capture-pane -p -S -200 -t "$(agent_pane)" 2>/dev/null || true; }
started_with() { inner display-message -p -t "$(agent_pane)" '#{pane_start_command}' 2>/dev/null || true; }
wait_agent() { # regex [seconds]
  local i=0 limit=$(( ${2:-60} * 5 ))
  until agent_screen | grep -Eqi -- "$1"; do
    i=$((i + 1)); [ "$i" -le "$limit" ] || fail "the agent never showed: $1"
    sleep 0.2
  done
}

echo "1. version, and the agent is the real one"
"$FLIGHT" --version | grep -Eq '^flight [0-9]+\.[0-9]+\.[0-9]+' || fail "--version"
echo "   $(codex --version 2>&1 | head -1)"

echo "2. the form offers Codex first; create a workspace with it"
start_flight
wait_screen 'No Flight workspaces yet' 60
outer send-keys -t s n
wait_screen 'New workspace'
wait_screen 'Codex'
outer send-keys -t s proj Tab BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace BSpace "$BASE/work" Tab Tab Enter
wait_screen 'Created workspace proj|proj +(idle|ready|working)'
wait_windows 1
case "$(started_with)" in */codex*) ;; *) fail "the workspace is not running codex: $(started_with)" ;; esac
wait_agent 'codex|openai|sign in|log in' 90

echo "3. quitting the dashboard keeps the session; coming back finds it"
quit_flight
[ "$(windows)" = 1 ] || fail "the session did not survive the dashboard"
start_flight
wait_screen 'proj +(idle|ready|working)' 60

echo "4. the tmux server is lost: the workspace is saved, and says it starts a new conversation"
inner kill-server
wait_screen 'SAVED'
wait_screen 'Enter starts a new conversation'

echo "5. restore: Enter starts a new Codex; nothing asks it to resume"
outer send-keys -t s Enter
wait_windows 1
wait_agent 'codex|openai|sign in|log in' 90
case "$(started_with)" in
  */codex) ;;
  */codex\ --dangerously*) fail "restored without asking, which nobody chose" ;;
  *) fail "restored as something else: $(started_with)" ;;
esac
case "$(started_with)" in *resume*) fail "restore asked Codex to resume: $(started_with)" ;; esac

quit_flight
echo "real Codex journey OK"
