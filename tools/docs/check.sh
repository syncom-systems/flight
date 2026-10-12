#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# Keeps the user documentation honest: every relative link resolves, and every `flight ...`
# command the guides show names a command (and subcommand) the binary really has.
#
#   tools/docs/check.sh /path/to/flight
set -eu

FLIGHT=${1:?usage: check.sh /path/to/flight}
cd "$(dirname "$0")/../.."
FILES="README.md SECURITY.md docs/install.md docs/two-machines.md docs/services.md docs/concepts.md docs/troubleshooting.md"
bad=0

echo "links"
for f in $FILES; do
  dir=$(dirname "$f")
  grep -oE '\]\([^)]+\)' "$f" | sed -E 's/^\]\((.*)\)$/\1/' | while read -r link; do
    case "$link" in http*|\#*|mailto:*) continue ;; esac
    target=${link%%#*}
    [ -z "$target" ] && continue
    [ -e "$dir/$target" ] || { echo "  $f: broken link $link"; echo x >> "${TMPDIR:-/tmp}/docs-check.$$"; }
  done
done
[ ! -e "${TMPDIR:-/tmp}/docs-check.$$" ] || { rm -f "${TMPDIR:-/tmp}/docs-check.$$"; bad=1; }

echo "commands"
for f in $FILES; do
  # Indented code lines (four spaces) that start with an optional "A$ "/"B$ " and `flight`.
  grep -E '^    ([AB]\$ )?flight ' "$f" | sed -E 's/^    ([AB]\$ )?//' | while read -r _ group sub rest; do
    case "$group" in
      --version|-V|--once) continue ;;
      orchestrator|node|ui|workspaces|solo) ;;
      *) echo "  $f: unknown command 'flight $group'"; echo x >> "${TMPDIR:-/tmp}/docs-check.$$"; continue ;;
    esac
    help=$("$FLIGHT" "$group" --help 2>&1 || true)
    case "$sub" in ''|--*|-*|"<"*|"'"*) continue ;; esac
    if [ "$sub" = enrollment ]; then sub="enrollment ${rest%% *}"; fi
    echo "$help" | grep -qF -- "$sub" || { echo "  $f: 'flight $group $sub' is not in 'flight $group --help'"; echo x >> "${TMPDIR:-/tmp}/docs-check.$$"; }
  done
done
[ ! -e "${TMPDIR:-/tmp}/docs-check.$$" ] || { rm -f "${TMPDIR:-/tmp}/docs-check.$$"; bad=1; }

[ "$bad" = 0 ] && echo "docs OK" || { echo "docs check FAILED"; exit 1; }
