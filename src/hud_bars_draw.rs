//! Macroquad drawing of the health and mana bars from the rectangles `hud_bars::layout_bars`
//! returns. All the rules (where the base part, the added capacity, the locked end and the
//! divider go) are in `hollow_steps::hud_bars`; this only fills them.

use hollow_steps::hud_bars::{BarColors, BarLayout, Rgba};
use macroquad::prelude::*;

/// Fills `rect` with `color`; a part with nothing in it (zero width) draws nothing.
fn fill(rect: Rect, color: Rgba) {
    if rect.w > 0.0 && rect.h > 0.0 {
        let [red, green, blue, alpha] = color;
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::from_rgba(red, green, blue, alpha),
        );
    }
}

/// The bars, back to front: the empty health bar (the added capacity has the same empty and
/// fill colors, so it is part of it), the trailing bar, the pending heal and the current health,
/// then the greyed locked end and the divider at the start of added capacity; then the same for
/// the mana bar.
pub fn draw_bars(layout: &BarLayout, colors: &BarColors) {
    fill(layout.empty, colors.empty);
    fill(layout.trailing, colors.trailing);
    fill(layout.pending, colors.pending);
    fill(layout.health, colors.health);
    fill(layout.health_locked, colors.locked);
    fill(layout.health_edge, colors.extension_edge);
    fill(layout.mana_empty, colors.mana_empty);
    fill(layout.mana, colors.mana);
    fill(layout.mana_locked, colors.locked);
    fill(layout.mana_edge, colors.extension_edge);
}
