<!-- SPDX-License-Identifier: MIT -->

# Running as a service

`flight node run` and `flight orchestrator run` are background infrastructure. Run them from the system's service manager, not from a terminal: they start by themselves, restart if they fail, and have predictable logs. Stopping or restarting them never ends your sessions.

(Plain `flight` on one machine needs none of this.)

## macOS (launchd, per user)

The template is `contrib/launchd/name.flight.node.plist`; the steps, and why it must be launchd and not a terminal, are in `contrib/launchd/README.md`. In short, fill in the three `@...@` places (binary, the node's config directory, a log directory), check it with `plutil -lint`, and load it:

    launchctl bootstrap gui/$(id -u) ~/Library/LaunchAgents/name.flight.node.plist

The template's PATH is `/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin`: it must contain `tmux` and the agent programs you use (`claude`, `codex`). If one lives elsewhere (for example `~/.local/bin`), add that directory to the `PATH` in the plist, or a workspace will say `claude was not found on this node's PATH` (or `codex`).

Check and remove:

    launchctl print gui/$(id -u)/name.flight.node | grep -E 'state|pid|runs'
    launchctl bootout gui/$(id -u)/name.flight.node && rm ~/Library/LaunchAgents/name.flight.node.plist

## Linux (systemd, per user)

    mkdir -p ~/.config/systemd/user
    cp contrib/systemd/flight-node.service ~/.config/systemd/user/
    systemctl --user daemon-reload
    systemctl --user enable --now flight-node
    loginctl enable-linger "$USER"        # run from boot and after logout
    systemctl --user status flight-node
    journalctl --user -u flight-node -f

The unit sets `PATH` to `~/.local/bin:/usr/local/bin:/usr/bin:/bin`: `tmux` and the agent programs (`claude`, `codex`) must be found there, or add their directory to `Environment=PATH=` in the unit. For the orchestrator use `contrib/systemd/flight-orchestrator.service` after editing `--advertise`. The units set `KillMode=process`, so that stopping or restarting the service does not kill the tmux server and your sessions with it.

(Homebrew installs the templates under `$(brew --prefix)/share/flight/contrib`.)
