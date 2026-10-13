// SPDX-License-Identifier: MIT

/// What a new workspace starts. A closed choice, never typed text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Program {
    Claude,
    /// Claude started so that it does not ask before acting. Always spelled out in the form.
    ClaudeSkipPermissions,
    /// Codex (ADR-013). Offered only to a person who chose it in setup.
    Codex,
    /// Codex started with its flag that never asks before acting and does not sandbox. Always
    /// spelled out in the form.
    CodexSkipPermissions,
    /// A workspace made of a shell alone. The form does not offer it (a workspace is made for
    /// an agent, and gets its shell afterwards); the wire still carries it for other callers.
    Shell,
}

impl Program {
    /// What the new-workspace form offers when nothing else was chosen: Claude.
    pub const ALL: [Program; 2] = [Self::Claude, Self::ClaudeSkipPermissions];

    pub fn label(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::ClaudeSkipPermissions => "Claude, no permission prompts",
            Self::Codex => "Codex",
            Self::CodexSkipPermissions => "Codex, no approval prompts",
            Self::Shell => "Shell",
        }
    }

    /// What the form offers for the agents a person works with, in their order of preference
    /// (provider ids as saved by setup). Unknown ids are ignored; nothing recognised means the
    /// default. Each agent comes with its spelled-out "no prompts" variant after it.
    pub fn choices(preference: &[&str]) -> Vec<Program> {
        let mut out = Vec::new();
        for id in preference {
            let pair = match *id {
                "claude" => [Self::Claude, Self::ClaudeSkipPermissions],
                "codex" => [Self::Codex, Self::CodexSkipPermissions],
                _ => continue,
            };
            for p in pair {
                if !out.contains(&p) {
                    out.push(p);
                }
            }
        }
        if out.is_empty() {
            Self::ALL.to_vec()
        } else {
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_is_claude_and_the_preference_orders_the_agents() {
        assert_eq!(Program::choices(&[]), Program::ALL.to_vec());
        assert_eq!(Program::choices(&["gemini"]), Program::ALL.to_vec());
        assert_eq!(
            Program::choices(&["codex", "claude"]),
            vec![
                Program::Codex,
                Program::CodexSkipPermissions,
                Program::Claude,
                Program::ClaudeSkipPermissions
            ]
        );
        assert_eq!(
            Program::choices(&["claude", "claude"]),
            Program::ALL.to_vec(),
            "no duplicates"
        );
    }

    #[test]
    fn a_no_prompts_choice_always_says_so_in_its_label() {
        assert!(Program::ClaudeSkipPermissions
            .label()
            .contains("no permission prompts"));
        assert!(Program::CodexSkipPermissions
            .label()
            .contains("no approval prompts"));
    }
}
