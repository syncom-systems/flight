#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
#
# The Homebrew formula against a release package, before the release is published. The tap's
# current formula is taken as it is, only its download URLs and checksums are pointed at the local
# package; it is then installed, audited-for-install, tested, and the files the documentation
# promises (the guides, the launchd and systemd templates under share/flight/contrib) are
# checked in the installed tree. A change to the layout of the archive that the formula does not
# follow fails here instead of silently breaking `brew install`.
#
#   tools/release/check-homebrew.sh /path/to/flight-<ver>-<target>.tar.gz
#
# Needs Homebrew. It installs into Homebrew's prefix (a CI machine, not a workstation).
set -eu

TARBALL=$(cd "$(dirname "${1:?usage: check-homebrew.sh /path/to/flight-<ver>-<target>.tar.gz}")" && pwd)/$(basename "$1")
REPO=$(cd "$(dirname "$0")/../.." && pwd)
WORK=$(mktemp -d "${TMPDIR:-/tmp}/flbrew.XXXXXX")
trap 'rm -rf "$WORK"' EXIT
command -v brew >/dev/null || { echo "Homebrew is required" >&2; exit 2; }

FORMULA_URL=${FORMULA_URL:-https://raw.githubusercontent.com/syncom-systems/homebrew-flight/main/Formula/flight.rb}
echo "1. the tap's formula"
curl -fsSL "$FORMULA_URL" -o "$WORK/flight.rb"

echo "2. pointed at this package (url and sha256 of this target only)"
python3 - "$WORK/flight.rb" "$TARBALL" <<'PY'
import hashlib, re, sys
path, tarball = sys.argv[1], sys.argv[2]
name = tarball.rsplit('/', 1)[1]
target = re.match(r'flight-[^-]+(?:-rc\.\d+)?-(.+)\.tar\.gz$', name).group(1)
sha = hashlib.sha256(open(tarball, 'rb').read()).hexdigest()
src = open(path).read()
pat = re.compile(r'(url ")([^"]*-' + re.escape(target) + r'\.tar\.gz)(")(\s*\n\s*sha256 ")([0-9a-f]{64})(")')
new, n = pat.subn(lambda m: m.group(1) + 'file://' + tarball + m.group(3) + m.group(4) + sha + m.group(6), src)
if n != 1:
    sys.exit(f"the formula has no url/sha256 pair for {target} ({n} matches)")
open(path, 'w').write(new)
PY

echo "3. install it from a local tap"
brew tap-new --no-git local/flightcheck >/dev/null
cp "$WORK/flight.rb" "$(brew --repository local/flightcheck)/Formula/flight.rb"
brew install --formula local/flightcheck/flight
brew test local/flightcheck/flight

echo "4. the installed tree holds what the documentation points at"
prefix=$(brew --prefix local/flightcheck/flight)
fail() { echo "FAIL: $1" >&2; exit 1; }
want_version=$(basename "$TARBALL" | sed -E 's/^flight-(.+)-(aarch64-apple-darwin|x86_64-unknown-linux-gnu)\.tar\.gz$/\1/')
"$prefix/bin/flight" --version | grep -q "^flight $want_version " || fail "version is not $want_version"
for f in README.md install.md two-machines.md services.md concepts.md troubleshooting.md; do
  [ -f "$prefix/share/doc/flight/$f" ] || fail "documentation $f is not installed under share/doc/flight"
done
for f in launchd/name.flight.node.plist launchd/README.md systemd/flight-node.service systemd/flight-orchestrator.service; do
  [ -f "$prefix/share/flight/contrib/$f" ] || fail "service template $f is not installed under share/flight/contrib"
done
# Every guide of the package is installed (a guide added to the archive must reach Homebrew users).
tmp="$WORK/unpacked"; mkdir "$tmp"; tar -xzf "$TARBALL" -C "$tmp"
for f in "$tmp"/*/docs/*.md; do
  [ -f "$prefix/share/doc/flight/$(basename "$f")" ] || fail "$(basename "$f") is in the archive but not installed"
done
[ -x "$prefix/bin/tmux" ] || command -v tmux >/dev/null || fail "tmux was not installed as a dependency"

echo "5. the repository's docs/services.md names the template location the formula installs"
grep -q 'share/flight/contrib' "$REPO/docs/services.md" || fail "docs/services.md no longer names share/flight/contrib"

brew uninstall --formula local/flightcheck/flight >/dev/null 2>&1 || true
brew untap local/flightcheck >/dev/null 2>&1 || true
echo "homebrew formula OK"
