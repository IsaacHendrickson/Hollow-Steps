//! What the map screen in play shows: the rooms the player has seen, the rooms one exit from them
//! as empty boxes, and the exits of the seen rooms. Everything else is left out, not hidden.
use std::collections::HashSet;

use super::{Cell, MapDiscovery, MapLayout, MapLink};
use crate::room::RoomId;

/// A room the map screen draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayRoom {
    pub id: RoomId,
    /// The room's name; only a room the player has been in shows one.
    pub name: Option<String>,
    pub cell: Cell,
    /// The player has been here: its box is filled. Otherwise it is an empty box.
    pub visited: bool,
    /// The player is in this room now.
    pub current: bool,
}

/// The rooms and links the map screen draws.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayMap {
    /// In room id order.
    pub rooms: Vec<PlayRoom>,
    /// The exits of visited rooms (a room not visited yet has shown no exit).
    pub links: Vec<MapLink>,
}

impl PlayMap {
    /// The part of `layout` the player knows: rooms in `seen` (filled, with their names) and the
    /// rooms their exits lead to (empty boxes), `current` marked.
    pub fn new(layout: &MapLayout, seen: &MapDiscovery, current: RoomId) -> Self {
        let links: Vec<MapLink> = layout
            .links
            .iter()
            .filter(|link| seen.contains(link.from))
            .copied()
            .collect();
        let next_door: HashSet<RoomId> = links.iter().map(|link| link.to).collect();
        let rooms = layout
            .rooms
            .iter()
            .filter(|room| seen.contains(room.id) || next_door.contains(&room.id))
            .map(|room| {
                let visited = seen.contains(room.id);
                PlayRoom {
                    id: room.id,
                    name: visited.then(|| room.name.clone()),
                    cell: room.cell,
                    visited,
                    current: room.id == current,
                }
            })
            .collect();
        Self { rooms, links }
    }

    /// The cell of every room drawn: what the grid is fitted to.
    pub fn cells(&self) -> Vec<Cell> {
        self.rooms.iter().map(|room| room.cell).collect()
    }
}
