//! How `GameState` owns the dungeon map (`crate::dungeon_map`, `docs/dungeon-map.md`): the rooms
//! this run has seen, and the map of every room built from the live rooms.
//!
//! A room is recorded when the player enters it (`enter_room`), when the game starts in it, when
//! the run's hub is reached again, and at the start of every tick for the room the player is in
//! (which also covers the editor moving the player). The set is cleared when a run starts and
//! when one ends. It is keyed by room id, so an editor Save, which rebuilds every room, keeps it.
//! A test room records nothing. With no hub nothing is ever cleared: the map starts with the
//! start room and grows as the player walks.
use super::GameState;
use crate::dungeon_map::{MapDiscovery, MapLayout, MapRoomInput};

/// What the map screen knows about the run.
#[derive(Debug, Default)]
pub(super) struct MapState {
    pub(super) discovery: MapDiscovery,
}

impl GameState {
    /// The rooms visited this run.
    pub fn map_discovery(&self) -> &MapDiscovery {
        &self.map.discovery
    }

    /// Every room of the world on the map grid, seen or not. The map screen draws only the seen
    /// ones and the rooms one exit from them; the editor's window draws all.
    pub fn dungeon_map(&self) -> MapLayout {
        let rooms: Vec<MapRoomInput> = self.rooms.iter().map(MapRoomInput::from_room).collect();
        MapLayout::build(&rooms)
    }

    /// Records the room the player is in as visited. A test room is a sandbox and records
    /// nothing.
    pub(super) fn discover_current_room(&mut self) {
        if self.test_run.is_none() {
            let id = self.room().id;
            self.map.discovery.record(id);
        }
    }
}
