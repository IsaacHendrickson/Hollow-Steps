//! The dungeon map as pure data: which cell of a grid every room sits in, which links join the
//! rooms, and (in `MapDiscovery`) which rooms this run has seen. No window and no drawing; the
//! screen is `hud_map` (play) and the Dungeon map window (editor). Design note:
//! `docs/dungeon-map.md`.
//!
//! A room may name its own cell (`map_cell` in its file). The others are placed by walking the
//! exits: a Right exit puts the neighbor one cell right, Left one left, Bottom one down and Top
//! one up (rows grow downward, like the game's y axis). Everything is deterministic: the same
//! rooms give the same map, whatever order they are listed in.
use std::collections::{HashMap, HashSet};

use crate::editor::RoomLayout;
use crate::room::{Room, RoomEdge, RoomId};
use crate::room_name::room_title;

mod discovery;
mod editor;
#[cfg(test)]
mod editor_tests;
mod grid;
#[cfg(test)]
mod grid_tests;
mod place;
mod play;
#[cfg(test)]
mod play_tests;
#[cfg(test)]
mod tests;

pub use discovery::MapDiscovery;
pub use editor::{EditorMap, EditorMapRoom};
pub use grid::{screen_layout, MapGrid, MapScreen, BOX_FRACTION, MAX_PITCH};
use place::{nearest_free, step, Wide};
pub use play::{PlayMap, PlayRoom};

/// A cell of the map grid: (column, row).
pub type Cell = (i32, i32);

/// How far from the origin a room's own cell may be, on either number.
pub const CELL_LIMIT: i32 = 64;

/// Checks a room's own cell: both numbers from `-CELL_LIMIT` to `CELL_LIMIT`.
pub fn check_cell((x, y): Cell) -> Result<(), String> {
    let range = -CELL_LIMIT..=CELL_LIMIT;
    if range.contains(&x) && range.contains(&y) {
        Ok(())
    } else {
        Err(format!(
            "map cell ({x}, {y}) is out of range: both numbers must be from -{CELL_LIMIT} to {CELL_LIMIT}"
        ))
    }
}

/// One exit as the map sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapExit {
    /// The wall the exit is on; `None` for a door, which says nothing about where its room is.
    pub edge: Option<RoomEdge>,
    pub target: RoomId,
    /// The key number that opens the exit, if it is locked.
    pub lock: Option<u8>,
}

/// One room as `MapLayout::build` takes it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRoomInput {
    pub id: RoomId,
    /// The name the map shows: the room's own, or its default display name.
    pub name: String,
    /// The room's own cell, if it has one.
    pub cell: Option<Cell>,
    pub exits: Vec<MapExit>,
}

impl MapRoomInput {
    /// A room of the running game.
    pub fn from_room(room: &Room) -> Self {
        Self {
            id: room.id,
            name: room_title(room.id, room.name.as_deref()),
            cell: room.map_cell,
            exits: room
                .exits
                .iter()
                .map(|exit| MapExit {
                    edge: (!exit.is_door()).then_some(exit.edge),
                    target: exit.target_room,
                    lock: exit.locked_by,
                })
                .collect(),
        }
    }

    /// A room as the editor holds it, saved or not.
    pub fn from_layout(id: RoomId, layout: &RoomLayout) -> Self {
        Self {
            id,
            name: room_title(id, layout.name.as_deref()),
            cell: layout.map_cell,
            exits: layout
                .exits
                .iter()
                .map(|exit| MapExit {
                    edge: (!exit.is_door()).then_some(exit.edge),
                    target: exit.target_room,
                    lock: exit.locked_by,
                })
                .collect(),
        }
    }
}

/// How a room came by its cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellSource {
    /// The room's own cell, kept.
    Explicit,
    /// The room's own cell was another room's too; this one was moved to a free cell.
    Conflict,
    /// Placed by an exit of a room already on the map.
    Walked,
    /// No exit walk reached it: it sits in the row below everything else.
    Row,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRoom {
    pub id: RoomId,
    pub name: String,
    pub cell: Cell,
    pub source: CellSource,
}

/// One exit drawn between two cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapLink {
    pub from: RoomId,
    pub to: RoomId,
    pub from_cell: Cell,
    pub to_cell: Cell,
    /// The target room has no exit back to this one.
    pub one_way: bool,
    /// The key number that opens the exit, if it is locked.
    pub lock: Option<u8>,
}

/// Two rooms that name the same cell. The room with the lower id keeps it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapConflict {
    pub kept: RoomId,
    pub moved: RoomId,
    pub cell: Cell,
}

/// Every room on the grid, the links between them and the cells that were claimed twice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapLayout {
    /// In room id order.
    pub rooms: Vec<MapRoom>,
    /// One per exit that leads to another room on the map, in room id order.
    pub links: Vec<MapLink>,
    pub conflicts: Vec<MapConflict>,
}

