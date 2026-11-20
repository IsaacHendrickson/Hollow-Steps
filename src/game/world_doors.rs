//! Doors in play (`crate::door_exit`): an exit that stands anywhere in a room and is used by
//! pressing Up in front of it.
//!
//! One rule decides both entering a door and the "Up: enter" prompt the scene draws, so the two
//! cannot disagree: `door_in_reach`. The player must stand on the ground with their center
//! within the door's width, the exits must be unlocked (a boss room's doors stay shut until the
//! boss falls), and it must not be a test room, whose exits would lead into saved rooms. An
//! item or charm in reach takes the press first, so the door and its prompt wait until it is
//! taken (the item shows its own "Up: pick up" prompt); Up with X is an upward strike, not a door.
use super::GameState;
use crate::room::Exit;

impl GameState {
    /// The door the player could enter right now by pressing Up, if any.
    pub fn door_in_reach(&self) -> Option<&Exit> {
        if self.test_run.is_some()
            || !self.player.is_grounded()
            || self.item_in_reach()
            || self.charm_in_reach().is_some()
        {
            return None;
        }
        self.room()
            .door_at(self.player.position(), self.player.size())
    }
}
