//! The procedural look of a room's solids: a draw list of simple shapes.
//!
//! Pure data and math, no drawing and no art files. `build` turns solids into `Shape`s
//! (rectangles, lines, triangles and convex polygons with colors) that the front end draws in
//! world space, so the collision rectangles are never touched: this only decides how they
//! look. Each solid gets
//!
//! - a base fill with a subtle vertical gradient in three bands (a lighter band under an
//!   exposed top, the palette fill, a darker band over an exposed bottom),
//! - speckles and short cracks inside the fill,
//! - along its *exposed* edges only (see `edges`): a dark outline, and on an exposed top a lit
//!   strip, a jittered stone lip and optional moss tufts; under an exposed bottom, stalactites.
//!
//! Everything is deterministic. A solid's seed comes from the room, the solid's own position
//! (not its place in the list) and the palette's `seed_offset`, and each feature draws from its
//! own stream, so reordering, adding or removing other solids never changes how this one
//! looks, and neither does toggling moss. Moving a solid gives it the look of its new spot.
//! The generator is `particles::Rng`; nothing reads the clock.
//!
//! The list never reaches outside a solid by more than `TOP_MARGIN` above, `BOTTOM_MARGIN`
//! below and nothing at the sides (lines count half their thickness).
//!
//! `TerrainLook` keeps the last list so the front end rebuilds it only when the room, its
//! solids, the ground or the palette change.

use macroquad::math::Rect;

use crate::particles::Rng;
use crate::room::RoomId;

mod colors;
mod edges;
mod rock;

pub use edges::{exposure, Edge, EdgeSpan, Exposure};

#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod limit_tests;
#[cfg(test)]
mod tests;

/// Thickness of the dark outline along an exposed edge, in pixels (inside the solid).
pub const OUTLINE_WIDTH: f32 = 2.0;
/// Height of the lit strip along an exposed top, inside the solid.
pub const HIGHLIGHT_HEIGHT: f32 = 2.0;
/// Most the rocky lip rises above an exposed top.
pub const LIP_HEIGHT: f32 = 3.0;
/// Most a moss blade rises above an exposed top.
pub const MOSS_HEIGHT: f32 = 6.0;
/// Most a stalactite hangs below an exposed bottom.
pub const DRIP_LENGTH: f32 = 12.0;
/// How far the draw list may extend above a solid (moss is the tallest decoration).
pub const TOP_MARGIN: f32 = MOSS_HEIGHT;
/// How far the draw list may extend below a solid.
pub const BOTTOM_MARGIN: f32 = DRIP_LENGTH;
/// Most speckles and cracks one solid gets, however large it is.
pub const MAX_SPECKLES: usize = 256;
pub const MAX_CRACKS: usize = 16;
/// How far below the floor line the ground cover reaches, so a solid standing on the floor
/// counts as buried there.
const GROUND_DEPTH: f32 = 64.0;

/// Red, green and blue, each 0 to 1.
pub type Rgb = (f32, f32, f32);
pub type Point = (f32, f32);

/// The look when a room has no palette of its own: the dark blue-grey of the original flat
/// terrain, its pale blue rim as the lit top, a near-black edge and a mossy green accent.
pub const DEFAULT_FILL: Rgb = (0.133, 0.145, 0.227);
pub const DEFAULT_EDGE: Rgb = (0.051, 0.055, 0.102);
pub const DEFAULT_HIGHLIGHT: Rgb = (0.435, 0.478, 0.643);
pub const DEFAULT_ACCENT: Rgb = (0.286, 0.545, 0.443);

/// A color with transparency, each part 0 to 1.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Rgba {
    pub fn new(rgb: Rgb, a: f32) -> Self {
        Self {
            r: rgb.0,
            g: rgb.1,
            b: rgb.2,
            a,
        }
    }

    pub fn opaque(rgb: Rgb) -> Self {
        Self::new(rgb, 1.0)
    }
}

/// One simple shape of a draw list, in world coordinates.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: Rgba,
    },
    Line {
        from: Point,
        to: Point,
        thickness: f32,
        color: Rgba,
    },
    Triangle {
        points: [Point; 3],
        color: Rgba,
    },
    /// A convex polygon, drawn as a triangle fan from its first point.
    Polygon {
        points: Vec<Point>,
        color: Rgba,
    },
}

impl Shape {
    pub fn rect(x: f32, y: f32, w: f32, h: f32, color: Rgba) -> Self {
        Self::Rect { x, y, w, h, color }
    }

    pub fn line(from: Point, to: Point, thickness: f32, color: Rgba) -> Self {
        Self::Line {
            from,
            to,
            thickness,
            color,
        }
    }

    pub fn triangle(points: [Point; 3], color: Rgba) -> Self {
        Self::Triangle { points, color }
    }

    /// The smallest box around the shape, as `(top-left, bottom-right)`. A line counts half
    /// its thickness on every side.
    pub fn bounds(&self) -> (Point, Point) {
        let around = |points: &[Point], pad: f32| {
            let mut low = (f32::MAX, f32::MAX);
            let mut high = (f32::MIN, f32::MIN);
            for point in points {
                low = (low.0.min(point.0), low.1.min(point.1));
                high = (high.0.max(point.0), high.1.max(point.1));
            }
            if points.is_empty() {
                return ((0.0, 0.0), (0.0, 0.0));
            }
            ((low.0 - pad, low.1 - pad), (high.0 + pad, high.1 + pad))
        };
        match self {
            Self::Rect { x, y, w, h, .. } => ((*x, *y), (x + w, y + h)),
            Self::Line {
                from,
                to,
                thickness,
                ..
            } => around(&[*from, *to], thickness / 2.0),
            Self::Triangle { points, .. } => around(points, 0.0),
            Self::Polygon { points, .. } => around(points, 0.0),
        }
    }
}

