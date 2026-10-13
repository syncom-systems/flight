// SPDX-License-Identifier: MIT

//! Guided setup (ADR-012): find what Flight needs on this machine and, only with an explicit yes
//! for each command, install it. The 1.x non-interactive behaviour is unchanged: without a
//! terminal, with `--no-setup` or `FLIGHT_NO_SETUP=1`, nothing is asked and nothing is run.

mod agent;
mod console;
mod finding;
mod flow;
mod machine;
mod plan;
mod platform;
mod settings;
mod step;

#[cfg(test)]
mod flow_tests;

use crate::args::{config_dir, Args};
use agent::Agent;
use console::{Console, TerminalConsole};
use finding::{Findings, TmuxStatus};
use flow::{run as run_flow, Mode, Outcome};
use machine::{Machine, SystemMachine};
use plan::{plan, Plan};
use platform::Platform;
use settings::{Settings, SettingsStore};
use std::io::IsTerminal;
use std::path::Path;
use step::Step;

pub const USAGE: &str = "usage: flight setup [--check] [--config-dir DIR]

  Looks at what Flight needs on this machine (tmux 3.3 or newer, an agent program such as
  Claude Code) and offers to install what is missing. Every command is shown first and runs
  only if you answer yes; nothing is installed otherwise. Homebrew itself is never installed.
  It also asks which agents you work with and remembers the answer.

  --check   only say what is missing and the commands that would fix it; run nothing, ask
            nothing; exits with an error when something is missing.

  A plain `flight` offers setup once when tmux or every agent program is missing and it is
  talking to a terminal; --no-setup or FLIGHT_NO_SETUP=1 turns that off.";

/// `flight setup`.
pub fn run(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return Ok(());
    }
    let parsed = Args::parse(args, &["--config-dir"], &["--check"])?;
    let dir = config_dir(&parsed)?;
    let store = SettingsStore::in_dir(&dir);
    let mode = if parsed.switch("--check") {
        Mode::Check
    } else {
        if !interactive() {
            return Err(
                "setup asks questions and needs a terminal; use `flight setup --check` to see \
                 what is missing and the commands to run yourself"
                    .to_owned(),
            );
        }
        Mode::Asked
    };
    match run_flow(mode, &mut SystemMachine, &mut TerminalConsole, &store) {
        Outcome::Ready => Ok(()),
        Outcome::Incomplete | Outcome::Declined => {
            Err("something Flight needs is still missing".to_owned())
        }
    }
}

/// Called by a plain `flight` before it starts: offer setup once, if it is useful and welcome.
/// Never errors: whatever happens, the caller goes on to its own checks, which refuse clearly
/// when tmux is still missing.
pub fn offer_before_start(config_dir: &Path, args: &Args) {
    if args.switch("--no-setup") || std::env::var_os("FLIGHT_NO_SETUP").is_some() || !interactive()
    {
        return;
    }
    let store = SettingsStore::in_dir(config_dir);
    if store
        .load()
        .ok()
        .flatten()
        .is_some_and(|s| s.setup.declined)
    {
        return;
    }
    let machine = SystemMachine;
    if !Findings::gather(&machine).blocks_a_first_run() {
        return;
    }
    run_flow(
        Mode::Offered,
        &mut SystemMachine,
        &mut TerminalConsole,
        &store,
    );
}

fn interactive() -> bool {
    std::io::stdin().is_terminal() && std::io::stdout().is_terminal()
}

/// The agents the person works with, as saved by setup (provider ids, in their order of
/// preference). Nothing saved, or a file that cannot be used, means none (the form then offers
/// Claude): the file is never touched here.
pub fn chosen_agents(config_dir: &Path) -> Vec<String> {
    SettingsStore::in_dir(config_dir)
        .load()
        .ok()
        .flatten()
        .map(|s| s.agents.enabled)
        .unwrap_or_default()
}
