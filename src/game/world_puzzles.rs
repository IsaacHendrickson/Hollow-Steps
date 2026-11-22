//! Puzzle pieces in play (`crate::puzzle`, `docs/puzzles.md`): what a run has broken and switched
//! on, kept apart from the rooms so that a room rebuilt from its file gets it back.
//!
//! A wall the nail broke, and a switch the nail turned on, stay that way for the rest of the run.
//! They are remembered by room and by the numbers of their rectangle, never by a place in a list
//! (the editor can reorder and delete pieces), the way `exit_lock::OpenedExits` remembers a
//! door. The editor's Save rebuilds a saved room from its file, and `restore_puzzles` then breaks
//! and switches on again what the run had; an entry that matches no piece any more (moved,
//! resized or deleted by the author) is forgotten. Nothing is written to disk, and the run reset
//! forgets everything. A wall that was hit but not yet broken is not remembered.
//!
//! A test room is a sandbox: its pieces start whole and off, nothing it does is remembered, and
//! ending it puts back the saved room with what the run had done to it.
// `Puzzles::strike` is called from `GameState::step` in the next commit (part 2, in play).
#![cfg_attr(not(test), allow(dead_code))]
use macroquad::math::Rect;

use super::GameState;
use crate::room::{Room, RoomId};

type Numbers = (f32, f32, f32, f32);

/// One piece, as its room file places it.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Spot {
    room: RoomId,
    rect: Numbers,
}

impl Spot {
    fn new(room: RoomId, rect: Rect) -> Self {
        Self {
            room,
            rect: (rect.x, rect.y, rect.w, rect.h),
        }
    }

    fn rect(&self) -> Rect {
        let (x, y, w, h) = self.rect;
        Rect::new(x, y, w, h)
    }
}

/// The walls broken during the run.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct BrokenWalls {
    spots: Vec<Spot>,
}

impl BrokenWalls {
    /// Remembers that the wall at `rect` in `room` broke.
    pub(super) fn record(&mut self, room: RoomId, rect: Rect) {
        self.spots.push(Spot::new(room, rect));
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.spots.len()
    }

    /// Room `room` was just rebuilt from its file, so `puzzles` is whole again: breaks every wall
    /// that was broken and is still placed in the same place, and forgets the entries of that
    /// room that match nothing.
    pub(super) fn apply_to_rebuilt(
        &mut self,
        room: RoomId,
        puzzles: &mut crate::puzzle::RoomPuzzles,
    ) {
        self.spots
            .retain(|spot| spot.room != room || puzzles.break_at(spot.rect()));
    }
}

/// The switches turned on during the run.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SwitchesOn {
    spots: Vec<Spot>,
}

impl SwitchesOn {
    /// Remembers that the switch at `rect` in `room` went on.
    pub(super) fn record(&mut self, room: RoomId, rect: Rect) {
        self.spots.push(Spot::new(room, rect));
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.spots.len()
    }

    /// Like `BrokenWalls::apply_to_rebuilt`, for switches.
    pub(super) fn apply_to_rebuilt(
        &mut self,
        room: RoomId,
        puzzles: &mut crate::puzzle::RoomPuzzles,
    ) {
        self.spots
            .retain(|spot| spot.room != room || puzzles.switch_on_at(spot.rect()));
    }
}

/// What the run has broken and switched on.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Puzzles {
    pub(super) broken: BrokenWalls,
    pub(super) on: SwitchesOn,
}

impl Puzzles {
    /// Swing number `swing` is active with `hitbox` in `room`: walls take a hit and switches go
    /// on, and when `remember` is set (never in a test room) the walls that broke and the
    /// switches that went on are remembered. Returns what to tell the player, if a switch went on.
    pub(super) fn strike(
        &mut self,
        room: &mut Room,
        hitbox: Rect,
        swing: u64,
        remember: bool,
    ) -> Option<String> {
        let struck = room.puzzles.strike(hitbox, swing);
        if remember {
            for rect in &struck.broken {
                self.broken.record(room.id, *rect);
            }
            for (rect, _) in &struck.switched_on {
                self.on.record(room.id, *rect);
            }
        }
        struck
            .switched_on
            .first()
            .map(|(_, id)| format!("Switch {id} on"))
    }
}

impl GameState {
    /// The rooms `ids` were just rebuilt from their files: breaks and switches on again what the
    /// run had broken and switched on in them.
    pub(super) fn restore_puzzles(&mut self, ids: &[RoomId]) {
        for room in self.rooms.iter_mut().filter(|room| ids.contains(&room.id)) {
            self.puzzles
                .broken
                .apply_to_rebuilt(room.id, &mut room.puzzles);
            self.puzzles.on.apply_to_rebuilt(room.id, &mut room.puzzles);
        }
    }
}

#[cfg(test)]
mod tests;
