// SPDX-License-Identifier: MIT

//! Refuse early, and say why, when tmux is missing or older than Flight supports.

use flight_tmux::TmuxVersion;
use std::process::Command;

/// Succeeds when `tmux -V` reports a supported version. A build that names no version
/// (`tmux master`) is let through: it cannot be judged.
pub fn require_supported_tmux() -> Result<(), String> {
    let output = Command::new("tmux")
        .arg("-V")
        .output()
        .map_err(|e| format!("tmux is needed and could not be run ({e}); {HELP}"))?;
    judge(&String::from_utf8_lossy(&output.stdout))
}

fn judge(reported: &str) -> Result<(), String> {
    match TmuxVersion::parse(reported) {
        Some(found) if !found.is_supported() => Err(format!(
            "tmux {found} is older than the {} Flight needs; {HELP}",
            TmuxVersion::MINIMUM
        )),
        _ => Ok(()),
    }
}

const HELP: &str = "see docs/install.md";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_tmux_is_refused_with_both_versions_named() {
        let why = judge("tmux 3.2a").unwrap_err();
        assert!(why.contains("3.2") && why.contains("3.3"), "{why}");
    }

    #[test]
    fn a_supported_or_unnumbered_tmux_passes() {
        assert!(judge("tmux 3.3a").is_ok());
        assert!(judge("tmux 3.7b").is_ok());
        assert!(judge("tmux master").is_ok());
    }
}
