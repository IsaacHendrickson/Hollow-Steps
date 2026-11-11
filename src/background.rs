//! Parallax background layers: procedural shapes behind a room's terrain.
//!
//! Pure data and math, no drawing. A room file lists layers back to front in its `layout`
//! (`background`); each layer is a color, a depth factor and a few shapes. The front end
//! draws them first, through the same camera as the world, each moved by
//! `parallax_offset` so far layers slide less than near ones. No art files are involved.
//!
//! Shapes are placed in room coordinates, as they appear when the camera rests against the
//! room's top-left corner. At any other camera position a layer is shifted by its offset, so
//! `depth` 1 stays pinned to the world and `depth` 0 stays pinned to the screen.

use macroquad::math::{Rect, Vec2};
use serde::{Deserialize, Serialize};

/// Most layers a room may have.
pub const MAX_LAYERS: usize = 8;
/// Most shapes a layer may have.
pub const MAX_SHAPES: usize = 64;
/// Largest absolute coordinate or size a shape may use, in pixels.
pub const MAX_EXTENT: f32 = 100_000.0;

/// One procedural shape, in room coordinates.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Shape {
    /// A rectangle whose top-left corner is `(x, y)`.
    Rect { x: f32, y: f32, w: f32, h: f32 },
    /// A filled circle around `(x, y)`.
    Circle { x: f32, y: f32, radius: f32 },
    /// A filled triangle through three points, e.g. a ridge or a spike.
    Triangle {
        a: (f32, f32),
        b: (f32, f32),
        c: (f32, f32),
    },
}

/// One background layer. Layers are drawn in the order they are listed, first at the back.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackgroundLayer {
    /// 0 stays fixed on the screen, 1 moves with the world, in between slides slower than
    /// the world (farther away).
    pub depth: f32,
    /// Red, green, blue and alpha, 0 to 255.
    pub color: (u8, u8, u8, u8),
    pub shapes: Vec<Shape>,
}

impl Shape {
    fn validate(&self) -> Result<(), String> {
        let in_reach = |value: f32| value.is_finite() && value.abs() <= MAX_EXTENT;
        let sized = |value: f32| in_reach(value) && value > 0.0;
        let ok = match *self {
            Self::Rect { x, y, w, h } => in_reach(x) && in_reach(y) && sized(w) && sized(h),
            Self::Circle { x, y, radius } => in_reach(x) && in_reach(y) && sized(radius),
            Self::Triangle { a, b, c } => [a, b, c]
                .iter()
                .all(|point| in_reach(point.0) && in_reach(point.1)),
        };
        if ok {
            Ok(())
        } else {
            Err(format!(
                "must be finite, within {MAX_EXTENT} pixels, with a positive size"
            ))
        }
    }
}

impl BackgroundLayer {
    pub fn validate(&self) -> Result<(), String> {
        if !self.depth.is_finite() || !(0.0..=1.0).contains(&self.depth) {
            return Err("depth must be between 0 and 1".into());
        }
        if self.shapes.is_empty() {
            return Err("needs at least one shape".into());
        }
        if self.shapes.len() > MAX_SHAPES {
            return Err(format!("has more than {MAX_SHAPES} shapes"));
        }
        for (index, shape) in self.shapes.iter().enumerate() {
            shape
                .validate()
                .map_err(|error| format!("shape {index} {error}"))?;
        }
        Ok(())
    }
}

/// Checks a room's layers, naming the first one that is wrong.
pub fn validate(layers: &[BackgroundLayer]) -> Result<(), String> {
    if layers.len() > MAX_LAYERS {
        return Err(format!("background has more than {MAX_LAYERS} layers"));
    }
    for (index, layer) in layers.iter().enumerate() {
        layer
            .validate()
            .map_err(|error| format!("background layer {index}: {error}"))?;
    }
    Ok(())
}

/// How far to move a layer from where its shapes were placed, so it parallaxes.
///
/// `view_top_left` is the top-left of what the camera shows (before any screen shake, so the
/// shake moves every layer together), `bounds` the room and `view` the size of the screen.
/// The camera's travel from the room's top-left is clamped to what it can actually travel in
/// this room, so the offset never grows past the room's edges. On an axis where the room is
/// no larger than the view the camera cannot travel, and the offset is zero.
///
/// With `depth` 1 the offset is zero (the layer moves with the world); with `depth` 0 it
/// equals the travel (the layer stays still on the screen). A depth outside 0 to 1 is
/// clamped, and a non-finite one counts as 1.
pub fn parallax_offset(view_top_left: Vec2, bounds: Rect, view: Vec2, depth: f32) -> Vec2 {
    let depth = if depth.is_finite() {
        depth.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let travel = |top_left: f32, start: f32, length: f32, view: f32| {
        let furthest = (length - view).max(0.0);
        if top_left.is_finite() && furthest.is_finite() {
            (top_left - start).clamp(0.0, furthest)
        } else {
            0.0
        }
    };
    Vec2::new(
        travel(view_top_left.x, bounds.x, bounds.w, view.x),
        travel(view_top_left.y, bounds.y, bounds.h, view.y),
    ) * (1.0 - depth)
}

#[cfg(test)]
mod tests;
