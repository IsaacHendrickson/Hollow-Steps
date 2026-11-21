//! Locked exits in play (`crate::exit_lock`, `docs/keys-and-locks.md`).
//!
//! An exit locked by key `n` refuses the player until the run holds `Item::Key(n)`. Touching it
//! (an edge exit) or pressing Up in front of it (a door) with the key uses the key up, takes the
//! lock off the live exit for the rest of the run and lets the player through. Without the key
//! nothing happens but a notice. One key opens one door: a second door locked by the same number
//! stays locked. The boss lock (`Room::exits_locked`) is checked first and wins: while it holds,
//! the exits do not trigger at all, so no key is spent and no notice shows.
//!
//! The opened doors are remembered by place (`OpenedExits`), so the editor's Save, which rebuilds
//! a saved room from its file, opens them again. A test room is a sandbox: its exits do nothing,
//! so no key is used there and nothing is remembered. The run reset relocks everything.
use super::GameState;
use crate::exit_lock::{lock_message, OpenedExits};
use crate::item::Item;
use crate::notice::Notice;
use crate::room::{Exit, RoomId};

/// The doors opened with a key this run, and the notice about a locked one.
#[derive(Debug, Default)]
pub(super) struct Locks {
    pub(super) opened: OpenedExits,
    pub(super) notice: Notice,
}

impl GameState {
    /// What to tell the player right now ("Locked: needs Key 3"), if anything.
    pub fn notice(&self) -> Option<&str> {
        self.locks.notice.text()
    }

    /// The player is using `exit`. Returns true when they may go through: the exit is free, or
    /// they held its key, which is used up and takes the lock off the live exit. Returns false,
    /// with the notice showing and nothing else changed, when the key is missing.
    pub(super) fn pass_exit(&mut self, exit: Exit) -> bool {
        let Some(key) = exit.locked_by else {
            return true;
        };
        if !self.player.take_item(Item::Key(key)) {
            self.locks.notice.show(lock_message(key));
            return false;
        }
        let room = &mut self.rooms[self.current];
        if let Some(live) = room.exits.iter_mut().find(|live| **live == exit) {
            live.locked_by = None;
        }
        if self.test_run.is_none() {
            self.locks.opened.record(room.id, &exit);
        }
        true
    }

    /// The rooms `ids` were just rebuilt from their files: opens the exits the run has opened
    /// in them again.
    pub(super) fn restore_opened_exits(&mut self, ids: &[RoomId]) {
        for room in self.rooms.iter_mut().filter(|room| ids.contains(&room.id)) {
            self.locks.opened.apply_to_rebuilt(room.id, &mut room.exits);
        }
    }
}

#[cfg(test)]
mod tests;
