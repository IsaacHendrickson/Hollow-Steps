//! Undo and redo for the Charms panel, by snapshots, apart from the room's own history and the
//! Player panel's. A snapshot is the whole edited charm list. A drag, from the mouse press to
//! the release, is one step, and so is one stretch of typing in a text box; every other edit (a
//! click, a typed number, a Reset, New charm, Delete) is one step.
//!
//! The keys reach this history only while the Charms panel has focus (`charm_panel`); the room's
//! history gets them otherwise.

use std::collections::VecDeque;

use crate::charm_editor::CharmDraft;
use crate::charms::Charm;

/// Most undo steps kept; the oldest are dropped first.
pub const HISTORY_LIMIT: usize = 100;

type Snapshot = Vec<Charm>;

#[derive(Debug, Default)]
pub struct CharmHistory {
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
    /// The list as it was when a drag (or a stretch of typing) began, until it ends.
    gesture: Option<Snapshot>,
}

impl CharmHistory {
    pub fn can_undo(&self) -> bool {
        self.gesture.is_some() || !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        self.gesture.is_none() && !self.redo.is_empty()
    }

    pub fn gesture_open(&self) -> bool {
        self.gesture.is_some()
    }

    /// How many steps undo can go back.
    pub fn undo_steps(&self) -> usize {
        self.undo.len()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// One edit just happened: `before` is how the list was, `after` how it is. Does nothing if
    /// they are equal. A new edit clears redo. Ends an open drag first, so steps stay in order.
    pub fn record(&mut self, before: Snapshot, after: &[Charm]) {
        self.end_gesture(&before);
        if before != after {
            self.redo.clear();
            if self.undo.len() == HISTORY_LIMIT {
                self.undo.pop_front();
            }
            self.undo.push_back(before);
        }
    }

    /// A drag begins: remember `before` so the whole drag becomes one step.
    pub fn begin_gesture(&mut self, before: &[Charm]) {
        if self.gesture.is_none() {
            self.gesture = Some(before.to_vec());
        }
    }

    /// The drag ends. Records one step if the list changed meanwhile.
    pub fn end_gesture(&mut self, after: &[Charm]) {
        if let Some(before) = self.gesture.take() {
            self.record(before, after);
        }
    }

    /// Called once per frame with the list before and after the panel drew. While `held` (a
    /// mouse button is down, or a text box has the keyboard) a change belongs to a gesture; a
    /// change with nothing held is a step of its own; a frame with no change and nothing held
    /// closes a gesture that was open.
    pub fn observe(&mut self, before: Snapshot, after: &[Charm], held: bool) {
        if before == after {
            if !held {
                self.end_gesture(after);
            }
        } else if held {
            self.begin_gesture(&before);
        } else {
            self.record(before, after);
        }
    }

    /// Goes back one step; the list it replaces becomes a redo step. A drag or typing stretch
    /// still open is closed first, so it is what gets undone. `false` if there is nothing to
    /// undo.
    pub fn undo(&mut self, draft: &mut CharmDraft) -> bool {
        self.end_gesture(draft.edited());
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(draft.edited().to_vec());
        draft.set_edited(previous);
        true
    }

    /// The reverse of `undo`.
    pub fn redo(&mut self, draft: &mut CharmDraft) -> bool {
        self.end_gesture(draft.edited());
        let Some(next) = self.redo.pop() else {
            return false;
        };
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(draft.edited().to_vec());
        draft.set_edited(next);
        true
    }
}

#[cfg(test)]
mod tests;
