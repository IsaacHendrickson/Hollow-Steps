//! Macroquad adapter for the terrain look: draws a `terrain_style` draw list. `draw_shapes`
//! draws in world space (call it with the world camera set); `draw_shapes_mapped` draws the
//! same list through a world-to-screen mapping, which is how the room editor's canvas, with its
//! own zoom and pan, shows it. Shapes outside the view are skipped.

use hollow_steps::terrain_style::{Rgba, Shape};
use macroquad::prelude::*;

/// Shapes this far outside the view are still drawn, so screen shake and the 1 px lines never
/// show an edge.
const CULL_MARGIN: f32 = 48.0;

fn color(color: Rgba) -> Color {
    Color::new(color.r, color.g, color.b, color.a)
}

/// Draws the shapes in order. `view` is the world rectangle the camera shows.
pub fn draw_shapes(shapes: &[Shape], view: Rect) {
    draw_shapes_mapped(shapes, view, 1.0, |point| point);
}

/// Draws the shapes in order through `screen`, which maps a world point to the point it is
/// drawn at, with line thickness multiplied by `scale` (the zoom of that mapping). `view` is the
/// world rectangle that is on screen; the rest is skipped.
pub fn draw_shapes_mapped(shapes: &[Shape], view: Rect, scale: f32, screen: impl Fn(Vec2) -> Vec2) {
    let visible = Rect::new(
        view.x - CULL_MARGIN,
        view.y - CULL_MARGIN,
        view.w + 2.0 * CULL_MARGIN,
        view.h + 2.0 * CULL_MARGIN,
    );
    let at = |point: &(f32, f32)| screen(vec2(point.0, point.1));
    for shape in shapes {
        let (low, high) = shape.bounds();
        if high.0 < visible.x
            || low.0 > visible.right()
            || high.1 < visible.y
            || low.1 > visible.bottom()
        {
            continue;
        }
        match shape {
            Shape::Rect {
                x,
                y,
                w,
                h,
                color: c,
            } => {
                // A rectangle stays a rectangle under a zoom and a pan, so two corners do it.
                let from = screen(vec2(*x, *y));
                let to = screen(vec2(x + w, y + h));
                draw_rectangle(from.x, from.y, to.x - from.x, to.y - from.y, color(*c));
            }
            Shape::Line {
                from,
                to,
                thickness,
                color: c,
            } => {
                let (from, to) = (at(from), at(to));
                draw_line(from.x, from.y, to.x, to.y, thickness * scale, color(*c));
            }
            Shape::Triangle { points, color: c } => {
                draw_triangle(at(&points[0]), at(&points[1]), at(&points[2]), color(*c));
            }
            Shape::Polygon { points, color: c } => {
                // Convex, so a fan from the first point fills it exactly.
                for pair in points.windows(2).skip(1) {
                    draw_triangle(at(&points[0]), at(&pair[0]), at(&pair[1]), color(*c));
                }
            }
        }
    }
}
