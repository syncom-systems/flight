// SPDX-License-Identifier: MIT

use std::path::PathBuf;

/// The program Codex is started as: found on the node's `PATH`.
pub(super) const PROGRAM: &str = "codex";

/// Codex's flag that never asks before acting and does not sandbox. Spelled out in full in the
/// form and never a default (ADR-013).
const BYPASS: &str = "--dangerously-bypass-approvals-and-sandbox";

/// A new Codex session in the session's directory (tmux sets it). Codex assigns its own
/// conversation id and reports none (ADR-010), so no identity is chosen and no reference kept.
pub(super) fn new_argv(codex: PathBuf, skip: bool) -> Vec<String> {
    let mut argv = vec![codex.to_string_lossy().into_owned()];
    if skip {
        argv.push(BYPASS.to_owned());
    }
    argv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_codex_asks_before_acting_unless_the_flag_is_chosen() {
        assert_eq!(new_argv("/bin/codex".into(), false), ["/bin/codex"]);
        assert_eq!(
            new_argv("/bin/codex".into(), true),
            ["/bin/codex", "--dangerously-bypass-approvals-and-sandbox"]
        );
    }
}
