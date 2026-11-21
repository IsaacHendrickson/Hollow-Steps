//! Pure hazard editing for the room draft: add one beside the spawn, move it, change its size
//! and numbers. Every edit is checked with `RoomLayout::validate_geometry` on a copy, so a
//! refused edit leaves the draft as it was. Deleting is `RoomDraft::delete(Selection::Hazard)`.
use macroquad::math::Vec2;

use crate::editor::{RoomDraft, RoomLayout, Selection};
use crate::hazard::HazardPlacement;

/// Half the player's width, which a new hazard keeps clear of the spawn point, plus a margin.
const SPAWN_CLEARANCE: f32 = 16.0 + 16.0;

impl RoomLayout {
    /// Moves hazard `index` so its center is at `center`, kept inside the room.
    pub(crate) fn move_hazard(&mut self, index: usize, center: Vec2) -> Result<(), String> {
        if !center.is_finite() {
            return Err("position must be finite".into());
        }
        let room = self.rect();
        let hazard = self.hazards.get_mut(index).ok_or("missing hazard")?;
        let (_, _, w, h) = hazard.rect;
        hazard.rect = (
            (center.x - w / 2.0).max(room.x).min(room.right() - w),
            (center.y - h / 2.0).max(room.y).min(room.bottom() - h),
            w,
            h,
        );
        Ok(())
    }
}

impl RoomDraft {
    pub fn hazard(&self, index: usize) -> Option<&HazardPlacement> {
        self.layout.hazards.get(index)
    }

    /// Adds a standard hazard standing on the floor next to the spawn point (to its right, or
    /// to its left when the right side has no room) and returns its selection. Drag it where
    /// it belongs.
    pub fn add_hazard(&mut self) -> Result<Selection, String> {
        let (spawn_x, _) = self.layout.spawn;
        let (_, _, width, height) = HazardPlacement::standard(0.0, 0.0).rect;
        let top = self.layout.floor_y - height;
        let mut error = "no room beside the spawn point for a hazard".to_owned();
        for x in [spawn_x + SPAWN_CLEARANCE, spawn_x - SPAWN_CLEARANCE - width] {
            let mut layout = self.layout.clone();
            layout.hazards.push(HazardPlacement::standard(x, top));
            match layout.validate_geometry() {
                Ok(()) => {
                    self.layout = layout;
                    return Ok(Selection::Hazard(self.layout.hazards.len() - 1));
                }
                Err(refused) => error = refused,
            }
        }
        Err(error)
    }

    /// Replaces hazard `index` with `placement` (a size or number edit), if the layout stays
    /// valid.
    pub fn replace_hazard(
        &mut self,
        index: usize,
        placement: HazardPlacement,
    ) -> Result<(), String> {
        let mut layout = self.layout.clone();
        *layout.hazards.get_mut(index).ok_or("missing hazard")? = placement;
        layout.validate_geometry()?;
        self.layout = layout;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
