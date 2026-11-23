//! A room seen in a mirror (`docs/variety.md`): the pure half of the dungeon flip. Nothing here
//! reads a file, a seed or the game; `flip_world` (in `world`) applies it to every room of a
//! dungeon, and `game::world_load` calls that when a run is flipped.
//!
//! The mirror is the vertical line through the middle of a room's bounds, so a room maps onto
//! itself: a point `x` goes to `2 * bounds.x + bounds.w - x`, a rectangle by its far edge, the
//! y axis never changes, and `bounds`, `floor_y` and the name stay as they are. Mirroring a room
//! twice gives the room back (exactly, for whole-number coordinates).
//!
//! What is mirrored: the spawn, terrain, hazards (their side too), breakables, switches,
//! barriers, items, charms, pickup slots, every exit, the gates that name those exits, and the
//! map cell. What is kept: the piece mark, the terrain look and the background (drawing only, a
//! known limit). An exit's arrival `spawn` is a point in another room, so only `flip_world`,
//! which knows that room, moves it.
use crate::editor::{ExitPlacement, RoomLayout};
use crate::gates::ExitRef;
use crate::hazard_look::HazardFacing;
use crate::room::{RoomEdge, RoomEnemies};

mod source;
mod world;
pub(crate) use source::room_ids;
pub use source::FlippedDocuments;
pub use world::flip_world;

#[cfg(test)]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;

/// `(x, y, width, height)`, the way room files hold a rectangle.
type Rectangle = (f32, f32, f32, f32);

/// The mirror line of a room: `x` maps to `sum - x`.
#[derive(Clone, Copy, Debug)]
struct Axis {
    sum: f32,
    width: f32,
}

impl Axis {
    fn of(bounds: Rectangle) -> Self {
        Self {
            sum: 2.0 * bounds.0 + bounds.2,
            width: bounds.2,
        }
    }

    fn x(self, x: f32) -> f32 {
        self.sum - x
    }

    fn point(self, (x, y): (f32, f32)) -> (f32, f32) {
        (self.x(x), y)
    }

    fn rect(self, (x, y, width, height): Rectangle) -> Rectangle {
        (self.x(x + width), y, width, height)
    }
}

/// Where the point `at` of a room with `bounds` lands when the room is mirrored.
pub fn mirror_point(bounds: (f32, f32, f32, f32), at: (f32, f32)) -> (f32, f32) {
    Axis::of(bounds).point(at)
}

/// An exit as it stands in the mirrored room. Its `spawn` is in another room and stays.
fn mirror_exit(exit: &ExitPlacement, axis: Axis) -> ExitPlacement {
    let mut mirrored = exit.clone();
    if let Some(door) = exit.door {
        // A door has no edge or offset of its own: only its rectangle moves.
        mirrored.door = Some(axis.rect(door));
        return mirrored;
    }
    match exit.edge {
        RoomEdge::Left => mirrored.edge = RoomEdge::Right,
        RoomEdge::Right => mirrored.edge = RoomEdge::Left,
        // The offset is measured from the left, and the exit now ends where it began.
        RoomEdge::Top | RoomEdge::Bottom => {
            mirrored.offset = axis.width - exit.offset - exit.length;
        }
    }
    mirrored
}

/// How exit `exit` of a room with `bounds` is named once the room is mirrored: the `ExitRef` of
/// the mirrored exit, found by looking `exit` up in `exits` (a reference carries no length).
/// `None` when the room has no such exit.
fn mirror_exit_ref(exit: &ExitRef, exits: &[ExitPlacement], bounds: Rectangle) -> Option<ExitRef> {
    let found = exits
        .iter()
        .find(|placement| ExitRef::of(placement) == *exit)?;
    Some(ExitRef::of(&mirror_exit(found, Axis::of(bounds))))
}

fn mirror_facing(facing: HazardFacing) -> HazardFacing {
    match facing {
        HazardFacing::Left => HazardFacing::Right,
        HazardFacing::Right => HazardFacing::Left,
        other => other,
    }
}

/// The room as seen in a mirror. See the module comment for what moves and what stays.
pub fn mirror_layout(layout: &RoomLayout) -> RoomLayout {
    let axis = Axis::of(layout.bounds);
    let mut room = layout.clone();
    room.spawn = axis.point(layout.spawn);
    room.terrain = layout.terrain.iter().map(|rect| axis.rect(*rect)).collect();
    for hazard in &mut room.hazards {
        hazard.rect = axis.rect(hazard.rect);
        hazard.facing = mirror_facing(hazard.facing);
    }
    for breakable in &mut room.breakables {
        breakable.rect = axis.rect(breakable.rect);
    }
    for switch in &mut room.switches {
        switch.rect = axis.rect(switch.rect);
    }
    for barrier in &mut room.barriers {
        barrier.rect = axis.rect(barrier.rect);
    }
    for item in &mut room.items {
        item.position = axis.point(item.position);
    }
    for charm in &mut room.charms {
        charm.position = axis.point(charm.position);
    }
    // The piece mark has no place in a room and stays; each pickup slot is a point like an item.
    for slot in &mut room.slots {
        slot.position = axis.point(slot.position);
    }
    room.exits = layout
        .exits
        .iter()
        .map(|exit| mirror_exit(exit, axis))
        .collect();
    // A gate names one of this room's exits, which has just moved. Its entrance names an exit of
    // another room (or the spawn), which only `flip_world` can follow.
    for gate in &mut room.gates {
        if let Some(moved) = mirror_exit_ref(&gate.exit, &layout.exits, layout.bounds) {
            gate.exit = moved;
        }
    }
    // The map's columns run the other way; rows are as they were.
    room.map_cell = layout
        .map_cell
        .map(|(column, row)| (column.saturating_neg(), row));
    room
}

/// A room file seen in a mirror: the layout, each enemy's patrol (the ends swap, so the left
/// end is the mirror of the old right end) and the boss's position. A document with no layout
/// has no bounds to mirror in and comes back unchanged.
pub fn mirror_document(document: &RoomEnemies) -> RoomEnemies {
    let Some(layout) = &document.layout else {
        return document.clone();
    };
    let axis = Axis::of(layout.bounds);
    let mut mirrored = document.clone();
    mirrored.layout = Some(mirror_layout(layout));
    for enemy in &mut mirrored.enemies {
        let (left, right) = (enemy.patrol_left, enemy.patrol_right);
        enemy.patrol_left = axis.x(right);
        enemy.patrol_right = axis.x(left);
    }
    if let Some(boss) = &mut mirrored.boss {
        boss.position = axis.point(boss.position);
    }
    mirrored
}
