//! Undo and redo by snapshots of a list the editor edits, for a panel that has its own history
//! apart from the room's (the Upgrades panel's is `SnapshotHistory<Upgrade>`). A snapshot is the
//! whole edited list. A drag, from the mouse press to the release, is one step, and so is one
//! stretch of typing in a text box; every other edit (a click, a typed number, a Reset, New,
//! Delete, Add level) is one step.
//!
//! The keys reach a panel's history only while that panel has focus (the panel decides); the
//! room's history gets them otherwise. The list is reached through `Snapshots`, so a draft keeps
//! its own rules for what it holds.

use std::collections::VecDeque;

/// Most undo steps kept; the oldest are dropped first.
pub const HISTORY_LIMIT: usize = 100;

/// A draft whose edited list a history can read and replace.
pub trait Snapshots<T> {
    /// The list as it is now.
    fn snapshot(&self) -> &[T];
    /// Puts back a list that was this draft's once; it is held as it is.
    fn restore(&mut self, list: Vec<T>);
}

#[derive(Debug)]
pub struct SnapshotHistory<T> {
    undo: VecDeque<Vec<T>>,
    redo: Vec<Vec<T>>,
    /// The list as it was when a drag (or a stretch of typing) began, until it ends.
    gesture: Option<Vec<T>>,
}

impl<T> Default for SnapshotHistory<T> {
    fn default() -> Self {
        Self {
            undo: VecDeque::new(),
            redo: Vec::new(),
            gesture: None,
        }
    }
}

impl<T: Clone + PartialEq> SnapshotHistory<T> {
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
    pub fn record(&mut self, before: Vec<T>, after: &[T]) {
        self.end_gesture(&before);
        if before != after {
            self.redo.clear();
            self.push_undo(before);
        }
    }

    fn push_undo(&mut self, snapshot: Vec<T>) {
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
    }

    /// A drag begins: remember `before` so the whole drag becomes one step.
    pub fn begin_gesture(&mut self, before: &[T]) {
        if self.gesture.is_none() {
            self.gesture = Some(before.to_vec());
        }
    }

    /// The drag ends. Records one step if the list changed meanwhile.
    pub fn end_gesture(&mut self, after: &[T]) {
        if let Some(before) = self.gesture.take() {
            self.record(before, after);
        }
    }

    /// Called once per frame with the list before and after the panel drew. While `held` (a
    /// mouse button is down, or a text box has the keyboard) a change belongs to a gesture; a
    /// change with nothing held is a step of its own; a frame with no change and nothing held
    /// closes a gesture that was open.
    pub fn observe(&mut self, before: Vec<T>, after: &[T], held: bool) {
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
    pub fn undo(&mut self, draft: &mut impl Snapshots<T>) -> bool {
        self.end_gesture(draft.snapshot());
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(draft.snapshot().to_vec());
        draft.restore(previous);
        true
    }

    /// The reverse of `undo`.
    pub fn redo(&mut self, draft: &mut impl Snapshots<T>) -> bool {
        self.end_gesture(draft.snapshot());
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.push_undo(draft.snapshot().to_vec());
        draft.restore(next);
        true
    }
}

#[cfg(test)]
mod tests;
