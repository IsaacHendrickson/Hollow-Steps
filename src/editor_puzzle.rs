//! Pure puzzle-piece editing for the room draft: add a breakable wall, a switch or a barrier
//! beside the spawn, move one, change its size and its number, delete one. Every edit is checked
//! with `RoomLayout::validate_geometry` on a copy, so a refused edit leaves the draft as it was:
//! a piece on the spawn, a number out of range, a switch number used twice, or a barrier left
//! without its switch (the error names the barrier). `docs/puzzles.md`.
use macroquad::math::Vec2;

use crate::editor::{RoomDraft, RoomLayout, Selection};
use crate::puzzle::{
    BarrierPlacement, BreakablePlacement, SwitchPlacement, FIRST_SWITCH, LAST_SWITCH,
};

/// What a new piece looks like; the author then edits each piece's own numbers.
const NEW_BREAKABLE: (f32, f32) = (32.0, 64.0);
const NEW_HITS: u8 = 3;
const NEW_SWITCH: (f32, f32) = (24.0, 24.0);
const NEW_BARRIER: (f32, f32) = (24.0, 96.0);

/// Half the player's width, which a new piece keeps clear of the spawn point, plus a margin.
const SPAWN_CLEARANCE: f32 = 16.0 + 16.0;

/// What the editor panel edits for one piece: its size, and its one number (a wall's hits, a
/// switch's own number, or the number of the switch that opens a barrier).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PieceFields {
    pub width: f32,
    pub height: f32,
    pub number: u8,
}

type Numbers = (f32, f32, f32, f32);

/// `rect` resized to `width` by `height`, keeping its center line and its bottom edge, so a
/// piece standing on the floor grows upward from it (like a hazard).
fn resized((x, y, w, h): Numbers, width: f32, height: f32) -> Numbers {
    (x + (w - width) / 2.0, y + h - height, width, height)
}

impl RoomLayout {
    /// Moves a piece so its center is at `center`, kept inside the room.
    pub(crate) fn move_puzzle(&mut self, selection: Selection, center: Vec2) -> Result<(), String> {
        if !center.is_finite() {
            return Err("position must be finite".into());
        }
        let room = self.rect();
        let rect = match selection {
            Selection::Breakable(i) => &mut self.breakables.get_mut(i).ok_or("missing wall")?.rect,
            Selection::Switch(i) => &mut self.switches.get_mut(i).ok_or("missing switch")?.rect,
            Selection::Barrier(i) => &mut self.barriers.get_mut(i).ok_or("missing barrier")?.rect,
            _ => return Err("that is not a puzzle piece".into()),
        };
        let (_, _, w, h) = *rect;
        *rect = (
            (center.x - w / 2.0).max(room.x).min(room.right() - w),
            (center.y - h / 2.0).max(room.y).min(room.bottom() - h),
            w,
            h,
        );
        Ok(())
    }

    /// Takes a piece out, if the layout stays valid: deleting a switch that a barrier still
    /// needs is refused, and the error names the barrier.
    pub(crate) fn delete_puzzle(&mut self, selection: Selection) -> Result<(), String> {
        let mut next = self.clone();
        match selection {
            Selection::Breakable(i) if i < next.breakables.len() => {
                next.breakables.remove(i);
            }
            Selection::Switch(i) if i < next.switches.len() => {
                next.switches.remove(i);
            }
            Selection::Barrier(i) if i < next.barriers.len() => {
                next.barriers.remove(i);
            }
            _ => return Err("select a wall, switch or barrier to delete".into()),
        }
        next.validate_puzzles()?;
        *self = next;
        Ok(())
    }
}

impl RoomDraft {
    /// Puts a piece of `size` standing on the floor next to the spawn point (to its right, or to
    /// its left when the right side has no room), made by `push`, and returns its selection.
    /// Drag it where it belongs.
    fn add_piece(
        &mut self,
        what: &str,
        size: (f32, f32),
        push: impl Fn(&mut RoomLayout, Numbers) -> Selection,
    ) -> Result<Selection, String> {
        let (spawn_x, _) = self.layout.spawn;
        let top = self.layout.floor_y - size.1;
        let mut error = format!("no room beside the spawn point for a {what}");
        for x in [
            spawn_x + SPAWN_CLEARANCE,
            spawn_x - SPAWN_CLEARANCE - size.0,
        ] {
            let mut layout = self.layout.clone();
            let selection = push(&mut layout, (x, top, size.0, size.1));
            match layout.validate_geometry() {
                Ok(()) => {
                    self.layout = layout;
                    return Ok(selection);
                }
                Err(refused) => error = refused,
            }
        }
        Err(error)
    }

