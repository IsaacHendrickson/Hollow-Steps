//! What the player holds besides items: abilities (`crate::ability`).
use super::Player;
use crate::ability::AbilitySet;

impl Player {
    /// The abilities found this run. The dash check reads them; collecting an item that grants
    /// one (`Item::grants`) is what adds it.
    pub fn abilities(&self) -> &AbilitySet {
        &self.abilities
    }
}
