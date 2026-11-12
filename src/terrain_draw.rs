//! Macroquad adapter for the terrain look: draws a `terrain_style` draw list in world space.
//! Call with the world camera set. Shapes outside the view are skipped.

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
    let visible = Rect::new(
        view.x - CULL_MARGIN,
        view.y - CULL_MARGIN,
        view.w + 2.0 * CULL_MARGIN,
        view.h + 2.0 * CULL_MARGIN,
    );
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
            } => draw_rectangle(*x, *y, *w, *h, color(*c)),
            Shape::Line {
                from,
                to,
                thickness,
                color: c,
            } => draw_line(from.0, from.1, to.0, to.1, *thickness, color(*c)),
            Shape::Triangle { points, color: c } => draw_triangle(
                vec2(points[0].0, points[0].1),
                vec2(points[1].0, points[1].1),
                vec2(points[2].0, points[2].1),
                color(*c),
            ),
            Shape::Polygon { points, color: c } => {
                // Convex, so a fan from the first point fills it exactly.
                for pair in points.windows(2).skip(1) {
                    draw_triangle(
                        vec2(points[0].0, points[0].1),
                        vec2(pair[0].0, pair[0].1),
                        vec2(pair[1].0, pair[1].1),
                        color(*c),
                    );
                }
            }
        }
    }
}
