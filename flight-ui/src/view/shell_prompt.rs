// SPDX-License-Identifier: MIT

use super::{NewSurfaceRequest, PromptInput, SurfaceChoice};
use crate::collect::CreateFailure;
use crate::snapshot::{SurfaceKind, Workspace, WorkspaceKey};
use flight_state::SurfaceId;

/// Which button has the focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptButton {
    Create,
    Cancel,
}

/// What the prompt wants done after an input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptOutcome {
    None,
    Cancel,
    Submit(NewSurfaceRequest),
}

/// "No shell yet: create one in this workspace?" Pure state, like the new-workspace form. The
/// workspace's host and directory are shown, never asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellPrompt {
    key: WorkspaceKey,
    name: String,
    host_label: String,
    root: String,
    focus: PromptButton,
    error: Option<String>,
    submitting: bool,
    /// Another shell beside those the workspace has, not its first.
    another: bool,
    known: Vec<SurfaceId>,
}

impl ShellPrompt {
    pub fn new(workspace: &Workspace) -> Self {
        Self {
            another: false,
            known: Vec::new(),
            ..Self::base(workspace)
        }
    }

    /// Asks to add one more shell to a workspace that has some.
    pub fn another(workspace: &Workspace) -> Self {
        Self {
            another: true,
            known: workspace
                .surfaces
                .iter()
                .filter(|s| s.kind == SurfaceKind::Shell)
                .map(|s| s.id.clone())
                .collect(),
            ..Self::base(workspace)
        }
    }

    fn base(workspace: &Workspace) -> Self {
        Self {
            another: false,
            known: Vec::new(),
            key: workspace.key(),
            name: workspace.name.clone(),
            host_label: workspace.host_label.clone(),
            root: workspace.root.clone(),
            focus: PromptButton::Create,
            error: None,
            submitting: false,
        }
    }

    pub fn key(&self) -> &WorkspaceKey {
        &self.key
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn host_label(&self) -> &str {
        &self.host_label
    }

    pub fn is_another(&self) -> bool {
        self.another
    }

    pub fn root(&self) -> &str {
        &self.root
    }

    pub fn focus(&self) -> PromptButton {
        self.focus
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn submitting(&self) -> bool {
        self.submitting
    }

    pub fn handle(&mut self, input: PromptInput) -> PromptOutcome {
        if self.submitting {
            return PromptOutcome::None;
        }
        match input {
            PromptInput::Next => {
                self.focus = match self.focus {
                    PromptButton::Create => PromptButton::Cancel,
                    PromptButton::Cancel => PromptButton::Create,
                };
                PromptOutcome::None
            }
            PromptInput::Cancel => PromptOutcome::Cancel,
            PromptInput::Yes => self.submit(),
            PromptInput::Enter => match self.focus {
                PromptButton::Create => self.submit(),
                PromptButton::Cancel => PromptOutcome::Cancel,
            },
        }
    }

    fn submit(&mut self) -> PromptOutcome {
        self.error = None;
        self.submitting = true;
        PromptOutcome::Submit(NewSurfaceRequest {
            host: self.key.host.clone(),
            workspace: self.key.workspace.clone(),
            name: self.name.clone(),
            host_label: self.host_label.clone(),
            kind: SurfaceChoice::Shell,
            another: self.another,
            known: self.known.clone(),
        })
    }

    /// The node refused (or could not be reached): say so in the prompt, which stays open.
    pub fn fail(&mut self, failure: &CreateFailure) {
        self.submitting = false;
        self.error = Some(match failure {
            CreateFailure::Unreachable => format!(
                "{} is not connected right now. Nothing was created.",
                self.host_label
            ),
            CreateFailure::UnknownWorkspace => {
                "That workspace is gone from its host. Nothing was created.".to_owned()
            }
            CreateFailure::Unsupported if self.another => {
                "This dashboard or the workspace's node cannot hold more than one shell. \
                 Nothing was created."
                    .to_owned()
            }
            CreateFailure::Unsupported => {
                "This dashboard is not connected to an orchestrator, so it cannot add a shell."
                    .to_owned()
            }
            CreateFailure::ProgramUnavailable(why) | CreateFailure::Other(why) => why.clone(),
            CreateFailure::NoSuchDirectory(why) => why.clone(),
            CreateFailure::AlreadyExists => "This workspace already has a shell.".to_owned(),
        });
    }
}
