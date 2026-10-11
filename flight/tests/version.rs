// SPDX-License-Identifier: MIT

//! The real binary reports its version without touching tmux or any config.

use std::process::Command;

#[test]
fn version_flag_prints_one_line_and_succeeds() {
    for flag in ["--version", "-V"] {
        let out = Command::new(env!("CARGO_BIN_EXE_flight"))
            .arg(flag)
            .output()
            .unwrap();
        assert!(out.status.success(), "{flag}");
        let text = String::from_utf8(out.stdout).unwrap();
        assert!(
            text.starts_with(&format!("flight {} (", env!("CARGO_PKG_VERSION"))),
            "{text:?}"
        );
        assert_eq!(text.lines().count(), 1);
    }
}
