//! How `GameState` owns the keybinds: the loaded bindings, the file they are saved to, and the
//! changes the pause menu's Settings screen makes. Every change is saved at once; a failed save
//! keeps the change for this run and says so in the menu's message line.

use std::path::PathBuf;

use macroquad::input::KeyCode;

use super::GameState;
use crate::keybinds::{self, Keybinds};
use crate::pause_menu::PauseOutcome;

impl GameState {
    /// The keys that do each action.
    pub fn keybinds(&self) -> &Keybinds {
        &self.keybinds
    }

    /// Reads the keybinds saved at `path` and saves every later change there. A missing file
    /// means the shipped keys. A file that cannot be used leaves the shipped keys in effect and
    /// is reported in the error; it is left alone on disk until the player changes a key.
    pub fn use_keybinds_file(&mut self, path: PathBuf) -> Result<(), String> {
        let loaded = keybinds::load(&path);
        self.keybinds_path = Some(path);
        match loaded {
            Ok(binds) => {
                self.keybinds = binds;
                Ok(())
            }
            Err(error) => {
                self.keybinds = Keybinds::default();
                Err(format!("Keybinds not loaded, using the defaults: {error}"))
            }
        }
    }

    /// A key was pressed while the Settings screen waits for one: binds it to the waiting slot.
    /// A key already in use trades places (the message line says so); a refused key keeps the
    /// wait going. Does nothing when no slot is waiting.
    pub fn pause_bind_key(&mut self, key: KeyCode) {
        let Some((action, slot)) = self.pause_menu.listening() else {
            return;
        };
        match self.keybinds.rebind(action, slot, key) {
            Ok(rebound) => {
                self.pause_menu.stop_listening();
                let note = rebound.swapped.map(|swap| keybinds::swap_note(key, swap));
                self.changed_keybinds(note);
            }
            Err(error) => self
                .pause_menu
                .set_note(Some(keybinds::error_note(error, Some(key)))),
        }
    }

    /// Delete (or Backspace) while the Settings screen waits for a key: empties the waiting
    /// slot. Only an alternate key can be emptied; the wait goes on after a refusal.
    pub fn pause_clear_key(&mut self) {
        let Some((action, slot)) = self.pause_menu.listening() else {
            return;
        };
        match self.keybinds.clear(action, slot) {
            Ok(()) => {
                self.pause_menu.stop_listening();
                self.changed_keybinds(None);
            }
            Err(error) => self
                .pause_menu
                .set_note(Some(keybinds::error_note(error, None))),
        }
    }

    /// Puts every action back on its shipped keys and removes the saved file, so the shipped
    /// keys apply on the next run too.
    fn reset_keybinds(&mut self) {
        self.keybinds.reset();
        let removed = match &self.keybinds_path {
            Some(path) => keybinds::remove(path),
            None => Ok(()),
        };
        let note = match removed {
            Ok(()) => "Keybinds reset to the defaults".to_owned(),
            Err(error) => format!("Keybinds reset, but the saved file stays: {error}"),
        };
        self.pause_menu.set_note(Some(note));
    }

    /// Saves after a change. The message line shows `note` (what a rebind swapped), or the
    /// reason the save failed, which takes its place.
    fn changed_keybinds(&mut self, note: Option<String>) {
        let saved = match &self.keybinds_path {
            Some(path) => keybinds::save(path, &self.keybinds),
            None => Ok(()),
        };
        self.pause_menu.set_note(match saved {
            Ok(()) => note,
            Err(error) => Some(format!("Keybinds not saved: {error}")),
        });
    }

    /// Does what a pause menu press asked of the game itself and returns what is left for the
    /// front end (only `Exit`). Abandon run is done here too: the result screen shows.
    pub(super) fn finish_pause_press(&mut self, outcome: PauseOutcome) -> PauseOutcome {
        match outcome {
            PauseOutcome::ResetKeybinds => {
                self.reset_keybinds();
                PauseOutcome::Continue
            }
            PauseOutcome::AbandonRun => {
                self.abandon_run();
                PauseOutcome::Continue
            }
            other => other,
        }
    }
}
