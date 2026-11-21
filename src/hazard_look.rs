//! How a hazard looks: its style, the way it faces, its color, and the draw list that follows.
//!
//! Pure data and math, no drawing and no art files. `shapes` turns a hazard's rectangle into
//! the same `terrain_style::Shape` list the terrain uses, so the game and the editor canvas
//! draw it with the same adapter. The look never reaches outside the rectangle and never
//! changes the hit: collision is `hazard`'s rectangle, whatever is drawn in it.
//!
//! A hazard is drawn on one *base edge*, the side opposite its `facing`, and everything
//! points or flows toward the facing side. So a tall, thin hazard against a wall facing away
//! from it is a row of spikes along the wall, where facing up would be one needle:
//!
//! - `Spikes`: a row of whole triangles about `SPIKE_WIDTH` wide, each in two tones.
//! - `Lava`: a body with a darker deep band, a bright crust along the facing edge whose waves
//!   move with `time`, and a few glowing specks.
//! - `Thorns`: a dark vine along the base edge with thin, leaning thorns of uneven height.
//!
//! Everything is deterministic: a thorn or speck draws from a stream seeded by its own number,
//! so resizing or moving a hazard never reshuffles the ones that are still there. Only lava
//! reads `time`; nothing reads the clock.
use macroquad::math::Rect;
use serde::{Deserialize, Serialize};

use crate::particles::Rng;
use crate::terrain_style::{Point, Rgb, Rgba, Shape};

/// How wide one drawn spike is, aiming for; the edge is divided into whole spikes.
pub const SPIKE_WIDTH: f32 = 16.0;
/// How far apart thorns are along the vine, aiming for.
pub const THORN_SPACING: f32 = 10.0;
/// The width of one slice of the lava surface.
pub const LAVA_SLICE: f32 = 12.0;
/// Most pieces of one kind a hazard draws, however long it is (the room is 2000 px at most).
const MAX_PIECES: usize = 256;
/// Most glowing specks in lava.
const MAX_SPECKS: usize = 32;
/// Lava gets one speck for this much area.
const AREA_PER_SPECK: f32 = 700.0;

const BLACK: Rgb = (0.0, 0.0, 0.0);
const WHITE: Rgb = (1.0, 1.0, 1.0);

/// What a hazard looks like.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardStyle {
    #[default]
    Spikes,
    Lava,
    Thorns,
}

impl HazardStyle {
    pub const ALL: [HazardStyle; 3] = [Self::Spikes, Self::Lava, Self::Thorns];

    pub fn label(self) -> &'static str {
        match self {
            Self::Spikes => "Spikes",
            Self::Lava => "Lava",
            Self::Thorns => "Thorns",
        }
    }

    /// The color a hazard of this style has until its author picks one.
    pub fn default_color(self) -> Rgb {
        match self {
            Self::Spikes => (0.66, 0.70, 0.78),
            Self::Lava => (0.96, 0.36, 0.07),
            Self::Thorns => (0.32, 0.58, 0.24),
        }
    }
}

/// The side a hazard's points or surface face: where the player is meant to touch it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HazardFacing {
    /// On the floor, pointing up.
    #[default]
    Up,
    /// On the ceiling, pointing down.
    Down,
    /// On a wall to the right, pointing left.
    Left,
    /// On a wall to the left, pointing right.
    Right,
}

impl HazardFacing {
    pub const ALL: [HazardFacing; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];

    pub fn label(self) -> &'static str {
        match self {
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }
}

/// The hazard's rectangle seen from its base edge: `u` runs along the base edge and `v`
/// stands out from it toward the facing side. The styles draw in `(u, v)` and this puts the
/// pieces in the world.
struct Frame {
    rect: Rect,
    facing: HazardFacing,
}

impl Frame {
    /// How long the base edge is.
    fn length(&self) -> f32 {
        match self.facing {
            HazardFacing::Up | HazardFacing::Down => self.rect.w,
            HazardFacing::Left | HazardFacing::Right => self.rect.h,
        }
    }

    /// How far the hazard stands out from its base edge.
    fn depth(&self) -> f32 {
        match self.facing {
            HazardFacing::Up | HazardFacing::Down => self.rect.h,
            HazardFacing::Left | HazardFacing::Right => self.rect.w,
        }
    }

    fn point(&self, u: f32, v: f32) -> Point {
        let r = self.rect;
        match self.facing {
            HazardFacing::Up => (r.x + u, r.bottom() - v),
            HazardFacing::Down => (r.x + u, r.y + v),
            HazardFacing::Left => (r.right() - v, r.y + u),
            HazardFacing::Right => (r.x + v, r.y + u),
        }
    }

    fn triangle(&self, corners: [(f32, f32); 3], color: Rgb) -> Shape {
        let [a, b, c] = corners.map(|(u, v)| self.point(u, v));
        Shape::triangle([a, b, c], Rgba::opaque(color))
    }

    fn quad(&self, corners: [(f32, f32); 4], color: Rgb) -> Shape {
        Shape::Polygon {
            points: corners.iter().map(|&(u, v)| self.point(u, v)).collect(),
            color: Rgba::opaque(color),
        }
    }

    /// The axis-aligned box from `(u0, v0)` to `(u1, v1)`.
    fn block(&self, (u0, v0): (f32, f32), (u1, v1): (f32, f32), color: Rgb) -> Shape {
        let (a, b) = (self.point(u0, v0), self.point(u1, v1));
        Shape::rect(
            a.0.min(b.0),
            a.1.min(b.1),
            (a.0 - b.0).abs(),
            (a.1 - b.1).abs(),
            Rgba::opaque(color),
        )
    }
}

