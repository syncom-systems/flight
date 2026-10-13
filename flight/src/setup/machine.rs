// SPDX-License-Identifier: MIT

use super::{Platform, Step};
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

/// What setup needs to know about, and do to, the machine. Everything it touches is here, so
/// tests can stand in for the machine and prove that nothing runs without consent.
pub trait Machine {
    fn platform(&self) -> Platform;
    /// Whether `program` is an executable file on `PATH`.
    fn which(&self, program: &str) -> bool;
    /// The output of `tmux -V`, if tmux runs.
    fn tmux_version(&self) -> Option<String>;
    /// Whether setup is running as the administrator (then `sudo` is neither needed nor used).
    fn is_root(&self) -> bool;
    /// Run the step with the terminal attached (output shown, `sudo` able to ask for a
    /// password). The exit code, or why it could not start.
    fn run(&mut self, step: &Step) -> Result<i32, String>;
}

/// This machine.
pub struct SystemMachine;

impl Machine for SystemMachine {
    fn platform(&self) -> Platform {
        Platform::current()
    }

    fn which(&self, program: &str) -> bool {
        let Some(path) = std::env::var_os("PATH") else {
            return false;
        };
        std::env::split_paths(&path)
            .filter(|d| d.is_absolute())
            .map(|d| d.join(program))
            .any(|p| {
                std::fs::metadata(&p)
                    .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
            })
    }

    fn tmux_version(&self) -> Option<String> {
        let out = Command::new("tmux").arg("-V").output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    }

    fn is_root(&self) -> bool {
        Command::new("id")
            .arg("-u")
            .output()
            .is_ok_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "0")
    }

    fn run(&mut self, step: &Step) -> Result<i32, String> {
        let status = Command::new(step.program())
            .args(step.args())
            .status()
            .map_err(|e| format!("could not start {}: {e}", step.program()))?;
        Ok(status.code().unwrap_or(-1))
    }
}
