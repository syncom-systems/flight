<!-- SPDX-License-Identifier: MIT -->

# Troubleshooting

Find the symptom.

## `Ctrl-Space` does nothing inside a workspace (macOS)

macOS uses `Ctrl-Space` to switch input source and the terminal never sees it. Turn it off: System Settings, Keyboard, Keyboard Shortcuts, Input Sources, untick "Select the previous input source" (and the next one if it also uses `Ctrl-Space`). Then press `Ctrl-Space`, let go, then the next key; it is two presses, not a chord.

## "tmux is needed" or "tmux … is older than the 3.3 Flight needs"

Install or update tmux: `brew install tmux` (`brew upgrade tmux`), or `sudo apt install tmux` on Debian 12 / Ubuntu 24.04. Ubuntu 22.04 and Debian 11 ship a tmux that is too old and are not supported. Check with `tmux -V`.

## "claude was not found on this node's PATH"

The machine where the workspace runs does not have `claude`, or the program that runs the node cannot see it. Install it on that machine. If Flight runs there as a service, the service has its own short PATH: add the directory holding `claude` ([services.md](services.md)).

## A workspace shows as "saved" and is not running

Its processes are gone (the machine restarted, or the private tmux server was lost) but Flight remembers it. Select it and press Enter: it starts again in the same folder, and Claude continues the earlier conversation if it still exists. `r` only checks again; it starts nothing. If the folder is missing or changed, Flight refuses and says why rather than starting somewhere else.

## I see workspaces I do not recognise, or a different setup than I expected

A bare `flight` uses your default configuration (`~/.config/flight`) and the private tmux server named `flight`. If you set up a trial elsewhere, point at it: `FLIGHT_CONFIG_DIR=<short folder> flight solo --socket <name>`.

## The selected row is hard to read (Apple Terminal)

The selected row can render as dark grey on black. Known issue, [#111](https://github.com/syncom-systems/flight/issues/111). Another terminal profile or terminal app is a workaround.

## The single-machine mode did not start

Read `solo.log` in the configuration folder (`~/.config/flight/solo.log`). Common causes: another Flight already holds the port (`Address already in use`: quit the other one), or a configuration folder whose path is too long for a Unix socket (`path must be shorter than SUN_LEN`, about 90 characters: use a shorter `--config-dir`).

## A node says it has not joined

Run `flight node join` with a fresh bundle: a bundle works once and expires after a few minutes. `enrollment refused` means it was used or expired.

## A node shows Disconnected

`flight orchestrator node list` on the orchestrator machine. Check the node's log (`journalctl --user -u flight-node`, or the launchd log), that the node can reach the orchestrator's address and port (`--advertise` must be an address the node can reach), and that nothing else runs the same node identity (two nodes with one identity supersede each other).

## "saved workspaces … newer than this Flight" or "unknown field"

The file was written by a newer Flight, and an older one never reads it or changes it. Use the Flight that wrote it, or restore a backup (`workspaces.v<N>.bak`, [install.md](install.md#roll-back)).

## Reporting a problem

Include `flight --version`, your OS, `tmux -V` and the relevant log, at https://github.com/syncom-systems/flight/issues. For a security problem, see [SECURITY.md](../SECURITY.md) instead.
