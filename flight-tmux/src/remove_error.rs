// SPDX-License-Identifier: MIT

use crate::TmuxError;
use std::fmt;

/// Why a surface's window was not removed. In every case nothing else was touched.
#[derive(Debug)]
pub enum RemoveError {
    /// The session has no window carrying that surface id.
    NotFound,
    /// It is the session's only window: removing it would end the session, which is the
    /// whole workspace.
    OnlyWindow,
    Tmux(TmuxError),
}

impl fmt::Display for RemoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound => write!(f, "the workspace has no such surface"),
            Self::OnlyWindow => write!(f, "it is the only surface of its workspace"),
            Self::Tmux(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for RemoveError {}

impl From<TmuxError> for RemoveError {
    fn from(e: TmuxError) -> Self {
        Self::Tmux(e)
    }
}