impl MapLayout {
    /// Places every room.
    ///
    /// 1. A room with a cell keeps it. When several rooms name one cell, the lowest id keeps it
    ///    and each other is reported as a conflict and moved to the nearest free cell.
    /// 2. The rooms placed so far are walked in id order, and each exit places its target, if it
    ///    has none yet, next to the exit's room: right, left, below or above for a Right, Left,
    ///    Bottom or Top exit, or the nearest free cell for a door. Then the rooms that walk
    ///    placed are walked the same way, until nothing new is placed. A cell already taken
    ///    moves the room to the nearest free one. With no cell named anywhere, the room with the
    ///    lowest id is put at (0, 0) to start from.
    /// 3. A room no walk reached goes in a row below everything, in id order.
    ///
    /// "Nearest" counts cells (not diagonals); equal distances go up, left, right, then down.
    /// A room listed twice counts once, a link to a room that is not listed is dropped, and so
    /// is a link from a room to itself.
    pub fn build(inputs: &[MapRoomInput]) -> Self {
        let mut rooms: Vec<&MapRoomInput> = inputs.iter().collect();
        rooms.sort_by_key(|room| room.id.index());
        rooms.dedup_by_key(|room| room.id.index());
        let slot: HashMap<RoomId, usize> = rooms
            .iter()
            .enumerate()
            .map(|(index, room)| (room.id, index))
            .collect();
        let mut cells: Vec<Option<(Wide, CellSource)>> = vec![None; rooms.len()];
        let mut taken: HashSet<Wide> = HashSet::new();
        let mut conflicts = Vec::new();

        // 1. The rooms with a cell of their own.
        let mut owner: HashMap<Wide, usize> = HashMap::new();
        let mut displaced = Vec::new();
        for (index, room) in rooms.iter().enumerate() {
            let Some(cell) = room.cell.map(place::widen) else {
                continue;
            };
            if let Some(&first) = owner.get(&cell) {
                displaced.push((index, first, cell));
            } else {
                owner.insert(cell, index);
                taken.insert(cell);
                cells[index] = Some((cell, CellSource::Explicit));
            }
        }
        for (index, first, cell) in displaced {
            let spot = nearest_free(cell, &taken);
            taken.insert(spot);
            cells[index] = Some((spot, CellSource::Conflict));
            conflicts.push(MapConflict {
                kept: rooms[first].id,
                moved: rooms[index].id,
                cell: place::narrow(cell),
            });
        }

        // 2. Walk the exits, one wave of rooms at a time.
        let mut wave: Vec<usize> = (0..rooms.len())
            .filter(|&index| cells[index].is_some())
            .collect();
        if wave.is_empty() && !rooms.is_empty() {
            taken.insert((0, 0));
            cells[0] = Some(((0, 0), CellSource::Walked));
            wave.push(0);
        }
        while !wave.is_empty() {
            let mut next = Vec::new();
            for &index in &wave {
                let Some((from, _)) = cells[index] else {
                    continue;
                };
                for exit in &rooms[index].exits {
                    let Some(&target) = slot.get(&exit.target) else {
                        continue;
                    };
                    if cells[target].is_some() {
                        continue;
                    }
                    let spot = nearest_free(step(from, exit.edge), &taken);
                    taken.insert(spot);
                    cells[target] = Some((spot, CellSource::Walked));
                    next.push(target);
                }
            }
            next.sort_unstable();
            wave = next;
        }

        // 3. What no walk reached, in a row below everything.
        let placed = || cells.iter().flatten().map(|(cell, _)| *cell);
        let row = placed().map(|cell| cell.1).max().map_or(0, |y| y + 1);
        let left = placed().map(|cell| cell.0).min().unwrap_or(0);
        let mut along = 0;
        for entry in &mut cells {
            if entry.is_none() {
                *entry = Some(((left + along, row), CellSource::Row));
                along += 1;
            }
        }

        let cell_of = |index: usize| cells[index].map_or((0, 0), |(cell, _)| place::narrow(cell));
        let mut links = Vec::new();
        for (index, room) in rooms.iter().enumerate() {
            for exit in &room.exits {
                let Some(&target) = slot.get(&exit.target) else {
                    continue;
                };
                if target == index {
                    continue;
                }
                links.push(MapLink {
                    from: room.id,
                    to: exit.target,
                    from_cell: cell_of(index),
                    to_cell: cell_of(target),
                    one_way: !rooms[target]
                        .exits
                        .iter()
                        .any(|back| back.target == room.id),
                    lock: exit.lock,
                });
            }
        }
        let rooms = rooms
            .iter()
            .enumerate()
            .map(|(index, room)| MapRoom {
                id: room.id,
                name: room.name.clone(),
                cell: cell_of(index),
                source: cells[index].map_or(CellSource::Row, |(_, source)| source),
            })
            .collect();
        Self {
            rooms,
            links,
            conflicts,
        }
    }

    /// The room `id`, if it is on the map.
    pub fn room(&self, id: RoomId) -> Option<&MapRoom> {
        self.rooms.iter().find(|room| room.id == id)
    }

    /// True when another room has an exit leading to `id`.
    pub fn has_entrance(&self, id: RoomId) -> bool {
        self.links
            .iter()
            .any(|link| link.to == id && link.from != id)
    }

    /// True when `id` is one of the two rooms of a conflict.
    pub fn in_conflict(&self, id: RoomId) -> bool {
        self.conflicts
            .iter()
            .any(|conflict| conflict.kept == id || conflict.moved == id)
    }
}
