//! What a run's start and end do to the player.
use super::Player;

impl Player {
    /// Forgets every collected item (the Dash too, so the dash is locked again) and restores full
    /// health. Position, velocity and timers are left alone; the caller places the player.
    pub fn reset_for_run(&mut self) {
        self.collected_items.clear();
        self.health.respawn();
    }
}
