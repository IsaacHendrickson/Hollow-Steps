//! Macroquad drawing of a room's hazards. What each one looks like is pure
//! (`hollow_steps::hazard::Hazard::look`, from `hazard_look`); this paints that draw list with
//! the terrain's adapter, in the world and, through `editor_visuals`, on the editor canvas.
use hollow_steps::hazard::Hazard;
use macroquad::prelude::*;

use crate::terrain_draw;

/// Draws every hazard of the room; call with the world camera set. `view` is the world
/// rectangle the camera shows and `time` is seconds (only lava moves with it).
pub fn draw_hazards(hazards: &[Hazard], view: Rect, time: f32) {
    for hazard in hazards {
        terrain_draw::draw_shapes(&hazard.look(time), view);
    }
}
