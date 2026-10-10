// SPDX-License-Identifier: MIT

use flight_state::{HostId, SurfaceId, WorkspaceId};

/// The kinds of surface that can be added to a workspace that already exists. An agent is not
/// one of them: it is created with the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewSurface {
    /// A shell in the workspace's root directory, started the way the node starts any shell.
    Shell,
}

/// A validated request to add a surface to a workspace. It names the workspace and nothing
/// else: the host is the node asked, and the directory and session are the workspace's own,
/// which this node reads from its backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRequest {
    /// This node. Set by the node from its own identity; never taken from a request.
    pub host: HostId,
    pub workspace_id: WorkspaceId,
    pub kind: NewSurface,
    /// Add the surface beside those of its kind that exist. Without it a workspace that has
    /// one refuses, as it always has: only a caller that says so, knowing it has the
    /// capability, gets a second.
    pub another: bool,
}

/// A validated request to take one surface away from a workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceRemoval {
    /// This node. Set by the node from its own identity; never taken from a request.
    pub host: HostId,
    pub workspace_id: WorkspaceId,
    pub surface_id: SurfaceId,
}
