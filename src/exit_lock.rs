//! Keys and locked exits (`docs/keys-and-locks.md`).
//!
//! A key blocks entry and the game enforces it: an exit with `locked_by: Some(n)` refuses the
//! player until the run holds `Item::Key(n)`, then uses that key up and stays open for the rest
//! of the run. Here: the key ids, the labels the editor and the game show, and `OpenedExits`,
//! which remembers the opened doors so that a room rebuilt from its file (the editor's Save)
//! gets them open again. An ability such as Dash is different: the game never checks it at a door.
use macroquad::math::{Rect, Vec2};

use crate::editor::RoomDraft;
use crate::item::Item;
use crate::room::{Exit, RoomEdge, RoomId};

/// The lowest and highest key id a room file can use.
pub const FIRST_KEY: u8 = 1;
pub const LAST_KEY: u8 = 8;

/// `Ok` for a key id from `FIRST_KEY` to `LAST_KEY`.
pub fn check_key_id(id: u8) -> Result<(), String> {
    if (FIRST_KEY..=LAST_KEY).contains(&id) {
        Ok(())
    } else {
        Err(format!(
            "key {id} does not exist: keys are Key {FIRST_KEY} to Key {LAST_KEY}"
        ))
    }
}

/// `Ok` for an exit that is not locked or is locked by an existing key.
pub fn check_lock(locked_by: Option<u8>) -> Result<(), String> {
    locked_by.map_or(Ok(()), |id| {
        check_key_id(id).map_err(|error| format!("exit lock: {error}"))
    })
}

/// The label of a locked exit ("Key 3"), drawn on its band in the editor and in the game.
pub fn lock_label(locked_by: Option<u8>) -> Option<String> {
    locked_by.map(|id| Item::Key(id).name().to_owned())
}

/// What the notice says when the player touches an exit without its key.
pub fn lock_message(key: u8) -> String {
    format!("Locked: needs {}", Item::Key(key).name())
}

/// The number drawn on the marker of a key lying in a room ("3"); nothing for any other item.
pub fn key_marker_label(item: Item) -> Option<String> {
    match item {
        Item::Key(id) => Some(id.to_string()),
        _ => None,
    }
}

impl RoomDraft {
    /// Locks exit `index` with key `locked_by`, or unlocks it with `None`. Refused, with the
    /// draft unchanged, for a missing exit or a key outside 1..=8. Where the exit leads, and
    /// where it is, stay as they are.
    pub fn set_exit_lock(&mut self, index: usize, locked_by: Option<u8>) -> Result<(), String> {
        check_lock(locked_by)?;
        self.layout
            .exits
            .get_mut(index)
            .ok_or("missing exit")?
            .locked_by = locked_by;
        Ok(())
    }
}

/// The gap between a locked exit's band and its label.
pub const LABEL_GAP: f32 = 4.0;

/// Where the label of a locked exit goes, as the top-left corner and size of the text's box in
/// the coordinates of `zone` (the exit's band, or a door's rectangle): a door's label sits in the
/// middle of the door, an edge exit's just inside the room, next to its band and centered on it.
/// `text` is the size of the drawn text. One rule for the game and the editor canvas.
pub fn label_rect(edge: RoomEdge, is_door: bool, zone: Rect, text: Vec2) -> Rect {
    let center = zone.center();
    let origin = if is_door {
        center - text / 2.0
    } else {
        match edge {
            RoomEdge::Left => Vec2::new(zone.right() + LABEL_GAP, center.y - text.y / 2.0),
            RoomEdge::Right => Vec2::new(zone.x - LABEL_GAP - text.x, center.y - text.y / 2.0),
            RoomEdge::Top => Vec2::new(center.x - text.x / 2.0, zone.bottom() + LABEL_GAP),
            RoomEdge::Bottom => Vec2::new(center.x - text.x / 2.0, zone.y - LABEL_GAP - text.y),
        }
    };
    Rect::new(origin.x, origin.y, text.x, text.y)
}

/// One exit, as its room file places it, and the key that locked it. Never a list index: the
/// editor can reorder and delete exits. A door has no edge or offset of its own, so its
/// rectangle tells it apart.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OpenedExit {
    room: RoomId,
    edge: RoomEdge,
    offset: f32,
    door: Option<Rect>,
    key: u8,
}

impl OpenedExit {
    /// The exit `exit` of `room` while it is still locked; `None` if it is not locked.
    fn of(room: RoomId, exit: &Exit) -> Option<Self> {
        Some(Self {
            room,
            edge: exit.edge,
            offset: exit.offset,
            door: exit.door,
            key: exit.locked_by?,
        })
    }

    fn matches(&self, exit: &Exit) -> bool {
        exit.edge == self.edge
            && exit.offset == self.offset
            && exit.door == self.door
            && exit.locked_by == Some(self.key)
    }
}

/// The exits opened with a key during the run, so that a room rebuilt from its file does not
/// lock them again. Like taken items they last for the run and are never written to disk, and
/// the run reset relocks everything (`Default`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenedExits {
    opened: Vec<OpenedExit>,
}

impl OpenedExits {
    /// Remembers that `exit` of room `room` was opened. `exit` is the exit as it was while it
    /// was locked; an exit that was not locked is not remembered.
    pub fn record(&mut self, room: RoomId, exit: &Exit) {
        if let Some(opened) = OpenedExit::of(room, exit) {
            if !self.opened.contains(&opened) {
                self.opened.push(opened);
            }
        }
    }

    pub fn len(&self) -> usize {
        self.opened.len()
    }

    pub fn is_empty(&self) -> bool {
        self.opened.is_empty()
    }

    /// Room `room` was just rebuilt from its file and `exits` is what the file places. Opens
    /// every exit of that room that was opened before and is still locked by the same key in the
    /// same place. An entry that matches no exit is forgotten: the author moved or deleted that
    /// exit, took its lock off or changed its key, so what stands there now is new.
    pub fn apply_to_rebuilt(&mut self, room: RoomId, exits: &mut [Exit]) {
        self.opened.retain(|opened| {
            if opened.room != room {
                return true;
            }
            match exits.iter_mut().find(|exit| opened.matches(exit)) {
                Some(exit) => {
                    exit.locked_by = None;
                    true
                }
                None => false,
            }
        });
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod editing_tests;
