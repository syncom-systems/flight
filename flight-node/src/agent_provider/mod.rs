// SPDX-License-Identifier: MIT

//! The agent programs a node can start for a workspace (ADR-013).
//!
//! A provider is an adapter: which program to find, how to start it new, how to start it again
//! when the earlier conversation can be continued, and whether it can be at all. Everything that
//! used to name `claude` in the launch path goes through here, so the rest of the node (the
//! workspace model, saved definitions, recovery, the wire) speaks of "an agent with a provider
//! id" and nothing else.

mod codex;
mod provider;

pub(crate) use provider::Provider;
