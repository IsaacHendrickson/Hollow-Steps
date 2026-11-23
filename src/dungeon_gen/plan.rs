//! What the generator returns: the rooms of a dungeon, which piece each came from, and the hub
//! rewired to the start room.
use crate::editor::RoomLayout;
use crate::room::RoomId;

/// One room of a planned dungeon.
#[derive(Clone, Debug, PartialEq)]
pub struct PlannedRoom {
    /// The id the room has in the dungeon: `start_room` for the Start piece, `final_boss_room`
    /// for the Boss piece, the piece's own id for any other.
    pub instance: RoomId,
    /// The id of the piece the room was made from.
    pub piece: RoomId,
    /// Whether the piece was placed mirrored.
    pub mirrored: bool,
    /// The room as it plays: exits paired and aimed at their neighbours, locks and gates carried,
    /// keys and abilities lying where slots were. It carries no piece mark, no slots and no map
    /// cell: it is a room of a dungeon, not a piece of a pool.
    pub layout: RoomLayout,
}

/// A dungeon built from pieces, as the checker takes it (`catalog`).
#[derive(Clone, Debug, PartialEq)]
pub struct DungeonPlan {
    pub seed: u64,
    /// The start room first, then the rest of the spine up to the boss room, then the side
    /// branches in the order they were built.
    pub rooms: Vec<PlannedRoom>,
    /// The id of the hub room.
    pub hub_room: RoomId,
    /// The hub, with each exit that leads to the start room arriving at the start room's spawn.
    pub hub: RoomLayout,
    /// The attempt that gave this plan, counting from 1.
    pub attempts: u32,
}

impl DungeonPlan {
    /// Every room as the dungeon checker takes it, the hub first.
    pub fn catalog(&self) -> Vec<(RoomId, RoomLayout)> {
        let mut rooms = vec![(self.hub_room, self.hub.clone())];
        rooms.extend(
            self.rooms
                .iter()
                .map(|room| (room.instance, room.layout.clone())),
        );
        rooms
    }

    /// The planned room with this instance id.
    pub fn room(&self, instance: RoomId) -> Option<&PlannedRoom> {
        self.rooms.iter().find(|room| room.instance == instance)
    }
}
