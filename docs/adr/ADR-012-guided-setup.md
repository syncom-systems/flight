<!-- SPDX-License-Identifier: MIT -->

# ADR-012: Guided setup

Status: accepted for v1.1.0 (#119). Scope: finding out what Flight needs on this machine, and installing it **with the person's explicit consent**, in a terminal, before the dashboard. Not in scope: a graphical installer or signed application (#121), installing Homebrew, agent accounts and logins, anything on a remote node.

## The problem

A first-time user who lacks tmux (3.3 or newer) or an agent program hits an error and a command to type. The product should offer the fix, without taking away the ability to do it by hand, and without Flight ever changing the machine on its own.

## Decisions

1. **Consent is per action and explicit.** Setup prints what it found, then for each missing piece the **exact command** it would run (argument by argument, no shell), and asks `Run it? [y/N]`. Anything but `y` runs nothing. There is no `--yes`: scripted installs use the package manager directly, which the documentation shows. The default answer is no.
2. **Only fixed commands.** The set of commands is a table in the code (a package manager, a verb, a package name); nothing is built from user input, from the network, or from files. No shell is involved. `sudo` appears only where the step says so on screen, and is run by the person's own terminal, which asks for their password; Flight never sees it.
3. **Verified methods only.** A method is in the table only if it was checked (`brew info`, the distribution's package list): tmux via `brew install tmux` (macOS, Linux with Homebrew) or `sudo apt-get install -y tmux` (Debian 12, Ubuntu 24.04 families); Claude Code via `brew install --cask claude-code` (macOS); Codex via `brew install --cask codex` (macOS). Where no verified method exists for the platform, setup says so and points to the documentation. **Homebrew itself is never installed** (it needs an administrator password and a downloaded script); setup names the official instructions and stops there.
4. **After each action, check again.** Success is "the program is now found and, for tmux, at least 3.3", never "the command exited 0". A failure shows the command's own output and the exit status, installs nothing else, and does not retry.
5. **Where it runs.** `flight setup` always; and plain `flight` (single-machine mode) offers it, once, when something is missing and both standard input and output are a terminal. Declining is remembered (`flight setup` still works). **The non-interactive path is unchanged:** with no terminal, with `--no-setup`, or with `FLIGHT_NO_SETUP=1`, Flight behaves exactly as in 1.0.x: a missing or old tmux is refused with the message that names the fix, and nothing is prompted or run. `flight setup --check` prints the same findings and the commands, runs nothing, and exits non-zero when something is missing (for scripts and for humans who prefer to run the commands themselves).
6. **Preferences.** Setup asks which agents the person works with and which is the default for a new workspace. That is the only thing it records: `settings.toml` in the configuration directory (mode 0600), `version = 1`, written atomically; a newer version or an unknown key is refused and left untouched, as for every persisted file (ADR-008). Nothing in it is a credential. The new-workspace form (ADR-013) reads it.
7. **Nodes only.** Setup acts on the machine it runs on. For other machines the dashboard only reports a missing program (`claude was not found on this node's PATH`, with the fix); it never installs on a remote node.

## Structure (testable without touching the machine)

`flight::setup`: `Findings` (pure: what is present, what is missing), `Plan` (the table lookup: findings and platform to steps), `Step` (program, argv, why), a `Machine` trait (which, platform, version of tmux, run) and a `Console` trait (print, ask). The real implementations are thin; tests drive the whole flow with fakes: nothing runs without `y`, `n` and end-of-input run nothing, a failed command is reported and not retried, success is judged by the re-check, no terminal means no prompt, and the settings file round-trips and refuses newer versions.

## Consequences

- A first run on a bare Mac with Homebrew: `flight`, answer `y` to tmux and (if wanted) Claude, and the dashboard starts. No command typed, and every action was shown first.
- Without Homebrew, the person gets the instructions and the check, not an install.
- Linux needs the person's `sudo` password for apt, in their own terminal.
- The cost is a small module and its tests; the 1.x non-interactive guarantees are unchanged.
