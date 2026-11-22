//! Which rooms the player has seen this run.
use std::collections::HashSet;

use crate::room::RoomId;

/// The rooms visited this run, by room id. Keyed by id, not by position in the room list, so an
/// editor Save (which rebuilds every room) keeps it. `GameState` records a room when the player
/// enters it and clears the set when a run starts or ends; a test room records nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MapDiscovery {
    visited: HashSet<RoomId>,
}

impl MapDiscovery {
    /// Marks `id` visited. Returns true when it was new.
    pub fn record(&mut self, id: RoomId) -> bool {
        self.visited.insert(id)
    }

    pub fn contains(&self, id: RoomId) -> bool {
        self.visited.contains(&id)
    }

    pub fn clear(&mut self) {
        self.visited.clear();
    }

    pub fn len(&self) -> usize {
        self.visited.len()
    }

    pub fn is_empty(&self) -> bool {
        self.visited.is_empty()
    }
}
