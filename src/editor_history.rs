//! Undo and redo for the room draft, by snapshots. Only the draft (placements, layout,
//! boss placement) is tracked; entity, behavior and boss definition edits
//! (`EditorSession::content`) are not part of it.
use std::collections::VecDeque;

use crate::boss_definition::BossDefinitions;
use crate::editor::{EditorSession, RoomDraft};
use crate::enemy::EnemyDefinitions;

/// Most undo steps kept; the oldest are dropped first.
pub const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryKey {
    Undo,
    Redo,
}

/// Maps the keyboard to a history command: Ctrl+Z undoes; Ctrl+Y and Ctrl+Shift+Z redo.
/// `z` and `y` are "pressed this frame" (not held).
pub fn shortcut(ctrl: bool, shift: bool, z: bool, y: bool) -> Option<HistoryKey> {
    match (ctrl, z, y) {
        (true, true, _) if shift => Some(HistoryKey::Redo),
        (true, true, _) => Some(HistoryKey::Undo),
        (true, false, true) => Some(HistoryKey::Redo),
        _ => None,
    }
}

#[derive(Debug, Default)]
pub struct History {
    undo: VecDeque<RoomDraft>,
    redo: Vec<RoomDraft>,
    /// The draft as it was when a drag began, until the drag ends.
    gesture: Option<RoomDraft>,
}

impl History {
    /// Undo and redo are unavailable while a drag is in progress.
    pub fn can_undo(&self) -> bool {
        self.gesture.is_none() && !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        self.gesture.is_none() && !self.redo.is_empty()
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// A one-step change (place, delete, exit or item edit, room size) just happened:
    /// `before` is the draft as it was and `after` the draft now. Does nothing if they are
    /// equal. A new edit clears redo. Ends an open drag first, so steps stay in order.
    pub fn record(&mut self, before: RoomDraft, after: &RoomDraft) {
        self.end_gesture(&before);
        if &before != after {
            self.redo.clear();
            self.push_undo(before);
        }
    }

    /// A drag begins (mouse press): remember the draft so the whole drag becomes one step.
    /// Ignored if a drag is already open.
    pub fn begin_gesture(&mut self, before: &RoomDraft) {
        if self.gesture.is_none() {
            self.gesture = Some(before.clone());
        }
    }

    /// The drag ends (mouse release). Records one step if the draft changed meanwhile.
    pub fn end_gesture(&mut self, after: &RoomDraft) {
        if let Some(before) = self.gesture.take() {
            self.record(before, after);
        }
    }

    pub fn gesture_open(&self) -> bool {
        self.gesture.is_some()
    }

    /// Restores the previous draft into `draft`; the draft it replaces becomes a redo step.
    /// Returns `Ok(false)` if there is nothing to undo. `valid` checks the snapshot against
    /// the current definitions (which can have changed since): a snapshot that fails is
    /// dropped, `draft` is left as it is, and the reason is returned.
    pub fn undo(
        &mut self,
        draft: &mut RoomDraft,
        valid: impl Fn(&RoomDraft) -> Result<(), String>,
    ) -> Result<bool, String> {
        if self.gesture.is_some() {
            return Ok(false);
        }
        let Some(previous) = self.undo.pop_back() else {
            return Ok(false);
        };
        valid(&previous)?;
        self.redo.push(std::mem::replace(draft, previous));
        Ok(true)
    }

    /// The reverse of `undo`.
    pub fn redo(
        &mut self,
        draft: &mut RoomDraft,
        valid: impl Fn(&RoomDraft) -> Result<(), String>,
    ) -> Result<bool, String> {
        if self.gesture.is_some() {
            return Ok(false);
        }
        let Some(next) = self.redo.pop() else {
            return Ok(false);
        };
        valid(&next)?;
        self.push_undo(std::mem::replace(draft, next));
        Ok(true)
    }

    fn push_undo(&mut self, snapshot: RoomDraft) {
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(snapshot);
    }
}

/// Runs a draft edit and records it as one undo step if it succeeded and changed the draft.
pub fn tracked<R>(
    history: &mut History,
    session: &mut EditorSession,
    edit: impl FnOnce(&mut EditorSession) -> Result<R, String>,
) -> Result<R, String> {
    let before = session.draft.clone();
    let result = edit(session)?;
    history.record(before, &session.draft);
    Ok(result)
}

/// Whether a snapshot is still a valid room with the current definitions.
fn restorable(
    draft: &RoomDraft,
    enemies: &EnemyDefinitions,
    bosses: Result<&BossDefinitions, &String>,
) -> Result<(), String> {
    draft
        .validate_local(enemies)
        .map_err(|e| format!("cannot restore that state: {e}"))?;
    if draft.boss.is_some() {
        draft
            .validate_boss(bosses.map_err(|e| e.clone())?)
            .map_err(|e| format!("cannot restore that state: {e}"))?;
    }
    Ok(())
}

/// Undoes one step in the session and marks it unsaved. `Ok(false)` if there was nothing
/// to undo. The error says why a step could not be restored (for example, it uses an
/// entity that was renamed or deleted since, which undo does not cover).
pub fn undo_session(history: &mut History, session: &mut EditorSession) -> Result<bool, String> {
    let bosses = session.content.boss_definitions();
    let changed = history.undo(&mut session.draft, |d| {
        restorable(d, &session.definitions, bosses.as_ref())
    })?;
    session.dirty |= changed;
    Ok(changed)
}

/// Redoes one step in the session; see `undo_session`.
pub fn redo_session(history: &mut History, session: &mut EditorSession) -> Result<bool, String> {
    let bosses = session.content.boss_definitions();
    let changed = history.redo(&mut session.draft, |d| {
        restorable(d, &session.definitions, bosses.as_ref())
    })?;
    session.dirty |= changed;
    Ok(changed)
}
