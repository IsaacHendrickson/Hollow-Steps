//! What the player holds besides items: abilities (`crate::ability`), and spending an item
//! (a key used up on a locked exit).
use super::Player;
use crate::ability::AbilitySet;
use crate::item::Item;

impl Player {
    /// The abilities found this run. The dash check reads them; collecting an item that grants
    /// one (`Item::grants`) is what adds it.
    pub fn abilities(&self) -> &AbilitySet {
        &self.abilities
    }

    /// Uses up one `item` from the inventory. Returns false, and changes nothing, when the
    /// player holds none. Only keys are spent today: an ability is not an item that is used up.
    pub(crate) fn take_item(&mut self, item: Item) -> bool {
        match self.collected_items.iter().position(|held| *held == item) {
            Some(index) => {
                self.collected_items.remove(index);
                true
            }
            None => false,
        }
    }
}
