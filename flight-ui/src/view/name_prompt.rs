// SPDX-License-Identifier: MIT

use super::{NameInput, NameSurfaceRequest};
use crate::collect::CreateFailure;
use crate::snapshot::{Surface, Workspace};
use flight_state::{clean_label, HostId, SurfaceId, WorkspaceId, MAX_LABEL_CHARS};

/// What the prompt wants done after an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameOutcome {
    None,
    Cancel,
    Submit(NameSurfaceRequest),
}

/// "Name this shell:" with a line to type. Pure state, like the other prompts. The label is
/// presentation; it never selects a surface, and another shell may carry the same one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePrompt {
    host: HostId,
    workspace: WorkspaceId,
    workspace_name: String,
    surface: SurfaceId,
    /// What the shell is called now, for the title.
    current: Option<String>,
    text: String,
    error: Option<String>,
    submitting: bool,
}

impl NamePrompt {
    pub fn new(workspace: &Workspace, surface: &Surface) -> Self {
        let current = surface.label().map(str::to_owned);
        Self {
            host: workspace.host.clone(),
            workspace: workspace.id.clone(),
            workspace_name: workspace.name.clone(),
            surface: surface.id.clone(),
            text: current.clone().unwrap_or_default(),
            current,
            error: None,
            submitting: false,
        }
    }

    pub fn workspace_name(&self) -> &str {
        &self.workspace_name
    }

    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn submitting(&self) -> bool {
        self.submitting
    }

    pub fn handle(&mut self, input: NameInput) -> NameOutcome {
        if self.submitting {
            return NameOutcome::None;
        }
        match input {
            NameInput::Char(c) => {
                self.error = None;
                if !c.is_control() && self.text.chars().count() < MAX_LABEL_CHARS {
                    self.text.push(c);
                }
                NameOutcome::None
            }
            NameInput::Backspace => {
                self.error = None;
                self.text.pop();
                NameOutcome::None
            }
            NameInput::Cancel => NameOutcome::Cancel,
            NameInput::Enter => self.submit(),
        }
    }

    fn submit(&mut self) -> NameOutcome {
        let name = if self.text.trim().is_empty() {
            None
        } else {
            match clean_label(&self.text) {
                Some(name) => Some(name),
                None => {
                    self.error = Some("That is not a usable name.".to_owned());
                    return NameOutcome::None;
                }
            }
        };
        self.error = None;
        self.submitting = true;
        NameOutcome::Submit(NameSurfaceRequest {
            host: self.host.clone(),
            workspace: self.workspace.clone(),
            workspace_name: self.workspace_name.clone(),
            surface: self.surface.clone(),
            name,
        })
    }

    /// The node refused (or could not be reached): say so, and stay open.
    pub fn fail(&mut self, failure: &CreateFailure) {
        self.submitting = false;
        self.error = Some(match failure {
            CreateFailure::Unreachable => {
                "The workspace's host is not connected right now.".to_owned()
            }
            CreateFailure::UnknownWorkspace => {
                "That workspace is gone from its host. Nothing was changed.".to_owned()
            }
            CreateFailure::Unsupported => {
                "This dashboard or the workspace's node cannot keep shell names.".to_owned()
            }
            CreateFailure::ProgramUnavailable(why)
            | CreateFailure::Other(why)
            | CreateFailure::NoSuchDirectory(why) => why.clone(),
            CreateFailure::AlreadyExists => "That could not be changed.".to_owned(),
        });
    }
}
