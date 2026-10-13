// SPDX-License-Identifier: MIT

use crate::agent_provider::Provider;

/// What a new session runs: a closed set, never a command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    /// The `claude` found on the node's own `PATH`.
    Claude,
    /// The node's normal shell: tmux's `default-shell`, started the way tmux starts it for a
    /// session created by hand.
    Shell,
    /// `claude`, started so that it does not ask before acting.
    ClaudeSkipPermissions,
    /// The `codex` found on the node's own `PATH`.
    Codex,
    /// `codex`, started with its flag that never asks and does not sandbox.
    CodexSkipPermissions,
}

impl Program {
    /// The agent this starts and whether it is started without asking; `None` for a shell.
    pub(crate) fn agent(self) -> Option<(Provider, bool)> {
        match self {
            Self::Shell => None,
            Self::Claude => Some((Provider::Claude, false)),
            Self::ClaudeSkipPermissions => Some((Provider::Claude, true)),
            Self::Codex => Some((Provider::Codex, false)),
            Self::CodexSkipPermissions => Some((Provider::Codex, true)),
        }
    }

    /// The program for an agent and its permission choice.
    pub(crate) fn of_agent(provider: Provider, skip_permissions: bool) -> Self {
        match (provider, skip_permissions) {
            (Provider::Claude, false) => Self::Claude,
            (Provider::Claude, true) => Self::ClaudeSkipPermissions,
            (Provider::Codex, false) => Self::Codex,
            (Provider::Codex, true) => Self::CodexSkipPermissions,
        }
    }
}

/// A validated request to create a session, detached from the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRequest {
    pub name: String,
    /// Absolute, or `~`-relative to the node's home directory.
    pub dir: String,
    pub program: Program,
}
