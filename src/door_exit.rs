//! Door exits: an exit that stands anywhere in a room instead of on one of its edges, and is
//! used by pressing Up in front of it. Here: how a door is stored in a room file, how it is
//! checked, and how the editor adds and moves one. In play, see `Room::door_at`.
//!
//! In a room file an edge exit keeps its `edge`, `offset` and `length`; a door has a `door`
//! rectangle `(x, y, width, height)` in room coordinates instead, and no edge fields.
use macroquad::math::{Rect, Vec2};
use serde::de::{Deserializer, Error as _};
use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use crate::editor::{ExitPlacement, RoomDraft, RoomLayout, Selection};
use crate::editor_session::EditorSession;
use crate::room::{RoomEdge, RoomId, DOOR_SIZE};
use crate::terrain::Terrain;

/// The smallest width and height a door can have.
pub const MIN_DOOR_SIDE: f32 = 16.0;
/// Half the player's height: the spawn point is the player's center.
const PLAYER_HALF_HEIGHT: f32 = 24.0;

impl Serialize for ExitPlacement {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut out = serializer
            .serialize_struct("ExitPlacement", if self.door.is_some() { 3 } else { 5 })?;
        match self.door {
            Some(door) => out.serialize_field("door", &door)?,
            None => {
                out.serialize_field("edge", &self.edge)?;
                out.serialize_field("offset", &self.offset)?;
                out.serialize_field("length", &self.length)?;
            }
        }
        out.serialize_field("target_room", &self.target_room)?;
        out.serialize_field("spawn", &self.spawn)?;
        out.end()
    }
}

/// A plain value read into an `Option`: RON would otherwise want `Some(...)` around it.
fn plain<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

/// What a room file holds for one exit, before it is known to be an edge exit or a door.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExitFile {
    #[serde(default, deserialize_with = "plain")]
    edge: Option<RoomEdge>,
    #[serde(default, deserialize_with = "plain")]
    offset: Option<f32>,
    #[serde(default, deserialize_with = "plain")]
    length: Option<f32>,
    #[serde(default, deserialize_with = "plain")]
    door: Option<(f32, f32, f32, f32)>,
    target_room: RoomId,
    spawn: (f32, f32),
}

impl<'de> Deserialize<'de> for ExitPlacement {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let file = ExitFile::deserialize(deserializer)?;
        let (target_room, spawn) = (file.target_room, file.spawn);
        match (file.door, file.edge, file.offset, file.length) {
            (Some(door), None, None, None) => Ok(ExitPlacement {
                door: Some(door),
                ..ExitPlacement::edge_exit(RoomEdge::Bottom, 0.0, 0.0, target_room, spawn)
            }),
            (None, Some(edge), Some(offset), Some(length)) => Ok(ExitPlacement::edge_exit(
                edge,
                offset,
                length,
                target_room,
                spawn,
            )),
            (Some(_), ..) => Err(D::Error::custom(
                "a door exit has a `door` rectangle and no `edge`, `offset` or `length`",
            )),
            _ => Err(D::Error::custom(
                "an exit needs `edge`, `offset` and `length`, or a `door` rectangle",
            )),
        }
    }
}

impl ExitPlacement {
    /// An exit on a room edge.
    pub fn edge_exit(
        edge: RoomEdge,
        offset: f32,
        length: f32,
        target_room: RoomId,
        spawn: (f32, f32),
    ) -> Self {
        Self {
            edge,
            offset,
            length,
            target_room,
            spawn,
            door: None,
        }
    }

    pub fn is_door(&self) -> bool {
        self.door.is_some()
    }

    /// Why this exit, if it is a door, does not fit `layout`; `Ok` for any edge exit.
    pub(crate) fn check_door(&self, layout: &RoomLayout, terrain: &Terrain) -> Result<(), String> {
        let Some((x, y, w, h)) = self.door else {
            return Ok(());
        };
        let room = layout.rect();
        if ![x, y, w, h].iter().all(|n| n.is_finite())
            || w < MIN_DOOR_SIDE
            || h < MIN_DOOR_SIDE
            || !Vec2::from(self.spawn).is_finite()
        {
            return Err(format!(
                "a door must be at least {MIN_DOOR_SIDE} wide and high, with a valid destination"
            ));
        }
        if x < room.x || x + w > room.right() || y < room.y || y + h > layout.floor_y {
            return Err("a door must be inside the room above the floor".into());
        }
        if terrain.overlaps_rect(&Rect::new(x, y, w, h)) {
            return Err("exit must not be inside terrain".into());
        }
        Ok(())
    }

