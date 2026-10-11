// SPDX-License-Identifier: MIT

use flight_state::{HostId, SurfaceId, WorkspaceId};

/// A request to give one surface a label, or to clear it (`name: None`). The surface is named
/// by its own id under its workspace's; the label is presentation and never selects anything.
/// `host` and `workspace_name` are for messages; they are not sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameSurfaceRequest {
    pub host: HostId,
    pub workspace: WorkspaceId,
    pub workspace_name: String,
    pub surface: SurfaceId,
    /// Already a clean label (`flight_state::clean_label`).
    pub name: Option<String>,
}
