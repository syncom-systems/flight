// SPDX-License-Identifier: MIT

use super::{Agent, Findings, Platform, Step, TmuxStatus};

/// What setup proposes, from what is missing and what this machine can do about it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Plan {
    /// Commands that may be run, each only after a yes.
    pub steps: Vec<Step>,
    /// Things setup cannot do for the person, with where to look instead.
    pub notes: Vec<String>,
}

const BREW_TMUX: Step = Step {
    what: "install tmux with Homebrew",
    argv: &["brew", "install", "tmux"],
};
const BREW_UPGRADE_TMUX: Step = Step {
    what: "update tmux with Homebrew",
    argv: &["brew", "upgrade", "tmux"],
};
const APT_UPDATE_SUDO: Step = Step {
    what: "refresh apt's package lists (asks for your password through sudo)",
    argv: &["sudo", "apt-get", "update"],
};
const APT_TMUX_SUDO: Step = Step {
    what: "install tmux with apt (asks for your password through sudo)",
    argv: &["sudo", "apt-get", "install", "-y", "tmux"],
};
const APT_UPDATE_ROOT: Step = Step {
    what: "refresh apt's package lists",
    argv: &["apt-get", "update"],
};
const APT_TMUX_ROOT: Step = Step {
    what: "install tmux with apt",
    argv: &["apt-get", "install", "-y", "tmux"],
};
const BREW_CLAUDE: Step = Step {
    what: "install Claude Code with Homebrew",
    argv: &["brew", "install", "--cask", "claude-code"],
};
const BREW_CODEX: Step = Step {
    what: "install Codex with Homebrew",
    argv: &["brew", "install", "--cask", "codex"],
};

const HOMEBREW_NOTE: &str =
    "Homebrew is not installed. Setup never installs it; the official instructions are at https://brew.sh";

