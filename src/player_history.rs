//! Undo and redo for the Player panel, by snapshots, apart from the room's own history.
//!
//! A snapshot holds what the panel edits: the stat numbers (`player.ron`), the look
//! (`player_body.ron`) and the look's body template (the Parts and motion values, a shared
//! `Arc`, so a snapshot is cheap). Pose files are not in it. A drag, from the mouse press to
//! the release, is one step; every other edit (a click, a typed number, a Reset) is one step.
//!
//! The room's `editor_history` is untouched: the keys reach this history only while the Player
//! panel has focus (`player_panel`), and the room's history gets them otherwise.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::body::BodyTemplate;
use crate::editor::EditorSession;
use crate::player_body::PlayerLook;
use crate::tuning::TuningConfig;

/// Most undo steps kept; the oldest are dropped first.
pub const HISTORY_LIMIT: usize = 100;

/// What the Player panel edits, at one moment.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerSnapshot {
    stats: TuningConfig,
    look: PlayerLook,
    /// The look's body template, by name, when the editor has it.
    template: Option<(String, Arc<BodyTemplate>)>,
}

impl PlayerSnapshot {
    pub fn capture(session: &EditorSession) -> Self {
        let look = session.look.edited().clone();
        let name = &look.visual.template;
        Self {
            stats: *session.player.edited(),
            template: session
                .content
                .bodies
                .get(name)
                .map(|template| (name.clone(), Arc::clone(template))),
            look,
        }
    }

    /// Puts these values back into the session. A template that differs from the one the
    /// editor holds marks the editor unsaved, since the editor's Save is what writes it.
    pub fn restore(&self, session: &mut EditorSession) {
        // The numbers were in range when captured, so this cannot refuse them.
        let _ = session.player.set_edited(self.stats);
        session.look.restore_edited(self.look.clone());
        if let Some((name, template)) = &self.template {
            if session.content.bodies.get(name) != Some(template) {
                session
                    .content
                    .bodies
                    .insert(name.clone(), Arc::clone(template));
                session.dirty = true;
            }
        }
    }
}

#[derive(Debug, Default)]
pub struct PlayerHistory {
    undo: VecDeque<PlayerSnapshot>,
    redo: Vec<PlayerSnapshot>,
    /// The values as they were when a drag began, until the drag ends.
    gesture: Option<PlayerSnapshot>,
}

impl PlayerHistory {
    /// Undo and redo are unavailable while a drag is in progress.
    pub fn can_undo(&self) -> bool {
        self.gesture.is_none() && !self.undo.is_empty()
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

    /// One edit just happened: `before` is how things were, `after` how they are. Does
    /// nothing if they are equal. A new edit clears redo. Ends an open drag first, so the
    /// steps stay in order.
    pub fn record(&mut self, before: PlayerSnapshot, after: &PlayerSnapshot) {
        self.end_gesture(&before);
        if &before != after {
            self.redo.clear();
            if self.undo.len() == HISTORY_LIMIT {
                self.undo.pop_front();
            }
            self.undo.push_back(before);
        }
    }

    /// A drag begins: remember `before` so the whole drag becomes one step. Ignored if a
    /// drag is already open.
    pub fn begin_gesture(&mut self, before: &PlayerSnapshot) {
        if self.gesture.is_none() {
            self.gesture = Some(before.clone());
        }
    }

    /// The drag ends. Records one step if the values changed meanwhile.
    pub fn end_gesture(&mut self, after: &PlayerSnapshot) {
        if let Some(before) = self.gesture.take() {
            self.record(before, after);
        }
    }

    /// Called once per frame with the values before and after the panel drew. While a mouse
    /// button is down a change belongs to a drag; a change with no button down is a step of
    /// its own; a frame with no change and no button down closes a drag that was open.
    pub fn observe(&mut self, before: PlayerSnapshot, after: &PlayerSnapshot, pointer_down: bool) {
        if before == *after {
            if !pointer_down {
                self.end_gesture(after);
            }
        } else if pointer_down {
            self.begin_gesture(&before);
        } else {
            self.record(before, after);
        }
    }

    /// Goes back one step; the values it replaces become a redo step. `false` if there is
    /// nothing to undo or a drag is open.
    pub fn undo(&mut self, session: &mut EditorSession) -> bool {
        if self.gesture.is_some() {
            return false;
        }
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(PlayerSnapshot::capture(session));
        previous.restore(session);
        true
    }

    /// The reverse of `undo`.
    pub fn redo(&mut self, session: &mut EditorSession) -> bool {
        if self.gesture.is_some() {
            return false;
        }
        let Some(next) = self.redo.pop() else {
            return false;
        };
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.pop_front();
        }
        self.undo.push_back(PlayerSnapshot::capture(session));
        next.restore(session);
        true
    }
}
