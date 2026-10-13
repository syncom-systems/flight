// SPDX-License-Identifier: MIT

use super::{Agent, Machine};
use flight_tmux::TmuxVersion;

/// What `tmux -V` says about this machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxStatus {
    Missing,
    /// Found, but older than Flight supports.
    TooOld(String),
    Ok(String),
}

/// What is present on this machine, found by looking, never by assuming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Findings {
    pub tmux: TmuxStatus,
    pub claude: bool,
    pub codex: bool,
    pub brew: bool,
    pub apt: bool,
    pub sudo: bool,
    pub root: bool,
}

impl Findings {
    pub fn gather(machine: &dyn Machine) -> Self {
        let tmux = match machine.tmux_version() {
            None => TmuxStatus::Missing,
            Some(text) => match TmuxVersion::parse(&text) {
                Some(v) if !v.is_supported() => TmuxStatus::TooOld(v.to_string()),
                Some(v) => TmuxStatus::Ok(v.to_string()),
                // A build that names no version cannot be judged; it is let through.
                None => TmuxStatus::Ok(text.trim().to_owned()),
            },
        };
        Self {
            tmux,
            claude: machine.which(Agent::Claude.program()),
            codex: machine.which(Agent::Codex.program()),
            brew: machine.which("brew"),
            apt: machine.which("apt-get"),
            sudo: machine.which("sudo"),
            root: machine.is_root(),
        }
    }

    pub fn has(&self, agent: Agent) -> bool {
        match agent {
            Agent::Claude => self.claude,
            Agent::Codex => self.codex,
        }
    }

    pub fn tmux_ok(&self) -> bool {
        matches!(self.tmux, TmuxStatus::Ok(_))
    }

    /// Whether anything Flight cannot start without is missing: a usable tmux, or any agent
    /// program at all.
    pub fn blocks_a_first_run(&self) -> bool {
        !self.tmux_ok() || !(self.claude || self.codex)
    }
}
