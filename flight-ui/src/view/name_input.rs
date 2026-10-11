// SPDX-License-Identifier: MIT

/// What the user did in the shell-name prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameInput {
    Char(char),
    Backspace,
    /// Enter: use what is typed (nothing typed clears the name).
    Enter,
    Cancel,
}
