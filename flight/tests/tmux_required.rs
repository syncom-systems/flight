// SPDX-License-Identifier: MIT

//! Without a usable tmux, Flight says so at once and creates nothing.

use std::process::Command;

/// `name` keeps each test's scratch directory its own: the tests run in parallel.
fn without_tmux(name: &str, args: &[&str]) -> (bool, String, std::path::PathBuf) {
    let scratch =
        std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("no-tmux-{name}"));
    let _ = std::fs::remove_dir_all(&scratch);
    let empty = scratch.join("empty-path");
    std::fs::create_dir_all(&empty).unwrap();
    let config = scratch.join("config");
    let out = Command::new(env!("CARGO_BIN_EXE_flight"))
        .args(args)
        .env("PATH", &empty)
        .env("FLIGHT_CONFIG_DIR", &config)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        config,
    )
}

#[test]
fn solo_without_tmux_explains_and_creates_nothing() {
    let (ok, stderr, config) = without_tmux("solo", &["solo"]);
    assert!(!ok);
    assert!(
        stderr.contains("tmux") && stderr.contains("docs/install.md"),
        "{stderr}"
    );
    assert!(
        !config.exists(),
        "nothing is created before tmux is known to work"
    );
}

#[test]
fn the_version_flag_needs_no_tmux() {
    let (ok, _, _) = without_tmux("version", &["--version"]);
    assert!(ok);
}
