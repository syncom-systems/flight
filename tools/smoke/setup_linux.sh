#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Guided setup (ADR-012) on a bare Debian or Ubuntu machine with no tmux, run as the
# administrator (a fresh container): nothing is installed without a yes, a no leaves the machine
# as it was, a yes installs tmux through apt and the result is checked by looking. The terminal
# is a pseudo-terminal made by `script`, which is how a person's answers reach setup.
#
#   tools/smoke/setup_linux.sh /path/to/flight
#
# Needs: no tmux on PATH (it refuses to run otherwise: it would prove nothing), `script`.
set -eu

FLIGHT=$(cd "$(dirname "${1:?usage: setup_linux.sh /path/to/flight}")" && pwd)/$(basename "$1")
BASE=$(mktemp -d "${TMPDIR:-/tmp}/flsetup.XXXXXX")
trap 'rm -rf "$BASE"' EXIT
export FLIGHT_CONFIG_DIR="$BASE/cfg"
mkdir -p "$BASE/bin"
printf '#!/bin/sh\nexec sleep 3600\n' > "$BASE/bin/claude"
chmod +x "$BASE/bin/claude"
export PATH="$BASE/bin:$PATH"

fail() { echo "FAIL: $1" >&2; exit 1; }
command -v tmux >/dev/null && fail "tmux is already installed; this check needs a machine without it"
command -v script >/dev/null || fail "script (util-linux) is needed to give setup a terminal"
[ "$(id -u)" = 0 ] || fail "run as root (a fresh container)"

echo "1. --check says what is missing and the commands, asks nothing, runs nothing"
if out=$("$FLIGHT" setup --check 2>&1); then fail "--check succeeded without tmux"; fi
echo "$out" | grep -q 'tmux: not found' || fail "--check does not say tmux is missing: $out"
echo "$out" | grep -q 'apt-get install -y tmux' || fail "--check does not show the command: $out"
command -v tmux >/dev/null && fail "--check installed tmux"

echo "2. without a terminal a plain flight refuses as in 1.x and installs nothing"
if "$FLIGHT" solo </dev/null >"$BASE/solo.out" 2>&1; then fail "solo succeeded without tmux"; fi
grep -q 'tmux is needed' "$BASE/solo.out" || fail "solo's refusal changed: $(cat "$BASE/solo.out")"
command -v tmux >/dev/null && fail "a plain flight installed tmux"

echo "3. on a terminal, no to every question leaves the machine as it was"
printf '1\nn\nn\n' | script -qec "$FLIGHT setup" /dev/null >"$BASE/no.out" 2>&1 || true
grep -q 'command: apt-get install -y tmux' "$BASE/no.out" || fail "the command was not shown: $(cat "$BASE/no.out")"
command -v tmux >/dev/null && fail "tmux was installed after a no"

echo "4. on a terminal, yes to each command installs tmux, and setup checks by looking"
printf '1\ny\ny\n' | script -qec "$FLIGHT setup" /dev/null >"$BASE/yes.out" 2>&1 || fail "setup did not succeed: $(tail -n 30 "$BASE/yes.out")"
grep -q 'Ready.' "$BASE/yes.out" || fail "setup did not say Ready: $(tail -n 30 "$BASE/yes.out")"
command -v tmux >/dev/null || fail "tmux is still not installed"
tmux -V
"$FLIGHT" setup --check >/dev/null || fail "--check still reports something missing"
grep -q 'declined' "$FLIGHT_CONFIG_DIR/settings.toml" || fail "settings were not saved"

echo "setup journey OK"
