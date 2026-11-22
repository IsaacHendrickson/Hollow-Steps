//! Macroquad drawing of a room's puzzle pieces. What each one looks like is a pure draw list
//! (`terrain_style::Shape`, the same list the terrain and hazards use), built here from the
//! piece's rectangle and state, and painted with the terrain's adapter. The looks are fixed for
//! now: everything is a fraction of the piece's own rectangle, so nothing depends on the window.
//!
//! - A breakable wall is a cracked stone block with one crack line for each hit it has left.
//! - A switch is a lever plate: the lever leans one way with a grey knob while it is off, and the
//!   other way with a lit knob in the switch's tint while it is on.
//! - A barrier is a bar block tinted like its switch's number, so the pair can be matched by eye.
//!
//! A broken wall and an open barrier are not drawn at all. Drawing never changes collision.
use hollow_steps::puzzle::{RoomPuzzles, FIRST_SWITCH};
use hollow_steps::terrain_style::{Point, Rgb, Rgba, Shape};
use macroquad::prelude::Rect;

use crate::terrain_draw;

/// The tint of each switch number, `FIRST_SWITCH` to `LAST_SWITCH`: red, orange, yellow, green,
/// teal, blue, violet, pink.
const TINTS: [Rgb; 8] = [
    (0.90, 0.30, 0.30),
    (0.95, 0.60, 0.20),
    (0.95, 0.85, 0.25),
    (0.35, 0.80, 0.35),
    (0.25, 0.75, 0.75),
    (0.30, 0.50, 0.95),
    (0.65, 0.40, 0.90),
    (0.95, 0.50, 0.75),
];
const STONE: Rgb = (0.36, 0.33, 0.30);
const STONE_LIT: Rgb = (0.52, 0.48, 0.43);
const STONE_SHADE: Rgb = (0.24, 0.22, 0.20);
const CRACK: Rgb = (0.08, 0.07, 0.06);
const PLATE: Rgb = (0.22, 0.24, 0.30);
const LEVER: Rgb = (0.70, 0.72, 0.78);
const KNOB_OFF: Rgb = (0.42, 0.43, 0.47);
const IRON: Rgb = (0.17, 0.18, 0.22);

/// The edge strips of a block are this fraction of its smaller side, and no more than `RIM_MAX`
/// world units, so a big wall does not get a huge rim.
const RIM_FRACTION: f32 = 0.12;
const RIM_MAX: f32 = 4.0;
const CRACK_THICKNESS: f32 = 2.0;
const LEVER_THICKNESS: f32 = 3.0;

/// The tint of switch `id` (and of the barriers it opens). A number outside 1 to 8 wraps round,
/// so a file the checks would refuse still draws something.
pub(crate) fn switch_tint(id: u8) -> Rgb {
    TINTS[usize::from(id.saturating_sub(FIRST_SWITCH)) % TINTS.len()]
}

fn rim(rect: Rect) -> f32 {
    (rect.w.min(rect.h) * RIM_FRACTION).min(RIM_MAX)
}

fn fill(rect: Rect, color: Rgb) -> Shape {
    Shape::rect(rect.x, rect.y, rect.w, rect.h, Rgba::opaque(color))
}

/// A cracked stone block: lit top, shaded bottom, and `remaining` crack lines, evenly spread
/// across it and leaning alternately one way and the other.
pub(crate) fn breakable_shapes(rect: Rect, remaining: u8) -> Vec<Shape> {
    let edge = rim(rect);
    let mut shapes = vec![
        fill(rect, STONE),
        fill(Rect::new(rect.x, rect.y, rect.w, edge), STONE_LIT),
        fill(
            Rect::new(rect.x, rect.bottom() - edge, rect.w, edge),
            STONE_SHADE,
        ),
    ];
    let count = f32::from(remaining);
    for crack in 0..remaining {
        let along = rect.x + rect.w * (f32::from(crack) + 1.0) / (count + 1.0);
        let lean = if crack % 2 == 0 { 1.0 } else { -1.0 } * rect.w * 0.06;
        let (top, bottom): (Point, Point) = (
            (along - lean, rect.y + rect.h * 0.1),
            (along + lean, rect.y + rect.h * 0.9),
        );
        shapes.push(Shape::line(
            top,
            bottom,
            CRACK_THICKNESS,
            Rgba::opaque(CRACK),
        ));
    }
    shapes
}

/// A lever plate: a base with a stripe in the switch's tint, and a lever with a knob. Off, the
/// lever leans left and the knob is grey; on, it leans right and the knob is the tint, lit.
pub(crate) fn switch_shapes(rect: Rect, id: u8, on: bool) -> Vec<Shape> {
    let tint = switch_tint(id);
    let base = Rect::new(rect.x, rect.y + rect.h * 0.6, rect.w, rect.h * 0.4);
    let stripe = Rect::new(base.x, base.y, base.w, base.h * 0.3);
    let pivot: Point = (rect.center().x, base.y);
    let lean = if on { 0.3 } else { -0.3 };
    let tip: Point = (pivot.0 + rect.w * lean, rect.y + rect.h * 0.15);
    let size = rect.w.min(rect.h) * 0.25;
    let knob = Rect::new(tip.0 - size / 2.0, tip.1 - size / 2.0, size, size);
    vec![
        fill(base, PLATE),
        fill(stripe, tint),
        Shape::line(pivot, tip, LEVER_THICKNESS, Rgba::opaque(LEVER)),
        fill(knob, if on { tint } else { KNOB_OFF }),
    ]
}

/// A bar block: an iron body, a cap at each end and upright bars, in the tint of its switch.
/// The bars are about one for each 8 units of width, at least two.
pub(crate) fn barrier_shapes(rect: Rect, switch: u8) -> Vec<Shape> {
    let tint = switch_tint(switch);
    let edge = rim(rect);
    let bars = (rect.w / 8.0).floor().max(2.0);
    let slot = rect.w / bars;
    let mut shapes = vec![
        fill(rect, IRON),
        fill(Rect::new(rect.x, rect.y, rect.w, edge), tint),
        fill(Rect::new(rect.x, rect.bottom() - edge, rect.w, edge), tint),
    ];
    for bar in 0..bars as u32 {
        let x = rect.x + slot * (bar as f32 + 0.5) - slot * 0.2;
        shapes.push(fill(
            Rect::new(x, rect.y + edge, slot * 0.4, rect.h - 2.0 * edge),
            tint,
        ));
    }
    shapes
}

/// Draws the pieces still in the way, and every switch. Call with the world camera set; `view`
/// is the world rectangle the camera shows.
pub fn draw_puzzles(puzzles: &RoomPuzzles, view: Rect) {
    for wall in puzzles
        .breakables()
        .iter()
        .filter(|wall| wall.remaining > 0)
    {
        terrain_draw::draw_shapes(&breakable_shapes(wall.rect, wall.remaining), view);
    }
    for switch in puzzles.switches() {
        terrain_draw::draw_shapes(&switch_shapes(switch.rect, switch.id, switch.on), view);
    }
    for barrier in puzzles
        .barriers()
        .iter()
        .filter(|barrier| !puzzles.is_open(barrier))
    {
        terrain_draw::draw_shapes(&barrier_shapes(barrier.rect, barrier.switch), view);
    }
}

#[cfg(test)]
mod tests;
