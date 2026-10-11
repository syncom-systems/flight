// SPDX-License-Identifier: MIT

//! Embeds the commit the binary was built from, for `flight --version`.
//! `FLIGHT_COMMIT` (set by the release workflow) wins; otherwise git is asked; outside a
//! checkout the commit is `unknown`.

use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=FLIGHT_COMMIT");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    let commit = std::env::var("FLIGHT_COMMIT")
        .ok()
        .filter(|c| !c.is_empty())
        .or_else(from_git)
        .unwrap_or_else(|| "unknown".to_owned());
    println!("cargo:rustc-env=FLIGHT_BUILD_COMMIT={commit}");
}

fn from_git() -> Option<String> {
    let out = Command::new("git")
        .args(["rev-parse", "--short=12", "HEAD"])
        .output()
        .ok()?;
    let text = String::from_utf8(out.stdout).ok()?;
    let text = text.trim();
    (out.status.success() && !text.is_empty()).then(|| text.to_owned())
}
