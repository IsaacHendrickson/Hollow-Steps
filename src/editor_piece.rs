//! Pure editing of a room's dungeon-piece mark and pickup slots (`crate::piece`) in the draft:
//! mark or unmark the room, add a slot beside the spawn, move it, change its kind. Every edit is
//! checked with `RoomLayout::validate_geometry` on a copy, so a refused edit leaves the draft as
//! it was. Deleting is `RoomDraft::delete(Selection::Slot)`; moving is `RoomDraft::move_to`.
use macroquad::math::Vec2;

use crate::editor::{RoomDraft, RoomLayout, Selection};
use crate::item::ITEM_SIZE;
use crate::piece::{PickupSlot, PieceError, PieceInfo, SlotKind, MAX_SLOTS};

/// Half the player's width, which a new slot keeps clear of the spawn point, plus a margin.
const SPAWN_CLEARANCE: f32 = 16.0 + 16.0;
/// How far a new slot steps along the floor, away from the spawn, to find a free spot: two
/// items wide, so the next try never overlaps the last.
const STEP: f32 = ITEM_SIZE.x * 2.0;

impl RoomLayout {
    /// Moves slot `index` so its center is at `center`; the layout check says whether it may stay
    /// there.
    pub(crate) fn move_slot(&mut self, index: usize, center: Vec2) -> Result<(), String> {
        if !center.is_finite() {
            return Err("position must be finite".into());
        }
        self.slots
            .get_mut(index)
            .ok_or("missing pickup slot")?
            .position = center.into();
        Ok(())
    }
}

impl RoomDraft {
    pub fn slot(&self, index: usize) -> Option<&PickupSlot> {
        self.layout.slots.get(index)
    }

    /// Marks the room as a piece (or, with `None`, takes the mark off; the slots stay). A weight
    /// outside the allowed range is refused and the draft stays as it was.
    pub fn set_piece(&mut self, piece: Option<PieceInfo>) -> Result<(), String> {
        let mut layout = self.layout.clone();
        layout.piece = piece;
        layout.validate_geometry()?;
        self.layout = layout;
        Ok(())
    }

    /// Adds an `Any` slot standing on the floor next to the spawn point (to its right, then its
    /// left, then further out along the floor until a spot is free) and returns its selection.
    /// Drag it where it belongs.
    pub fn add_slot(&mut self) -> Result<Selection, String> {
        let count = self.layout.slots.len();
        if count >= MAX_SLOTS {
            return Err(PieceError::TooManySlots { count: count + 1 }.to_string());
        }
        let (spawn_x, _) = self.layout.spawn;
        let y = self.layout.floor_y - ITEM_SIZE.y / 2.0;
        let mut offset = SPAWN_CLEARANCE;
        while offset <= self.layout.bounds.2 {
            for x in [spawn_x + offset, spawn_x - offset] {
                let mut layout = self.layout.clone();
                layout.slots.push(PickupSlot::new((x, y), SlotKind::Any));
                if layout.validate_geometry().is_ok() {
                    self.layout = layout;
                    return Ok(Selection::Slot(count));
                }
            }
            offset += STEP;
        }
        Err("no free spot on the floor for a pickup slot".into())
    }

    /// Changes what slot `index` may hold.
    pub fn set_slot_kind(&mut self, index: usize, kind: SlotKind) -> Result<(), String> {
        self.layout
            .slots
            .get_mut(index)
            .ok_or("missing pickup slot")?
            .kind = kind;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