    /// Adds a breakable wall of `NEW_HITS` hits.
    pub fn add_breakable(&mut self) -> Result<Selection, String> {
        self.add_piece("breakable wall", NEW_BREAKABLE, |layout, rect| {
            layout.breakables.push(BreakablePlacement {
                rect,
                hits: NEW_HITS,
            });
            Selection::Breakable(layout.breakables.len() - 1)
        })
    }

    /// Adds a switch with the lowest number the room does not use yet.
    pub fn add_switch(&mut self) -> Result<Selection, String> {
        let id = (FIRST_SWITCH..=LAST_SWITCH)
            .find(|id| !self.layout.switches.iter().any(|switch| switch.id == *id))
            .ok_or_else(|| format!("a room has at most {LAST_SWITCH} switches"))?;
        self.add_piece("switch", NEW_SWITCH, |layout, rect| {
            layout.switches.push(SwitchPlacement { rect, id });
            Selection::Switch(layout.switches.len() - 1)
        })
    }

    /// Adds a barrier opened by the room's first switch. A room without a switch refuses it:
    /// a barrier's switch must be in the same room.
    pub fn add_barrier(&mut self) -> Result<Selection, String> {
        let switch = self
            .layout
            .switches
            .first()
            .map(|switch| switch.id)
            .ok_or("add a switch first: a barrier is opened by a switch in the same room")?;
        self.add_piece("barrier", NEW_BARRIER, |layout, rect| {
            layout.barriers.push(BarrierPlacement { rect, switch });
            Selection::Barrier(layout.barriers.len() - 1)
        })
    }

    /// The size and number of the piece `selection` names, if it names one.
    pub fn piece_fields(&self, selection: Selection) -> Option<PieceFields> {
        let (rect, number) = match selection {
            Selection::Breakable(i) => {
                let wall = self.layout.breakables.get(i)?;
                (wall.rect, wall.hits)
            }
            Selection::Switch(i) => {
                let switch = self.layout.switches.get(i)?;
                (switch.rect, switch.id)
            }
            Selection::Barrier(i) => {
                let barrier = self.layout.barriers.get(i)?;
                (barrier.rect, barrier.switch)
            }
            _ => return None,
        };
        Some(PieceFields {
            width: rect.2,
            height: rect.3,
            number,
        })
    }

    /// The text the canvas writes on a piece's marker: a wall's hits, or the number of a switch
    /// and of the barrier it opens (the same text, so a pair can be matched by eye).
    pub fn marker_label(&self, selection: Selection) -> Option<String> {
        let fields = self.piece_fields(selection)?;
        Some(match selection {
            Selection::Breakable(_) if fields.number == 1 => "1 hit".into(),
            Selection::Breakable(_) => format!("{} hits", fields.number),
            _ => format!("S{}", fields.number),
        })
    }

    /// Gives the piece `fields`: a new size keeps the center line and the bottom edge. Refused,
    /// with the draft unchanged, when the layout would not be valid.
    pub fn set_piece_fields(
        &mut self,
        selection: Selection,
        fields: PieceFields,
    ) -> Result<(), String> {
        let mut layout = self.layout.clone();
        match selection {
            Selection::Breakable(i) => {
                let wall = layout.breakables.get_mut(i).ok_or("missing wall")?;
                wall.rect = resized(wall.rect, fields.width, fields.height);
                wall.hits = fields.number;
            }
            Selection::Switch(i) => {
                let switch = layout.switches.get_mut(i).ok_or("missing switch")?;
                switch.rect = resized(switch.rect, fields.width, fields.height);
                switch.id = fields.number;
            }
            Selection::Barrier(i) => {
                let barrier = layout.barriers.get_mut(i).ok_or("missing barrier")?;
                barrier.rect = resized(barrier.rect, fields.width, fields.height);
                barrier.switch = fields.number;
            }
            _ => return Err("that is not a puzzle piece".into()),
        }
        layout.validate_geometry()?;
        self.layout = layout;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
