// SPDX-License-Identifier: MIT

/// One command setup may run, once the person has said yes. A fixed table entry: the program
/// and every argument are known at build time, nothing is composed from input, and no shell
/// is involved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// What it achieves, in words for the person.
    pub what: &'static str,
    /// The program, then its arguments. `sudo` appears here, visibly, when it is needed.
    pub argv: &'static [&'static str],
}

impl Step {
    /// The command as the person would type it.
    pub fn command_line(&self) -> String {
        self.argv.join(" ")
    }

    pub fn program(&self) -> &'static str {
        self.argv.first().copied().unwrap_or("")
    }

    pub fn args(&self) -> &'static [&'static str] {
        self.argv.get(1..).unwrap_or(&[])
    }
}
