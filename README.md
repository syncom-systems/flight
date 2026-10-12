<!-- SPDX-License-Identifier: MIT -->

# Flight

Flight keeps your AI coding agents and shells running, and shows you all of them in one place. Start a workspace for a project, close your laptop, come back tomorrow: it is still there. If the machine restarts, Flight starts it again in the same folder and your Claude conversation picks up where it left off.

It works on one machine, or across several machines you own.

## Quick start

1. Install (macOS on Apple silicon, or Linux x86_64):

        brew install syncom-systems/flight/flight

   No Homebrew? See [docs/install.md](docs/install.md) for the download.

2. Start it:

        flight

3. Press `n`, choose the folder of a project, press Create. Your agent opens. That is all there is to set up.

When you want to leave, press `Ctrl-Space`, let go, then `q` to return to the dashboard, and `q` again to quit. Everything keeps running. Run `flight` any day later and it is all there.

**Mac users, one thing first.** macOS uses `Ctrl-Space` to switch keyboard language, so Flight never receives it. Turn that off in System Settings, Keyboard, Keyboard Shortcuts, Input Sources ("Select the previous input source").

## What you will use

| Key | Does |
|---|---|
| `n` | New workspace: pick a machine and a folder |
| Enter | Open the selected workspace (or start it again if it is saved) |
| `a` / `s` | Open its agent / its shell (Flight offers to make a shell) |
| `Ctrl-Space` then `a` / `s` / `q` | Inside a workspace: switch to agent / shell, or go back to the dashboard |
| `/` | Search |
| `?` | Help |
| `q` | Quit (nothing stops) |

A **workspace** is one project on one machine: an agent (Claude) and, if you want, any number of shells in the same folder.

## When things happen

- You close the dashboard, or the terminal: nothing stops. Run `flight` again.
- The machine restarts, or Flight's background session is lost: the workspace shows as **saved**. Press Enter on it and it starts again in the same folder, and Claude continues the earlier conversation when it still exists. Shells start fresh (what was running in them is gone).

More in [docs/concepts.md](docs/concepts.md).

## More machines

Run your agents on another machine and watch them from your laptop: [docs/two-machines.md](docs/two-machines.md).

## Reference

- [Install, upgrade, roll back, uninstall](docs/install.md)
- [Two or more machines](docs/two-machines.md)
- [Running as a service](docs/services.md)
- [How it works: what survives what](docs/concepts.md)
- [Troubleshooting](docs/troubleshooting.md)
- [What is and is not supported in 1.0](https://github.com/syncom-systems/flight/blob/main/docs/releases/v1.0-contract.md), [known limitations](https://github.com/syncom-systems/flight/issues/91)
- Security reports: [SECURITY.md](SECURITY.md)
