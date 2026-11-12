//! The visual-only slabs around a room: a ceiling, two walls and a floor, drawn like solids.
//!
//! Geometry. `Room::bounds` is the rectangle the player may move in, and `floor_y` is the top
//! of the floor inside it (it can sit above the bottom of the bounds, which leaves a strip of
//! ground under the floor line). The slabs sit *outside* that playable area, flush with it:
//!
//! - the ceiling runs over the whole top, corners included;
//! - the left and right walls stand between the ceiling and the floor slab;
//! - the floor slab starts at `floor_y`, so it also fills any ground below the floor line
//!   inside the bounds, and runs out under the walls and `thickness` below the bounds.
//!
//! The four pieces meet exactly and never overlap. Exits are 32 px zones against the inside
//! of an edge, so the slabs never reach them; each exit also gets a doorway: the slab is cut
//! open across its whole thickness over the exit's span. A zone is never covered, whichever
//! edge the exit is on and wherever the floor line sits.
//!
//! Nothing here is collision: `Room::bounds` and `floor_y` still do all the blocking.

use macroquad::math::Rect;

use crate::room::{Exit, RoomEdge};

/// Positions closer than this count as the same line.
const TOL: f32 = 0.01;
/// Pieces thinner than this (in either direction) are dropped as rounding crumbs.
const MIN_PIECE: f32 = 1.0;

/// The slabs around `bounds`, with doorways cut at `exits`. Empty when the thickness is not
/// a positive finite number or the bounds are broken. `floor_y` is clamped into the bounds.
pub fn boundary_slabs(bounds: Rect, floor_y: f32, thickness: f32, exits: &[Exit]) -> Vec<Rect> {
    let sound = [bounds.x, bounds.y, bounds.w, bounds.h, floor_y, thickness]
        .iter()
        .all(|value| value.is_finite());
    if !sound || thickness <= 0.0 || bounds.w <= 0.0 || bounds.h <= 0.0 {
        return Vec::new();
    }
    let t = thickness;
    let floor = floor_y.clamp(bounds.y, bounds.bottom());
    let frame = [
        // Ceiling.
        Rect::new(bounds.x - t, bounds.y - t, bounds.w + 2.0 * t, t),
        // Left and right walls, down to the floor line.
        Rect::new(bounds.x - t, bounds.y, t, floor - bounds.y),
        Rect::new(bounds.right(), bounds.y, t, floor - bounds.y),
        // Floor slab: from the floor line down, past the bottom of the bounds by `t`.
        Rect::new(
            bounds.x - t,
            floor,
            bounds.w + 2.0 * t,
            bounds.bottom() - floor + t,
        ),
    ];
    let doorways: Vec<Rect> = exits
        .iter()
        .map(|exit| doorway(exit, bounds, t))
        .filter(|hole| {
            [hole.x, hole.y, hole.w, hole.h]
                .iter()
                .all(|value| value.is_finite())
        })
        .collect();
    frame
        .into_iter()
        .flat_map(|slab| carve(slab, &doorways))
        .filter(|piece| piece.w >= MIN_PIECE && piece.h >= MIN_PIECE)
        .collect()
}

/// The opening for `exit`: its zone plus the strip outside it that a slab of `thickness`
/// would fill.
fn doorway(exit: &Exit, bounds: Rect, thickness: f32) -> Rect {
    let zone = exit.zone(bounds);
    match exit.edge {
        RoomEdge::Left => Rect::new(zone.x - thickness, zone.y, zone.w + thickness, zone.h),
        RoomEdge::Right => Rect::new(zone.x, zone.y, zone.w + thickness, zone.h),
        RoomEdge::Top => Rect::new(zone.x, zone.y - thickness, zone.w, zone.h + thickness),
        RoomEdge::Bottom => Rect::new(zone.x, zone.y, zone.w, zone.h + thickness),
    }
}

/// What is left of `slab` once every hole is cut out of it.
fn carve(slab: Rect, holes: &[Rect]) -> Vec<Rect> {
    holes.iter().fold(vec![slab], |pieces, hole| {
        pieces
            .into_iter()
            .flat_map(|piece| subtract(piece, *hole))
            .collect()
    })
}

/// `piece` without `hole`: up to four rectangles (full-height strips left and right of the
/// hole, then what is above and below it). A hole that only touches leaves the piece whole.
fn subtract(piece: Rect, hole: Rect) -> Vec<Rect> {
    let left = piece.x.max(hole.x);
    let right = piece.right().min(hole.right());
    let top = piece.y.max(hole.y);
    let bottom = piece.bottom().min(hole.bottom());
    if right - left <= TOL || bottom - top <= TOL {
        return vec![piece];
    }
    let mut parts = Vec::with_capacity(4);
    if left - piece.x > TOL {
        parts.push(Rect::new(piece.x, piece.y, left - piece.x, piece.h));
    }
    if piece.right() - right > TOL {
        parts.push(Rect::new(right, piece.y, piece.right() - right, piece.h));
    }
    if top - piece.y > TOL {
        parts.push(Rect::new(left, piece.y, right - left, top - piece.y));
    }
    if piece.bottom() - bottom > TOL {
        parts.push(Rect::new(
            left,
            bottom,
            right - left,
            piece.bottom() - bottom,
        ));
    }
    parts
}
