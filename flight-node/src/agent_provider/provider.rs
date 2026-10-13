// SPDX-License-Identifier: MIT

use super::codex;
use crate::agent_resume::{AgentSession, Claude};
use std::path::PathBuf;

/// An agent a node can launch. A closed set: a new provider is an arm in each method here and
/// the equivalence suite of ADR-013, not edits across the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Provider {
    Claude,
    Codex,
}

impl Provider {
    /// The id saved in a workspace definition's surface.
    pub(crate) fn id(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    pub(crate) fn from_id(id: &str) -> Option<Self> {
        match id {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    /// The program looked up on the node's `PATH`.
    pub(crate) fn program(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => codex::PROGRAM,
        }
    }

    /// A new agent: its command line, and the session identity the node chose for it when the
    /// provider lets the node choose one (`None` for a provider that assigns its own).
    pub(crate) fn new_launch(
        self,
        exe: PathBuf,
        skip_permissions: bool,
    ) -> Result<(Vec<String>, Option<AgentSession>), std::io::Error> {
        match self {
            Self::Claude => {
                let session = Claude::new_session()?;
                let argv = Claude::new_argv(exe, &session, skip_permissions);
                Ok((argv, Some(session)))
            }
            Self::Codex => Ok((codex::new_argv(exe, skip_permissions), None)),
        }
    }

    /// The command line that continues `session`, or `None` when this provider cannot continue
    /// a conversation at all.
    pub(crate) fn resume_launch(
        self,
        exe: PathBuf,
        session: &AgentSession,
        skip_permissions: bool,
    ) -> Option<Vec<String>> {
        match self {
            Self::Claude => Some(Claude::resume_argv(exe, session, skip_permissions)),
            Self::Codex => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_unknown_ones_are_not_providers() {
        for p in [Provider::Claude, Provider::Codex] {
            assert_eq!(Provider::from_id(p.id()), Some(p));
        }
        assert_eq!(Provider::from_id("gemini"), None);
        assert_eq!(Provider::from_id(""), None);
    }

    #[test]
    fn only_claude_chooses_an_identity_and_only_claude_continues() {
        let (argv, session) = Provider::Claude
            .new_launch("/bin/claude".into(), false)
            .unwrap();
        assert!(argv.contains(&"--session-id".to_owned()));
        let session = session.unwrap();
        assert!(Provider::Claude
            .resume_launch("/bin/claude".into(), &session, false)
            .is_some());
        let (argv, none) = Provider::Codex
            .new_launch("/bin/codex".into(), false)
            .unwrap();
        assert_eq!(argv, ["/bin/codex"]);
        assert!(none.is_none());
        assert!(Provider::Codex
            .resume_launch("/bin/codex".into(), &session, false)
            .is_none());
        // The two halves of ADR-010 agree: no launch to continue means no support claimed.
        use crate::agent_resume::{resume_support, Support};
        assert!(matches!(
            resume_support(Provider::Codex.id()),
            Support::Unsupported(_)
        ));
        assert_eq!(resume_support(Provider::Claude.id()), Support::Supported);
    }
}
