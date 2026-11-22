//! The small rules `MapLayout::build` places rooms with: which cell an exit leads to, and where a
//! room goes when that cell is taken. Cells are `i64` here so a scan next to the edge of the
//! `i32` range cannot overflow; they are narrowed back when the map is handed out.
use std::collections::HashSet;

use super::Cell;
use crate::room::RoomEdge;

pub(super) type Wide = (i64, i64);

pub(super) fn widen((x, y): Cell) -> Wide {
    (i64::from(x), i64::from(y))
}

pub(super) fn narrow((x, y): Wide) -> Cell {
    let clamp = |n: i64| i32::try_from(n).unwrap_or(if n < 0 { i32::MIN } else { i32::MAX });
    (clamp(x), clamp(y))
}

/// The cell next to `cell` that an exit on `edge` leads to; a door (`None`) says nothing, so its
/// room is looked for around `cell` itself.
pub(super) fn step((x, y): Wide, edge: Option<RoomEdge>) -> Wide {
    match edge {
        Some(RoomEdge::Right) => (x + 1, y),
        Some(RoomEdge::Left) => (x - 1, y),
        Some(RoomEdge::Bottom) => (x, y + 1),
        Some(RoomEdge::Top) => (x, y - 1),
        None => (x, y),
    }
}

/// `origin` if it is free; otherwise the nearest free cell by distance in cells (a diagonal
/// neighbor is two away). Cells at equal distance are tried in one fixed order: the row above
/// first, the cell to the left before the one to the right, the row below last.
pub(super) fn nearest_free(origin: Wide, taken: &HashSet<Wide>) -> Wide {
    // Some ring within `taken.len()` cells is always free: ring `d` holds `4d` cells.
    for distance in 0..=taken.len() as i64 {
        for dy in -distance..=distance {
            let dx = distance - dy.abs();
            for x in [origin.0 - dx, origin.0 + dx] {
                let cell = (x, origin.1 + dy);
                if !taken.contains(&cell) {
                    return cell;
                }
            }
        }
    }
    origin
}
