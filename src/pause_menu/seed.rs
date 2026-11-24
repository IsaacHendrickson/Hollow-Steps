//! The pause menu's seed line: "Seed 00000000000000ab" in a corner of the virtual screen while a
//! run is going, so the seed can be read off a paused run and typed into F4's fixed seed
//! (`docs/variety.md`). Pure text and placement; `hud::draw_pause_menu` draws it.
//!
//! The line sits in the top-left corner on every screen: the Settings panel is nearly as tall as
//! the screen, so the corner is the one place that never covers a button.
use macroquad::math::Vec2;

use crate::camera::{anchored, Anchor};
use crate::seed::format_seed;

/// The font size of the seed line. It is small on purpose: a corner note, not a button.
pub const SEED_FONT_SIZE: u16 = 14;
/// How far the line sits in from the corner.
pub const SEED_MARGIN: Vec2 = Vec2::new(8.0, 8.0);

/// The line for a run's seed: "Seed " and the 16 hex digits `parse_seed` reads back. `None`
/// unless a run is going and has a seed (the hub, a test room and a game with no hub have none).
pub fn seed_label(running: bool, seed: Option<u64>) -> Option<String> {
    if !running {
        return None;
    }
    seed.map(|seed| format!("Seed {}", format_seed(seed)))
}

/// The top-left corner of a seed line `size` big on a `view`-sized screen, in virtual-screen
/// coordinates, anchored to the top-left corner.
pub fn seed_origin(view: Vec2, size: Vec2) -> Vec2 {
    anchored(view, Anchor::TopLeft, size, SEED_MARGIN)
}

#[cfg(test)]
mod tests;
