// SPDX-License-Identifier: MIT

//! Selection movement, search and reconciliation, kept apart from the model's data.

use super::{Effect, FilterInput, ViewModel};
use crate::snapshot::WorkspaceKey;
use flight_state::PaneRef;

fn position(list: &[WorkspaceKey], sel: Option<&WorkspaceKey>) -> Option<usize> {
    sel.and_then(|s| list.iter().position(|k| k == s))
}

impl ViewModel {
    /// Make the selection valid for the new snapshot or filter without moving it onto another
    /// workspace unless its own is gone from the list.
    pub(super) fn reconcile(&mut self) {
        let list = self.keys();
        if let Some(i) = position(&list, self.selected.as_ref()) {
            self.hint = i;
            return;
        }
        let idx = self.hint.min(list.len().saturating_sub(1));
        self.selected = list.get(idx).cloned();
        self.hint = idx;
    }

    /// Select a workspace just created, as soon as a snapshot has it. The wait ends when it
    /// shows up, when the user moves the cursor, or after a while.
    pub(super) fn select_created(&mut self) {
        let Some(pending) = self.pending.as_mut() else {
            return;
        };
        let found = self
            .snapshot
            .hosts
            .iter()
            .filter(|h| h.host == pending.host)
            .flat_map(|h| h.panes.iter())
            .find(|p| p.session == pending.session)
            .map(|p| WorkspaceKey {
                host: p.pane_ref.host.clone(),
                workspace: p.workspace.clone(),
            });
        if let Some(key) = found {
            // The new workspace must be visible: a search that hides it is dropped.
            if !self.keys().contains(&key) {
                self.filter.clear();
                self.searching = false;
            }
            self.hint = self.keys().iter().position(|k| *k == key).unwrap_or(0);
            self.selected = Some(key);
            self.pending = None;
        } else {
            pending.snapshots_left = pending.snapshots_left.saturating_sub(1);
            if pending.snapshots_left == 0 {
                self.pending = None;
            }
        }
    }

    pub(super) fn step(&mut self, delta: isize) -> Effect {
        self.pending = None;
        self.pointing = None;
        self.opening = None;
        self.surface = None;
        let before = self.selected();
        let list = self.keys();
        let next = match position(&list, self.selected.as_ref()) {
            Some(i) => i
                .saturating_add_signed(delta)
                .min(list.len().saturating_sub(1)),
            None => 0,
        };
        self.selected = list.get(next).cloned();
        self.hint = next;
        self.select_effect(before)
    }

    /// Point at the workspace a listed pane belongs to (a click). Anything not listed is
    /// ignored.
    pub(super) fn select(&mut self, pane: &PaneRef) -> Effect {
        self.pending = None;
        let Some((i, key)) = self.listed().iter().enumerate().find_map(|(i, w)| {
            w.surfaces
                .iter()
                .any(|s| &s.pane.pane_ref == pane)
                .then(|| (i, w.key()))
        }) else {
            return Effect::None;
        };
        let before = self.selected();
        if self.selected.as_ref() != Some(&key) {
            self.surface = None;
        }
        self.selected = Some(key);
        self.hint = i;
        self.select_effect(before)
    }

    /// Move the marker to the next or previous surface of the selected workspace, wrapping.
    /// Landing on the first (the default) clears it.
    pub(super) fn step_surface(&mut self, delta: isize) -> Effect {
        self.pending = None;
        let Some(workspace) = self.selected_workspace() else {
            return Effect::None;
        };
        let count = workspace.surfaces.len();
        if count < 2 {
            return Effect::None;
        }
        let at = self
            .chosen_in(&workspace)
            .and_then(|c| workspace.surfaces.iter().position(|s| s.id == c.id))
            .unwrap_or(0);
        let next = if delta < 0 {
            at.checked_sub(1).unwrap_or(count.saturating_sub(1))
        } else if at.saturating_add(1) >= count {
            0
        } else {
            at.saturating_add(1)
        };
        let before = self.selected();
        self.surface = (next != 0)
            .then(|| workspace.surfaces.get(next).map(|s| s.id.clone()))
            .flatten();
        self.select_effect(before)
    }

    pub(super) fn filter_input(&mut self, input: FilterInput) -> Effect {
        match input {
            FilterInput::Char(c) => self.filter.push(c),
            FilterInput::Backspace => {
                self.filter.pop();
            }
            FilterInput::Accept => {
                self.searching = false;
                return Effect::None;
            }
            FilterInput::Clear => {
                self.filter.clear();
                self.searching = false;
            }
        }
        let before = self.selected();
        self.reconcile();
        self.select_effect(before)
    }
}
