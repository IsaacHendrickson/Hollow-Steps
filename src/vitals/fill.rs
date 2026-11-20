//! Filling the mana, for the start and end of a run.
use super::VitalsState;

impl VitalsState {
    /// Mana to its maximum. Health, the trailing bar and a heal in progress are left alone.
    pub fn fill_mana(&mut self) {
        self.mana = self.max_mana;
    }
}
