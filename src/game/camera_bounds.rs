//! The rectangle the follow camera stays inside.
use macroquad::math::Rect;

use super::GameState;

impl GameState {
    /// The room's bounds widened on every side by the thickness of its boundary slabs when
    /// its look draws them. The slabs are the walls, ceiling and floor of the room and stand
    /// just outside the bounds, so a camera held inside the bounds themselves would never
    /// show them: the edge of a room as wide as the screen would be out of sight. With the
    /// boundary off the camera keeps to the bounds, as the edge is only a line there.
    pub fn camera_bounds(&self) -> Rect {
        let room = self.room();
        let look = self
            .terrain_styles()
            .resolve_or_default(&room.terrain_style);
        let thickness = look.boundary_thickness;
        let margin = if look.draw_boundary && thickness.is_finite() && thickness > 0.0 {
            thickness
        } else {
            0.0
        };
        let bounds = room.bounds;
        Rect::new(
            bounds.x - margin,
            bounds.y - margin,
            bounds.w + 2.0 * margin,
            bounds.h + 2.0 * margin,
        )
    }
}
