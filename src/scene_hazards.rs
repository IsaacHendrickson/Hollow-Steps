//! Macroquad drawing of a room's hazards as rows of spikes. The spike shapes are pure
//! (`hollow_steps::hazard::Hazard::spike_triangles`); the editor canvas draws a plain marker.
use hollow_steps::hazard::Hazard;
use macroquad::prelude::*;

const SPIKE_FILL: Color = Color::new(0.80, 0.22, 0.38, 1.0);
const SPIKE_EDGE: Color = Color::new(1.0, 0.62, 0.70, 1.0);

/// Draws every hazard of the room; call with the world camera set.
pub fn draw_hazards(hazards: &[Hazard]) {
    for hazard in hazards {
        for [left, tip, right] in hazard.spike_triangles() {
            draw_triangle(left, tip, right, SPIKE_FILL);
            draw_triangle_lines(left, tip, right, 1.0, SPIKE_EDGE);
        }
    }
}
