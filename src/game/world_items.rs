//! Items lying in rooms, in play and in the editor's Save (`crate::item`).
//!
//! In play: an item is picked up with the pickup key (reach rule: `WorldItem::can_pick_up`) and
//! the run remembers it as taken from that room (`TakenItems`), like coins and charms last for
//! the run. A room rebuilt from its file, which is what the editor's Save does for every room it
//! writes, gets the taken items removed again (`restore_taken_items`); an item the author moved
//! or added in the editor is a new item and appears. Nothing is written to disk. `reload_rooms`
//! and `edit_room` keep the live rooms' items and so need nothing.
//!
//! A test room is a sandbox: an item taken there is not remembered, and the saved room, parked
//! while the test runs, comes back with its items.
use super::GameState;
use crate::events::GameEvent;
use crate::room::RoomId;

impl GameState {
    /// Picks up the first item within reach, if any: it goes into the inventory and leaves the
    /// room, and (outside a test room) the run remembers it. Returns whether one was picked up.
    pub(super) fn pick_up_item(&mut self) -> bool {
        let room = &mut self.rooms[self.current];
        let Some(index) = room
            .items
            .iter()
            .position(|item| item.can_pick_up(&self.player))
        else {
            return false;
        };
        let item = room.items.remove(index);
        self.player.collect_item(item.item);
        if self.test_run.is_none() {
            self.taken_items.record(room.id, item);
        }
        self.events.push(GameEvent::ItemPickedUp);
        true
    }

    /// The rooms `ids` were just rebuilt from their files: takes the items the run has taken
    /// out of them again.
    pub(super) fn restore_taken_items(&mut self, ids: &[RoomId]) {
        for room in self.rooms.iter_mut().filter(|room| ids.contains(&room.id)) {
            self.taken_items.apply_to_rebuilt(room.id, &mut room.items);
        }
    }
}
