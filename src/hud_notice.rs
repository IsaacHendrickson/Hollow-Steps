//! Macroquad drawing of the notice (`hollow_steps::notice`): a short message such as
//! "Locked: needs Key 3", at the top center of the virtual screen, only while it is showing.
use hollow_steps::camera::{anchored, Anchor};
use hollow_steps::game::GameState;
use macroquad::prelude::*;

use crate::hud::centered_baseline;

const FONT_SIZE: u16 = 26;
const PADDING: Vec2 = Vec2::new(18.0, 10.0);
/// How far below the top edge the notice hangs, clear of the room title at the right.
const TOP_OFFSET: f32 = 92.0;
const WARNING: Color = Color::new(0.94, 0.78, 0.47, 1.0);

pub fn draw_notice(game: &GameState, view: Vec2) {
    let Some(text) = game.notice() else {
        return;
    };
    let width = measure_text(text, None, FONT_SIZE, 1.0).width;
    let panel = vec2(width, f32::from(FONT_SIZE)) + PADDING * 2.0;
    let at = anchored(view, Anchor::TopCenter, panel, vec2(0.0, TOP_OFFSET));
    draw_rectangle(
        at.x,
        at.y,
        panel.x,
        panel.y,
        Color::from_rgba(22, 24, 40, 230),
    );
    draw_rectangle_lines(at.x, at.y, panel.x, panel.y, 2.0, WARNING);
    draw_text(
        text,
        at.x + PADDING.x,
        centered_baseline(at.y + panel.y / 2.0, FONT_SIZE),
        f32::from(FONT_SIZE),
        WARNING,
    );
}
