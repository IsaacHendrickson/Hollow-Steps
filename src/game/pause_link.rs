//! How `GameState` owns the pause menu: the Escape key, the paused check the loop and
//! `update` share, and the accessors `main.rs` drives the menu through.

use super::GameState;
use crate::keybinds::{Action, Slot};
use crate::pause_menu::{PauseItem, PauseMenu, PauseOutcome};

impl GameState {
    /// The pause menu, for drawing.
    pub fn pause_menu(&self) -> &PauseMenu {
        &self.pause_menu
    }

    /// True while the world stands still for the inventory, the pause menu or a run's result
    /// screen. The editor stops the world too, but it is not "paused": it has its own loop.
    pub fn is_paused(&self) -> bool {
        self.inventory_open || self.pause_menu.is_open() || self.showing_result()
    }

    /// The Escape key during play. It closes the inventory if that is open; otherwise it opens
    /// the pause menu, closes it, or goes back from Settings. Ignored while the editor is open
    /// (the editor has no pause menu) and on the result screen (only Enter leaves it).
    pub fn press_escape(&mut self) {
        if self.editor.is_some() || self.showing_result() {
            return;
        }
        if self.inventory_open {
            self.inventory_open = false;
        } else {
            self.pause_menu.escape();
        }
    }

    /// Moves the menu selection by `steps` buttons (negative is up). Does nothing while the
    /// menu is closed.
    pub fn pause_step(&mut self, steps: i32) {
        self.pause_menu.step(steps);
    }

    /// The mouse is resting on `item`.
    pub fn pause_hover(&mut self, item: PauseItem) {
        self.pause_menu.select(item);
    }

    /// Moves the highlight to the other key of the selected keybind row (Left/Right).
    pub fn pause_step_slot(&mut self, steps: i32) {
        self.pause_menu.step_slot(steps);
    }

    /// The mouse is resting on one key button of a keybind row.
    pub fn pause_hover_slot(&mut self, action: Action, slot: Slot) {
        self.pause_menu.select_slot(action, slot);
    }

    /// Presses the selected button (Enter). Reset keybinds is done here; only `Exit` is left
    /// for the front end.
    pub fn pause_activate_selected(&mut self) -> PauseOutcome {
        let outcome = self.pause_menu.activate_selected();
        self.finish_pause_press(outcome)
    }

    /// Presses `item` (a mouse click).
    pub fn pause_activate(&mut self, item: PauseItem) -> PauseOutcome {
        let outcome = self.pause_menu.activate(item);
        self.finish_pause_press(outcome)
    }

    /// Presses one key button of a keybind row (a mouse click): the menu waits for a new key.
    pub fn pause_activate_slot(&mut self, action: Action, slot: Slot) -> PauseOutcome {
        self.pause_menu.activate_slot(action, slot)
    }
}