/// The draw list of a hazard filling `rect`, in world coordinates. `color` is the base color
/// (the style's own tones are mixed from it); `time` is seconds, and only lava moves with it.
/// A rectangle that is empty or not finite draws nothing.
pub fn shapes(
    rect: Rect,
    style: HazardStyle,
    facing: HazardFacing,
    color: Rgb,
    time: f32,
) -> Vec<Shape> {
    let finite = [rect.x, rect.y, rect.w, rect.h]
        .iter()
        .all(|n| n.is_finite());
    if !finite || rect.w <= 0.0 || rect.h <= 0.0 {
        return Vec::new();
    }
    let frame = Frame { rect, facing };
    let color = (unit(color.0), unit(color.1), unit(color.2));
    let time = if time.is_finite() { time } else { 0.0 };
    match style {
        HazardStyle::Spikes => spikes(&frame, color),
        HazardStyle::Lava => lava(&frame, color, time),
        HazardStyle::Thorns => thorns(&frame, color),
    }
}

/// `value` kept between 0 and 1; not a number is 0.
fn unit(value: f32) -> f32 {
    if value.is_nan() {
        0.0
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

/// A stream for one numbered piece of one style, so a piece keeps its look when the others
/// around it come and go.
fn stream(salt: u64, index: usize) -> Rng {
    Rng::new(salt ^ (index as u64 + 1).wrapping_mul(0xA24B_AED4_963E_E407))
}

fn spikes(frame: &Frame, color: Rgb) -> Vec<Shape> {
    let (length, depth) = (frame.length(), frame.depth());
    let count = ((length / SPIKE_WIDTH).round() as usize).clamp(1, MAX_PIECES);
    let width = length / count as f32;
    let (shade, light) = (mix(color, BLACK, 0.45), mix(color, WHITE, 0.35));
    let mut shapes = vec![frame.block((0.0, 0.0), (length, depth.min(3.0) * 0.5), shade)];
    for i in 0..count {
        let (from, to) = (width * i as f32, width * (i + 1) as f32);
        let middle = (from + to) / 2.0;
        shapes.push(frame.triangle([(from, 0.0), (middle, depth), (to, 0.0)], color));
        // The lit half of the spike.
        shapes.push(frame.triangle([(from, 0.0), (middle, depth), (middle, 0.0)], light));
    }
    shapes
}

fn lava(frame: &Frame, color: Rgb, time: f32) -> Vec<Shape> {
    let (length, depth) = (frame.length(), frame.depth());
    let slices = ((length / LAVA_SLICE).ceil() as usize).clamp(1, MAX_PIECES);
    let amplitude = (depth * 0.15).min(3.0);
    let crust = (depth * 0.25).min(3.0);
    let (deep, hot) = (mix(color, BLACK, 0.35), mix(color, WHITE, 0.45));
    // The surface height at `u`: between `depth - amplitude` and `depth`, never past the rect.
    let surface = |u: f32| depth - amplitude * 0.5 * (1.0 + (time * 1.8 + u * 0.12).sin());
    let mut shapes = Vec::new();
    for i in 0..slices {
        let from = length * i as f32 / slices as f32;
        let to = length * (i + 1) as f32 / slices as f32;
        let (top_from, top_to) = (surface(from), surface(to));
        shapes.push(frame.quad(
            [(from, 0.0), (to, 0.0), (to, top_to), (from, top_from)],
            color,
        ));
        shapes.push(frame.block((from, 0.0), (to, depth * 0.4), deep));
        shapes.push(frame.quad(
            [
                (from, top_from - crust),
                (to, top_to - crust),
                (to, top_to),
                (from, top_from),
            ],
            hot,
        ));
    }
    if length >= 4.0 && depth >= 8.0 {
        let specks = ((length * depth / AREA_PER_SPECK) as usize).min(MAX_SPECKS);
        for i in 0..specks {
            let mut random = stream(0x1A7A, i);
            let u = 0.5 + random.unit() * (length - 3.0);
            let v = 1.0 + random.unit() * (depth * 0.6 - 4.0);
            shapes.push(frame.block((u, v), (u + 2.5, v + 2.5), hot));
        }
    }
    shapes
}

fn thorns(frame: &Frame, color: Rgb) -> Vec<Shape> {
    let (length, depth) = (frame.length(), frame.depth());
    let count = ((length / THORN_SPACING).round() as usize).clamp(1, MAX_PIECES);
    let spacing = length / count as f32;
    let vine = depth.min(3.0) * 0.8;
    let (dark, light) = (mix(color, BLACK, 0.5), mix(color, WHITE, 0.3));
    let half_base = (spacing * 0.35).min(3.0);
    let mut shapes = vec![frame.block((0.0, 0.0), (length, vine), dark)];
    for i in 0..count {
        let mut random = stream(0x7402, i);
        let middle = spacing * (i as f32 + 0.5);
        let height = depth * (0.55 + 0.45 * random.unit());
        // Thorns lean one way and the other in turn, by an uneven amount.
        let lean = (1.0 + random.unit() * 3.0) * if i % 2 == 0 { 1.0 } else { -1.0 };
        let tip = (middle + lean).clamp(0.0, length);
        let tone = if i % 3 == 2 { light } else { color };
        shapes.push(frame.triangle(
            [
                (middle - half_base, vine * 0.5),
                (middle + half_base, vine * 0.5),
                (tip, height),
            ],
            tone,
        ));
    }
    shapes
}

#[cfg(test)]
mod tests;
