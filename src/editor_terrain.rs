//! Pure terrain editing for the room draft: add, move, resize and delete solids, plus the
//! hit-testing and resize math behind the canvas handles. Every edit is checked with
//! `RoomLayout::validate_geometry` on a copy, so a refused edit leaves the draft as it was.
use macroquad::math::{Rect, Vec2};

use crate::editor::RoomDraft;

/// Smallest width and height, in pixels, of a solid drawn or resized in the editor.
pub const MIN_TERRAIN_SIZE: f32 = 8.0;

/// A grab point on the outline of a solid. Each moves the edge or edges it sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    NorthWest,
    North,
    NorthEast,
    West,
    East,
    SouthWest,
    South,
    SouthEast,
}

impl Handle {
    pub const ALL: [Handle; 8] = [
        Handle::NorthWest,
        Handle::North,
        Handle::NorthEast,
        Handle::West,
        Handle::East,
        Handle::SouthWest,
        Handle::South,
        Handle::SouthEast,
    ];

    /// The edges this handle moves: horizontal (-1 left, 0 none, 1 right) and vertical
    /// (-1 top, 0 none, 1 bottom).
    fn sides(self) -> (i8, i8) {
        match self {
            Handle::NorthWest => (-1, -1),
            Handle::North => (0, -1),
            Handle::NorthEast => (1, -1),
            Handle::West => (-1, 0),
            Handle::East => (1, 0),
            Handle::SouthWest => (-1, 1),
            Handle::South => (0, 1),
            Handle::SouthEast => (1, 1),
        }
    }

    /// Where the handle sits on `rect`: a corner or the middle of an edge.
    pub fn position(self, rect: &Rect) -> Vec2 {
        let (x, y) = self.sides();
        Vec2::new(
            rect.x + rect.w * ((x + 1) as f32 / 2.0),
            rect.y + rect.h * ((y + 1) as f32 / 2.0),
        )
    }
}

/// The handle of `rect` within `reach` (world units, per axis) of `point`; the nearest wins.
pub fn handle_at(rect: &Rect, point: Vec2, reach: f32) -> Option<Handle> {
    if !(reach.is_finite() && reach >= 0.0 && point.is_finite()) {
        return None;
    }
    Handle::ALL
        .into_iter()
        .filter_map(|handle| {
            let gap = handle.position(rect) - point;
            (gap.x.abs() <= reach && gap.y.abs() <= reach).then_some((handle, gap.length_squared()))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(handle, _)| handle)
}

/// `rect` after dragging `handle` to `pointer`. The opposite edges stay put and the size
/// never drops below `min`, so a drag past the far edge stops there instead of flipping.
pub fn resized(rect: &Rect, handle: Handle, pointer: Vec2, min: f32) -> Rect {
    let (x, y) = handle.sides();
    let (mut left, mut right) = (rect.x, rect.right());
    let (mut top, mut bottom) = (rect.y, rect.bottom());
    match x {
        -1 => left = pointer.x.min(right - min),
        1 => right = pointer.x.max(left + min),
        _ => {}
    }
    match y {
        -1 => top = pointer.y.min(bottom - min),
        1 => bottom = pointer.y.max(top + min),
        _ => {}
    }
    Rect::new(left, top, right - left, bottom - top)
}

/// The rectangle spanned by two corners, whichever way the drag went.
pub fn corners_rect(a: Vec2, b: Vec2) -> Rect {
    Rect::new(
        a.x.min(b.x),
        a.y.min(b.y),
        (a.x - b.x).abs(),
        (a.y - b.y).abs(),
    )
}

/// `point` moved to the nearest spot inside `room` (non-finite points come back unchanged).
pub fn clamp_to_room(room: &Rect, point: Vec2) -> Vec2 {
    if !point.is_finite() {
        return point;
    }
    Vec2::new(
        point.x.max(room.x).min(room.right()),
        point.y.max(room.y).min(room.bottom()),
    )
}

fn tuple(rect: &Rect) -> (f32, f32, f32, f32) {
    (rect.x, rect.y, rect.w, rect.h)
}

impl RoomDraft {
    pub fn terrain_rect(&self, index: usize) -> Option<Rect> {
        self.layout
            .terrain
            .get(index)
            .map(|&(x, y, w, h)| Rect::new(x, y, w, h))
    }

    pub fn terrain_rects(&self) -> Vec<Rect> {
        self.layout
            .terrain
            .iter()
            .map(|&(x, y, w, h)| Rect::new(x, y, w, h))
            .collect()
    }

    /// The solid drawn on top at `point` (later solids are drawn over earlier ones).
    pub fn terrain_at(&self, point: Vec2) -> Option<usize> {
        self.layout
            .terrain
            .iter()
            .rposition(|&(x, y, w, h)| Rect::new(x, y, w, h).contains(point))
    }

    /// Applies `edit` to a copy of the layout and keeps it only if the whole layout is still
    /// valid: solids inside the room, and the spawn, items and exits clear of them.
    fn edit_terrain<R>(
        &mut self,
        edit: impl FnOnce(&mut Vec<(f32, f32, f32, f32)>) -> Result<R, String>,
    ) -> Result<R, String> {
        let mut layout = self.layout.clone();
        let result = edit(&mut layout.terrain)?;
        layout.validate_geometry()?;
        self.layout = layout;
        Ok(result)
    }

    /// Adds a solid and returns its index.
    pub fn add_terrain(&mut self, rect: Rect) -> Result<usize, String> {
        if !(rect.w >= MIN_TERRAIN_SIZE && rect.h >= MIN_TERRAIN_SIZE) {
            return Err(format!(
                "terrain must be at least {MIN_TERRAIN_SIZE} px wide and tall"
            ));
        }
        self.edit_terrain(|terrain| {
            terrain.push(tuple(&rect));
            Ok(terrain.len() - 1)
        })
    }

    /// Replaces solid `index` with `rect` (the result of a move or a resize).
    pub fn set_terrain(&mut self, index: usize, rect: Rect) -> Result<(), String> {
        self.edit_terrain(|terrain| {
            *terrain.get_mut(index).ok_or("missing terrain")? = tuple(&rect);
            Ok(())
        })
    }

    /// Moves solid `index` so its top-left corner is at `top_left`, kept inside the room.
    pub fn move_terrain(&mut self, index: usize, top_left: Vec2) -> Result<(), String> {
        if !top_left.is_finite() {
            return Err("position must be finite".into());
        }
        let current = self.terrain_rect(index).ok_or("missing terrain")?;
        let room = self.layout.rect();
        self.set_terrain(
            index,
            Rect::new(
                top_left.x.max(room.x).min(room.right() - current.w),
                top_left.y.max(room.y).min(room.bottom() - current.h),
                current.w,
                current.h,
            ),
        )
    }

    /// Drags `handle` of solid `index` to `pointer`, kept inside the room.
    pub fn resize_terrain(
        &mut self,
        index: usize,
        handle: Handle,
        pointer: Vec2,
    ) -> Result<(), String> {
        if !pointer.is_finite() {
            return Err("position must be finite".into());
        }
        let current = self.terrain_rect(index).ok_or("missing terrain")?;
        let pointer = clamp_to_room(&self.layout.rect(), pointer);
        self.set_terrain(index, resized(&current, handle, pointer, MIN_TERRAIN_SIZE))
    }

    /// Removes solid `index`; later solids shift down by one.
    pub fn delete_terrain(&mut self, index: usize) -> Result<(), String> {
        if index >= self.layout.terrain.len() {
            return Err("missing terrain".into());
        }
        self.layout.terrain.remove(index);
        Ok(())
    }
}