/// Only methods that were checked are listed (ADR-012). An empty plan with no notes means
/// there is nothing to do.
pub fn plan(findings: &Findings, platform: Platform, wanted: &[Agent]) -> Plan {
    let mut plan = Plan::default();
    let mut homebrew_needed = false;

    match &findings.tmux {
        TmuxStatus::Ok(_) => {}
        status => {
            let upgrade = matches!(status, TmuxStatus::TooOld(_));
            if findings.brew && platform != Platform::Other {
                plan.steps.push(if upgrade {
                    BREW_UPGRADE_TMUX
                } else {
                    BREW_TMUX
                });
            } else if platform == Platform::Linux
                && findings.apt
                && (findings.root || findings.sudo)
            {
                if findings.root {
                    plan.steps.extend([APT_UPDATE_ROOT, APT_TMUX_ROOT]);
                } else {
                    plan.steps.extend([APT_UPDATE_SUDO, APT_TMUX_SUDO]);
                }
                plan.notes.push(
                    "apt installs tmux 3.3 or newer only on Debian 12, Ubuntu 24.04 and newer; \
                     setup checks the version afterwards"
                        .to_owned(),
                );
            } else if platform == Platform::MacOs {
                homebrew_needed = true;
            } else {
                plan.notes.push(
                    "tmux 3.3 or newer is needed; install it with your system's package manager \
                     (docs/install.md)"
                        .to_owned(),
                );
            }
        }
    }

    for &agent in wanted {
        if findings.has(agent) {
            continue;
        }
        if findings.brew && platform == Platform::MacOs {
            plan.steps.push(match agent {
                Agent::Claude => BREW_CLAUDE,
                Agent::Codex => BREW_CODEX,
            });
        } else {
            plan.notes.push(match agent {
                Agent::Claude => {
                    "Claude Code: install it from https://claude.com/product/claude-code, \
                                  then log in once by running `claude`"
                        .to_owned()
                }
                Agent::Codex => {
                    "Codex: install it from https://github.com/openai/codex, then log in \
                                 once by running `codex`"
                        .to_owned()
                }
            });
            if platform == Platform::MacOs {
                homebrew_needed = true;
            }
        }
    }

    if homebrew_needed && !findings.brew {
        plan.notes.insert(0, HOMEBREW_NOTE.to_owned());
    }
    plan
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(tmux: TmuxStatus, claude: bool, brew: bool, apt: bool) -> Findings {
        Findings {
            tmux,
            claude,
            codex: false,
            brew,
            apt,
            sudo: apt,
            root: false,
        }
    }

    #[test]
    fn nothing_missing_means_nothing_to_do() {
        let f = found(TmuxStatus::Ok("tmux 3.4".into()), true, true, false);
        assert_eq!(plan(&f, Platform::MacOs, &[Agent::Claude]), Plan::default());
    }

    #[test]
    fn a_missing_tmux_on_a_mac_with_homebrew_is_one_brew_step() {
        let f = found(TmuxStatus::Missing, true, true, false);
        let p = plan(&f, Platform::MacOs, &[Agent::Claude]);
        assert_eq!(p.steps, vec![BREW_TMUX]);
        assert!(p.notes.is_empty());
    }

    #[test]
    fn an_old_tmux_is_upgraded_not_installed() {
        let f = found(TmuxStatus::TooOld("3.2".into()), true, true, false);
        assert_eq!(
            plan(&f, Platform::Linux, &[]).steps,
            vec![BREW_UPGRADE_TMUX]
        );
    }

    #[test]
    fn linux_without_homebrew_uses_apt_with_sudo_shown() {
        let f = found(TmuxStatus::Missing, true, false, true);
        let p = plan(&f, Platform::Linux, &[Agent::Claude]);
        assert_eq!(p.steps, vec![APT_UPDATE_SUDO, APT_TMUX_SUDO]);
        assert_eq!(p.steps[0].command_line(), "sudo apt-get update");
        assert_eq!(p.steps[1].command_line(), "sudo apt-get install -y tmux");
    }

    #[test]
    fn as_root_there_is_no_sudo_and_without_either_there_is_no_apt_step() {
        let mut f = found(TmuxStatus::Missing, true, false, true);
        f.root = true;
        f.sudo = false;
        let p = plan(&f, Platform::Linux, &[]);
        assert_eq!(p.steps, vec![APT_UPDATE_ROOT, APT_TMUX_ROOT]);
        assert!(!p.steps.iter().any(|s| s.argv.contains(&"sudo")));
        f.root = false;
        let p = plan(&f, Platform::Linux, &[]);
        assert!(p.steps.is_empty());
        assert!(p.notes.iter().any(|n| n.contains("package manager")));
    }

    #[test]
    fn a_mac_without_homebrew_gets_instructions_and_never_an_install() {
        let f = found(TmuxStatus::Missing, false, false, false);
        let p = plan(&f, Platform::MacOs, &[Agent::Claude]);
        assert!(p.steps.is_empty());
        assert!(p.notes.first().is_some_and(|n| n.contains("brew.sh")));
        assert!(p.notes.iter().any(|n| n.contains("Claude Code")));
    }

    #[test]
    fn agents_are_installed_by_cask_on_macos_and_explained_elsewhere() {
        let f = found(TmuxStatus::Ok("tmux 3.4".into()), false, true, false);
        let mac = plan(&f, Platform::MacOs, &[Agent::Claude, Agent::Codex]);
        assert_eq!(mac.steps, vec![BREW_CLAUDE, BREW_CODEX]);
        let linux = plan(&f, Platform::Linux, &[Agent::Claude]);
        assert!(linux.steps.is_empty());
        assert!(linux.notes.iter().any(|n| n.contains("claude.com")));
    }

    #[test]
    fn only_wanted_agents_are_considered() {
        let f = found(TmuxStatus::Ok("tmux 3.4".into()), false, true, false);
        assert!(plan(&f, Platform::MacOs, &[]).steps.is_empty());
    }

    #[test]
    fn every_step_is_a_fixed_command_without_a_shell() {
        for s in [
            BREW_TMUX,
            BREW_UPGRADE_TMUX,
            APT_UPDATE_SUDO,
            APT_TMUX_SUDO,
            APT_UPDATE_ROOT,
            APT_TMUX_ROOT,
            BREW_CLAUDE,
            BREW_CODEX,
        ] {
            assert!(s.argv.len() >= 2, "{s:?}");
            assert!(!s.argv.iter().any(|a| a.contains(['|', ';', '&', '$', '`'])));
            assert!(s.what.len() > 8);
        }
    }
}
