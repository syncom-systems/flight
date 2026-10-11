<!-- SPDX-License-Identifier: MIT -->

# Installing Flight

Supported platforms are listed in `docs/releases/v1.0-contract.md`: macOS on Apple silicon and Linux on x86_64 with glibc 2.36 or newer and tmux 3.3 or newer (Debian 12, Ubuntu 24.04). Anything else is unsupported; in particular Ubuntu 22.04 and Debian 11 ship a tmux that is too old.

## Requirements

- `tmux` 3.3 or newer on every machine that runs Flight (`tmux -V`). Flight uses its own private tmux server (`tmux -L flight`) and never touches yours.
- For distributed mode: a TCP route from each node and dashboard machine to the orchestrator (default port 7676). Nothing else: no SSH, no inbound port on nodes.

macOS: `brew install tmux`. Debian/Ubuntu: `sudo apt install tmux`.

## Install from a release

Download the tarball for your platform and `SHA256SUMS` from the release page, then verify both the checksum and the build provenance before running anything:

    sha256sum -c SHA256SUMS --ignore-missing        # macOS: shasum -a 256 -c SHA256SUMS --ignore-missing
    gh attestation verify flight-<version>-<target>.tar.gz --repo syncom-systems/flight

`<target>` is `aarch64-apple-darwin` or `x86_64-unknown-linux-gnu`. The attestation proves the tarball was built by this repository's release workflow from the tagged commit.

    tar -xzf flight-<version>-<target>.tar.gz
    mkdir -p ~/.local/bin
    install -m 755 flight-<version>-<target>/flight ~/.local/bin/flight
    flight --version                                # flight <version> (<commit>)

Make sure `~/.local/bin` is on your `PATH`. On macOS a binary downloaded with a browser may carry a quarantine flag; `curl`/`gh release download` do not set it, and `xattr -d com.apple.quarantine ~/.local/bin/flight` clears it if it does.

## Single machine (the common case)

    flight

The first run sets everything up (about a second) and shows the dashboard; quitting stops what it started, and your sessions keep running. See `README.md` for the keys.

## Where things live

Everything is under `~/.config/flight` (or `$XDG_CONFIG_HOME/flight`, or the directory in `$FLIGHT_CONFIG_DIR`, or `--config-dir`), one directory per role: `orchestrator/`, `node/`, `ui/`. Each holds that role's identity and trust settings; `node/` also holds `workspaces.toml` (your saved workspaces, with `workspaces.prev.toml` and your snapshots) and `resume.toml` (agent conversation references), and `ui/` gets `layouts.toml` once you arrange a presentation. Files with secrets are mode 0600. Logs of the single-machine mode are in `solo.log` next to them. Sessions live in the private tmux server `flight`, not in these files.

## Distributed mode

One machine runs the orchestrator, every machine with agents runs a node, any machine can show the dashboard.

    # orchestrator machine
    flight orchestrator run --listen 0.0.0.0:7676 --advertise <address nodes will use>:7676
    flight orchestrator enrollment create           # one-time bundle, expires (default shortly)

    # each node machine (use the bundle once; it is useless afterwards)
    flight node join '<bundle>' --name <node-name>
    flight node run

    # each dashboard machine
    flight orchestrator enrollment create           # a fresh bundle, on the orchestrator
    flight ui join '<bundle>' --name <ui-name>
    flight ui

    flight orchestrator trust list                  # who is trusted
    flight orchestrator trust revoke <id>           # drop a machine at once

Trust is established by the bundle: it carries the orchestrator's key fingerprint, which the joining machine pins, and a single-use token, which the orchestrator accepts once. Every later connection is mutual TLS 1.3 against pinned fingerprints.

## Running as a service

`flight node run` and `flight orchestrator run` are infrastructure: run them from the service manager, not from a shell or tmux (`contrib/launchd/README.md` explains why).

macOS (launchd, per-user): follow `contrib/launchd/README.md`; the template is `contrib/launchd/name.flight.node.plist`.

Linux (systemd, per-user):

    mkdir -p ~/.config/systemd/user
    cp contrib/systemd/flight-node.service ~/.config/systemd/user/
    systemctl --user daemon-reload
    systemctl --user enable --now flight-node
    loginctl enable-linger "$USER"                  # run from boot and after logout
    systemctl --user status flight-node
    journalctl --user -u flight-node -f

The unit sets `PATH` for the node (`~/.local/bin:/usr/local/bin:/usr/bin:/bin`): `tmux` and the agent program (`claude`) must be found there, or add their directory to `Environment=PATH=` in the unit; a workspace that cannot start its agent says `claude was not found on this node's PATH`. For the orchestrator use `contrib/systemd/flight-orchestrator.service` after editing `--advertise`. The units set `KillMode=process` so that stopping or restarting the service does not kill the tmux server and your sessions with it.

## Backup

Stop nothing; copy the configuration directory (it is a handful of small files):

    cp -R ~/.config/flight ~/flight-config-backup-$(date +%Y%m%d)

`flight workspaces snapshot save <label>` keeps a named copy of the saved workspaces inside the node directory; `flight workspaces undo` takes back the last change.

## Upgrade

1. Back up (above).
2. Verify the new tarball as in "Install from a release".
3. Stop the services (`systemctl --user stop flight-node`, `launchctl bootout gui/$(id -u)/name.flight.node`); a single-machine `flight` is stopped by quitting it. Your tmux sessions keep running.
4. Replace `~/.local/bin/flight` with the new binary (keep the old one as `flight.prev`).
5. Start the services; `flight --version` confirms the version.

If the saved workspaces use an older schema, the new version upgrades the file in place and keeps the original as `workspaces.v<N>.bak`. Within one major version, peers negotiate capabilities, so nodes, orchestrator and dashboards may be upgraded one at a time.

## Roll back

Stop the services, put `flight.prev` back, start them. A newer file format is never read by an older Flight: it reports that the saved workspaces are newer and leaves the file alone. In that case restore the pre-upgrade backup (or `workspaces.v<N>.bak`) over `node/workspaces.toml` first; anything saved since the upgrade is then lost, which is why step 1 exists.

## Uninstall

    systemctl --user disable --now flight-node       # or: launchctl bootout ...; remove the plist
    rm ~/.config/systemd/user/flight-node.service
    rm ~/.local/bin/flight

Your configuration (`~/.config/flight`: identities, trust, saved workspaces) is **kept**; delete it yourself to forget everything, or keep it and a reinstall finds your workspaces again. Running sessions are kept too: they live in the private tmux server, which you end explicitly (this closes the sessions) with `tmux -L flight kill-server`.

## Troubleshooting

- `flight: ... tmux` errors: `tmux -V` must work and be 3.3 or newer on that machine.
- The single-machine mode did not come up: read `~/.config/flight/solo.log`.
- `path must be shorter than SUN_LEN`: the configuration directory's path is too long for a Unix socket (about 90 characters at most); use a shorter `--config-dir`.
- A node says it has not joined: run `flight node join` (the bundle is single-use and short-lived; make a fresh one).
- A node shows Disconnected: `flight orchestrator node list`; check the node's log (`journalctl --user -u flight-node`, or the launchd log) and that the address in `--advertise` is reachable.
- "saved workspaces ... newer than this Flight" or "unknown field": the file is left untouched; use the Flight that wrote it, or restore a backup.
- Report a problem with `flight --version`, the OS, `tmux -V` and the relevant log.