/// A room's terrain palette and switches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainStyle {
    /// The middle band of every solid; the other bands are derived from it.
    pub fill: Rgb,
    /// The dark outline along exposed edges.
    pub edge: Rgb,
    /// The lit strip along an exposed top; the lip is a blend of it and the fill.
    pub highlight: Rgb,
    /// Moss tufts and a share of the speckles.
    pub accent: Rgb,
    /// Mixed into every solid's seed: change it to reshuffle a room's speckles and moss.
    pub seed_offset: u32,
    /// Whether moss grows on exposed tops.
    pub moss: bool,
}

impl Default for TerrainStyle {
    fn default() -> Self {
        Self {
            fill: DEFAULT_FILL,
            edge: DEFAULT_EDGE,
            highlight: DEFAULT_HIGHLIGHT,
            accent: DEFAULT_ACCENT,
            seed_offset: 0,
            moss: true,
        }
    }
}

/// Folds `value` into `seed` (SplitMix64 finalizer over a hash combine).
fn mix(seed: u64, value: u64) -> u64 {
    let mut z = seed
        ^ value
            .wrapping_add(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(seed << 6)
            .wrapping_add(seed >> 2);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// A position in sixteenths of a pixel, so float noise below that never changes a look.
/// Not-a-number counts as 0.
fn quantize(value: f32) -> u64 {
    (value * 16.0).round() as i64 as u64
}

/// The seed of one solid: its room, its own position and the palette's offset.
pub fn solid_seed(room: RoomId, solid: &Rect, seed_offset: u32) -> u64 {
    let seed = mix(0x7E44_A1CE, room.index() as u64);
    let seed = mix(seed, quantize(solid.x));
    let seed = mix(seed, quantize(solid.y));
    mix(seed, u64::from(seed_offset))
}

/// An independent generator for one feature of a solid (`tag`), optionally keyed by where
/// along an edge it works (`salt`).
fn stream(seed: u64, tag: u64, salt: f32) -> Rng {
    Rng::new(mix(mix(seed, tag), quantize(salt)))
}

/// The rectangle that counts as ground when working out which edges are exposed: everything
/// from the floor line down. A solid standing on the floor has no exposed bottom.
pub fn ground_cover(bounds: Rect, floor_y: f32) -> Rect {
    Rect::new(
        bounds.x,
        floor_y,
        bounds.w,
        (bounds.bottom() - floor_y).max(0.0) + GROUND_DEPTH,
    )
}

/// A solid worth drawing: finite and with a positive size.
fn drawable(solid: &Rect) -> bool {
    [solid.x, solid.y, solid.w, solid.h]
        .iter()
        .all(|value| value.is_finite())
        && solid.w > 0.0
        && solid.h > 0.0
}

/// The draw list for a room's solids, back to front. All fills come first and all trims
/// after, so a neighbor's fill never covers another solid's outline, moss or stalactites.
/// `ground` (see `ground_cover`) counts as solid when working out which edges are exposed
/// but is not drawn. No solids give an empty list.
pub fn build(
    room: RoomId,
    solids: &[Rect],
    ground: Option<Rect>,
    style: &TerrainStyle,
) -> Vec<Shape> {
    let colors = colors::Colors::from_style(style);
    let cover: Vec<Rect> = ground.into_iter().collect();
    let mut fills = Vec::new();
    let mut trims = Vec::new();
    for (index, solid) in solids.iter().enumerate().filter(|(_, s)| drawable(s)) {
        let seed = solid_seed(room, solid, style.seed_offset);
        let open = exposure(solids, index, &cover);
        rock::fill_layer(*solid, &open, seed, &colors, &mut fills);
        rock::trim_layer(*solid, &open, seed, style, &colors, &mut trims);
    }
    fills.append(&mut trims);
    fills
}

/// The inputs a cached draw list was built from.
#[derive(Clone, Debug, PartialEq)]
struct Key {
    room: RoomId,
    solids: Vec<Rect>,
    ground: Option<Rect>,
    style: TerrainStyle,
}

/// The last draw list and what it was built from. Asking again with the same room, solids,
/// ground and palette returns the stored list; any change rebuilds it.
#[derive(Debug, Default)]
pub struct TerrainLook {
    key: Option<Key>,
    shapes: Vec<Shape>,
    builds: usize,
}

impl TerrainLook {
    pub fn shapes(
        &mut self,
        room: RoomId,
        solids: &[Rect],
        ground: Option<Rect>,
        style: &TerrainStyle,
    ) -> &[Shape] {
        let unchanged = self.key.as_ref().is_some_and(|key| {
            key.room == room && key.solids == solids && key.ground == ground && key.style == *style
        });
        if !unchanged {
            self.shapes = build(room, solids, ground, style);
            self.key = Some(Key {
                room,
                solids: solids.to_vec(),
                ground,
                style: *style,
            });
            self.builds += 1;
        }
        &self.shapes
    }

    /// How many times the list has been built (for tests and diagnostics).
    pub fn builds(&self) -> usize {
        self.builds
    }
}