    /// Moves a door so its center is at `center`, keeping its size. Edge exits are unchanged.
    pub(crate) fn move_door_to(&mut self, center: Vec2) {
        if let Some((_, _, w, h)) = self.door {
            self.door = Some((center.x - w / 2.0, center.y - h / 2.0, w, h));
        }
    }

    /// Sets a door's size, keeping its bottom edge and center line where they are.
    pub fn resize_door(&mut self, width: f32, height: f32) -> Result<(), String> {
        let (x, y, w, h) = self.door.ok_or("only a door has a size")?;
        if !width.is_finite()
            || !height.is_finite()
            || width < MIN_DOOR_SIDE
            || height < MIN_DOOR_SIDE
        {
            return Err(format!(
                "a door must be at least {MIN_DOOR_SIDE} wide and high"
            ));
        }
        self.door = Some((x + (w - width) / 2.0, y + h - height, width, height));
        Ok(())
    }
}

/// The y a door `width` wide from `left` can stand on, given the player's `feet` at the spawn:
/// the top of the highest solid at or below the feet that the door's width reaches, or the floor.
fn ground_below(layout: &RoomLayout, left: f32, width: f32, feet: f32) -> f32 {
    Terrain::from_tuples(&layout.terrain)
        .solids()
        .iter()
        .filter(|solid| solid.x < left + width && solid.right() > left && solid.y >= feet - 0.5)
        .map(|solid| solid.y)
        .fold(layout.floor_y, f32::min)
}

impl RoomDraft {
    /// Adds a door standing on the ground under the room's spawn point (the spawn can be in
    /// mid-air), leading to `target_room` and arriving at `spawn`. The ground is the top of the
    /// highest solid below the spawn within the door's width, or else the floor. Refused, with
    /// the draft unchanged, when it would not fit there.
    pub fn add_door(&mut self, target_room: RoomId, spawn: Vec2) -> Result<Selection, String> {
        let mut next = self.clone();
        let (px, py) = next.layout.spawn;
        let (w, h) = (DOOR_SIZE.x, DOOR_SIZE.y);
        let room = next.layout.rect();
        let left = (px - w / 2.0).clamp(room.x, (room.right() - w).max(room.x));
        let bottom = ground_below(&next.layout, left, w, py + PLAYER_HALF_HEIGHT);
        let index = next.layout.exits.len();
        next.layout.exits.push(ExitPlacement {
            door: Some((left, bottom - h, w, h)),
            ..ExitPlacement::edge_exit(RoomEdge::Bottom, 0.0, 0.0, target_room, spawn.into())
        });
        next.layout.validate_geometry()?;
        *self = next;
        Ok(Selection::Exit(index))
    }

    /// Changes the size of door `index`; refused, with the draft unchanged, when it would not fit.
    pub fn resize_door(&mut self, index: usize, width: f32, height: f32) -> Result<(), String> {
        let mut next = self.clone();
        next.layout
            .exits
            .get_mut(index)
            .ok_or("missing exit")?
            .resize_door(width, height)?;
        next.layout.validate_geometry()?;
        *self = next;
        Ok(())
    }
}

impl EditorSession {
    /// Places a new door on the ground under the spawn point and opens the room chooser, like
    /// `begin_link_new_exit`; Cancel takes the door out again. Returns the door's index.
    pub fn begin_link_new_door(&mut self) -> Result<usize, String> {
        if self.link.is_some() {
            return Err("finish or cancel the link in progress first".into());
        }
        let before = self.draft.clone();
        let was_dirty = self.dirty;
        let spawn = self.draft.layout.spawn.into();
        let id = self.id;
        self.draft.add_door(id, spawn)?;
        let exit = self.draft.layout.exits.len() - 1;
        self.dirty = true;
        self.start_link(exit, true, before, was_dirty)?;
        Ok(exit)
    }
}

#[cfg(test)]
mod tests;
