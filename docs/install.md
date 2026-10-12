<!-- SPDX-License-Identifier: MIT -->

# Installing Flight

Supported: macOS on Apple silicon, and Linux on x86_64 with glibc 2.36 or newer (Debian 12, Ubuntu 24.04). Everything else is unsupported ([contract](https://github.com/syncom-systems/flight/blob/main/docs/releases/v1.0-contract.md)); in particular Ubuntu 22.04 and Debian 11 ship a tmux that is too old.

Flight needs **tmux 3.3 or newer** on every machine that runs it (`tmux -V`). It uses its own private tmux server and never touches yours.

## Homebrew (macOS and Linux)

    brew install syncom-systems/flight/flight
    flight --version

This installs tmux too. Upgrade with `brew upgrade flight`.

## From a release download

Use this if you do not use Homebrew. First install tmux 3.3 or newer (`brew install tmux`, or `sudo apt install tmux` on Debian 12 / Ubuntu 24.04).

Download the tarball for your machine and `SHA256SUMS` from the [release page](https://github.com/syncom-systems/flight/releases), then verify both the checksum and the build provenance before running anything:

    sha256sum -c SHA256SUMS --ignore-missing        # macOS: shasum -a 256 -c SHA256SUMS --ignore-missing
    gh attestation verify flight-<version>-<target>.tar.gz --repo syncom-systems/flight

`<target>` is `aarch64-apple-darwin` or `x86_64-unknown-linux-gnu`. The attestation proves the tarball was built by this repository's release workflow from the tagged commit.

    tar -xzf flight-<version>-<target>.tar.gz
    mkdir -p ~/.local/bin
    install -m 755 flight-<version>-<target>/flight ~/.local/bin/flight
    flight --version                                # flight <version> (<commit>)

Make sure `~/.local/bin` is on your `PATH`. On macOS a binary downloaded with a browser may carry a quarantine flag; `curl` and `gh release download` do not set it, and `xattr -d com.apple.quarantine ~/.local/bin/flight` clears it if it does.

## First run

    flight

The first run sets everything up in about a second and shows the dashboard. See the [README](../README.md) for the keys. On macOS, first turn off the system's use of `Ctrl-Space` ([troubleshooting](troubleshooting.md)).

You also need the agent program you want to use, for Claude the `claude` command, installed and logged in on each machine where workspaces run.

## More machines, services

[two-machines.md](two-machines.md), [services.md](services.md).

## Where things live

Everything is under `~/.config/flight` (or `$XDG_CONFIG_HOME/flight`, or the directory in `$FLIGHT_CONFIG_DIR`, or `--config-dir`), one directory per role: `orchestrator/`, `node/`, `ui/`. Each holds that role's identity and trust settings; `node/` also holds `workspaces.toml` (your saved workspaces, with `workspaces.prev.toml` and your snapshots) and `resume.toml` (agent conversation references), and `ui/` gets `layouts.toml` once you arrange a presentation. Files with secrets are mode 0600. Logs of the single-machine mode are in `solo.log` next to them. Sessions live in the private tmux server `flight`, not in these files.

## Backup

Stop nothing; copy the configuration directory (a handful of small files):

    cp -R ~/.config/flight ~/flight-config-backup-$(date +%Y%m%d)

`flight workspaces snapshot save <label>` keeps a named copy of the saved workspaces inside the node directory; `flight workspaces undo` takes back the last change.

## Upgrade

1. Back up (above).
2. Homebrew: `brew upgrade flight`. Otherwise verify the new tarball as above and replace `~/.local/bin/flight` (keep the old one as `flight.prev`).
3. If Flight runs as a service, restart it (`systemctl --user restart flight-node`, or `launchctl kickstart -k gui/$(id -u)/name.flight.node`). Your tmux sessions keep running. A single-machine `flight` is restarted by quitting and starting it.
4. `flight --version` confirms the version.

If the saved workspaces use an older schema, the new version upgrades the file in place and keeps the original as `workspaces.v<N>.bak`. Within one major version, peers negotiate capabilities, so nodes, orchestrator and dashboards may be upgraded one at a time.

## Roll back

Stop the services, put `flight.prev` back, start them. A newer file format is never read by an older Flight: it reports that the saved workspaces are newer and leaves the file alone. In that case restore the pre-upgrade backup (or `workspaces.v<N>.bak`) over `node/workspaces.toml` first; anything saved since the upgrade is then lost, which is why step 1 exists.

## Uninstall

    brew uninstall flight                            # or: rm ~/.local/bin/flight
    # services, if you made them: see services.md

Your configuration (`~/.config/flight`: identities, trust, saved workspaces) is **kept**; delete it yourself to forget everything, or keep it and a reinstall finds your workspaces again. Running sessions are kept too: they live in the private tmux server, which you end explicitly (this closes the sessions) with `tmux -L flight kill-server`.
