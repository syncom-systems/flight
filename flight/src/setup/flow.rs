// SPDX-License-Identifier: MIT

use super::console::Answer;
use super::{plan, Agent, Console, Findings, Machine, Plan, Settings, SettingsStore, TmuxStatus};

/// How setup was reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `flight setup`: the person asked.
    Asked,
    /// A plain `flight` found something missing and offers to help, once.
    Offered,
    /// `flight setup --check`: say what is missing and the commands; run nothing, ask nothing.
    Check,
}

/// How a run of setup ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing Flight needs is missing now.
    Ready,
    /// Something is still missing (declined, failed, or not installable by setup).
    Incomplete,
    /// The offer was declined; it is not repeated.
    Declined,
}

fn describe_tmux(status: &TmuxStatus) -> String {
    match status {
        TmuxStatus::Missing => "tmux: not found (3.3 or newer is needed)".to_owned(),
        TmuxStatus::TooOld(v) => format!("tmux: {v} is older than the 3.3 Flight needs"),
        TmuxStatus::Ok(v) => format!("tmux: {v}"),
    }
}

fn report(console: &mut dyn Console, findings: &Findings) {
    console.say(&describe_tmux(&findings.tmux));
    for agent in Agent::ALL {
        let state = if findings.has(agent) {
            "found"
        } else {
            "not found"
        };
        console.say(&format!("{}: {state}", agent.label()));
    }
}

/// The agents the person wants, asked once; the saved choice is kept when there is one.
fn choose_agents(console: &mut dyn Console, reuse: Option<&Settings>) -> Vec<Agent> {
    if let Some(s) = reuse.filter(|s| s.setup.completed) {
        return s
            .agents
            .enabled
            .iter()
            .filter_map(|id| Agent::from_id(id))
            .collect();
    }
    console.say("");
    console.say("Which agents do you work with?");
    console.say("  1) Claude Code");
    console.say("  2) Codex (preview: starts and persists; conversations are not continued after a restart)");
    console.say("  3) Both");
    console.say("  4) Neither (shells only)");
    match console.line("Choose 1-4 [1]:").as_deref().map(str::trim) {
        Some("2") => vec![Agent::Codex],
        Some("3") => vec![Agent::Claude, Agent::Codex],
        Some("4") => Vec::new(),
        _ => vec![Agent::Claude],
    }
}

/// Run setup against `machine`, talking through `console`, keeping preferences in `store`.
/// Nothing is run without an explicit yes for that command.
pub fn run(
    mode: Mode,
    machine: &mut dyn Machine,
    console: &mut dyn Console,
    store: &SettingsStore,
) -> Outcome {
    let saved = match store.load() {
        Ok(s) => s,
        Err(e) => {
            console.say(&format!("Note: {e}. Your choices will not be saved."));
            None
        }
    };
    let can_save = store.load().is_ok();
    let findings = Findings::gather(machine);

    if mode == Mode::Check {
        report(console, &findings);
        let wanted = saved
            .as_ref()
            .map(|s| {
                s.agents
                    .enabled
                    .iter()
                    .filter_map(|id| Agent::from_id(id))
                    .collect()
            })
            .unwrap_or_else(|| vec![Agent::Claude]);
        let Plan { steps, notes } = plan(&findings, machine.platform(), &wanted);
        for s in &steps {
            console.say(&format!("To {}: {}", s.what, s.command_line()));
        }
        for n in &notes {
            console.say(n);
        }
        let missing = !findings.tmux_ok() || wanted.iter().any(|a| !findings.has(*a));
        return if missing {
            Outcome::Incomplete
        } else {
            Outcome::Ready
        };
    }

    if mode == Mode::Offered {
        console.say("Flight needs something that is not on this machine yet:");
        report(console, &findings);
        if console
            .confirm("Set it up now? (each command is shown first and runs only if you say yes)")
            == Answer::No
        {
            console.say(
                "Skipped. Run `flight setup` any time, or see docs/install.md to do it by hand.",
            );
            remember(store, can_save, saved, |s| s.setup.declined = true);
            return Outcome::Declined;
        }
    } else {
        report(console, &findings);
    }

    // A person who asked for setup is asked again; one who was only offered it keeps what
    // they chose before.
    let reuse = if mode == Mode::Offered {
        saved.as_ref()
    } else {
        None
    };
    let wanted = choose_agents(console, reuse);
    let Plan { steps, notes } = plan(&findings, machine.platform(), &wanted);
    if steps.is_empty() && notes.is_empty() {
        console.say("Everything Flight needs is installed.");
    }
    for step in &steps {
        console.say("");
        console.say(&format!("Next: {}", step.what));
        console.say(&format!("  command: {}", step.command_line()));
        if console.confirm("Run it?") == Answer::No {
            console.say("Not run.");
            continue;
        }
        match machine.run(step) {
            Ok(0) => console.say("Done."),
            Ok(code) => console.say(&format!(
                "That command failed (exit status {code}). Nothing else was changed and it is not retried."
            )),
            Err(why) => console.say(&format!("That command could not run: {why}")),
        }
    }
    if !notes.is_empty() {
        console.say("");
        for n in &notes {
            console.say(n);
        }
    }

    let after = Findings::gather(machine);
    console.say("");
    report(console, &after);
    let ready = after.tmux_ok() && wanted.iter().all(|a| after.has(*a));
    remember(store, can_save, saved, |s| {
        s.setup.completed = true;
        s.setup.declined = false;
        s.agents.enabled = wanted.iter().map(|a| a.id().to_owned()).collect();
        s.agents.default = wanted
            .first()
            .map(|a| a.id().to_owned())
            .unwrap_or_default();
    });
    if ready {
        console.say("Ready. Run `flight` to start.");
        Outcome::Ready
    } else {
        console.say("Not everything is in place yet; the lines above say what is missing.");
        Outcome::Incomplete
    }
}

fn remember(
    store: &SettingsStore,
    can_save: bool,
    saved: Option<Settings>,
    change: impl FnOnce(&mut Settings),
) {
    if !can_save {
        return;
    }
    let mut settings = saved.unwrap_or_default();
    change(&mut settings);
    let _ = store.save(&settings);
}
