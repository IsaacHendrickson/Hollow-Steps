//! Macroquad drawing of a locked exit's key number on its band, in the game. The text and where
//! it goes are pure (`hollow_steps::exit_lock`); the editor canvas draws the same label.
use hollow_steps::exit_lock::{label_rect, lock_label};
use hollow_steps::room::Exit;
use macroquad::prelude::*;

const LABEL_SIZE: u16 = 20;
const LOCK_COLOR: Color = Color::new(0.96, 0.78, 0.30, 1.0);

/// Outlines a locked exit's band (`zone`, in room coordinates) and writes its key next to it.
/// Does nothing for an exit that is not locked.
pub fn draw_lock(exit: &Exit, zone: Rect) {
    let Some(label) = lock_label(exit.locked_by) else {
        return;
    };
    let measured = measure_text(&label, None, LABEL_SIZE, 1.0);
    let text = vec2(measured.width, f32::from(LABEL_SIZE));
    let at = label_rect(exit.edge, exit.is_door(), zone, text);
    draw_rectangle_lines(zone.x, zone.y, zone.w, zone.h, 3.0, LOCK_COLOR);
    // `at` is the text's box; the baseline is the text's bottom less its descent.
    draw_text(
        &label,
        at.x,
        at.y + measured.offset_y,
        f32::from(LABEL_SIZE),
        LOCK_COLOR,
    );
}
