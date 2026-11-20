//! The pause menu: which screen is open, which button is selected and what a press does.
//!
//! Pure state and layout, no window. `GameState` owns one and stops the simulation while it is
//! open; `main.rs` turns keys and the mouse into the calls below and draws the result. The
//! Settings screen lists the keybinds: a row per action with its primary and alternate key, a
//! Reset keybinds button and Back. The menu only tracks which key slot is waiting for a key;
//! `GameState` owns the bindings themselves and changes them (`game::keybind_link`). During a run
//! the main screen also lists Abandon run, which asks for confirmation first.

use crate::keybinds::{Action, Slot};

mod layout;

pub use layout::{
    button_rects, item_at, panel_rect, slot_at, slot_rects, status_rect, title_baseline,
};

/// A button of the pause menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseItem {
    Unpause,
    Settings,
    /// Ends the program for now; later it will return to the main menu.
    Exit,
    /// Leaves the settings screen.
    Back,
    /// The keybind row of an action. Pressing it waits for a new key for the highlighted slot.
    Bind(Action),
    /// Puts every action back on its shipped keys.
    ResetKeybinds,
    /// Main screen of a run: asks whether to give the run up.
    AbandonRun,
    /// Confirm screen: gives the run up.
    ConfirmAbandon,
    /// Confirm screen: back to the main screen.
    KeepPlaying,
}

impl PauseItem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unpause => "Unpause",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
            Self::Back => "Back",
            Self::Bind(action) => action.label(),
            Self::ResetKeybinds => "Reset keybinds",
            Self::AbandonRun | Self::ConfirmAbandon => "Abandon run",
            Self::KeepPlaying => "Keep playing",
        }
    }
}

/// The screen the menu is showing while it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseScreen {
    Main,
    /// The main screen during a run: Abandon run sits between Settings and Exit.
    RunMain,
    /// The keybinds, Reset keybinds and a way back.
    Settings,
    /// Asks whether to abandon the run.
    Confirm,
}

/// The Settings screen's rows: a keybind row per action (in `Action::ALL` order), then Reset
/// keybinds and Back.
static SETTINGS_ITEMS: [PauseItem; Action::ALL.len() + 2] = {
    let mut items = [PauseItem::Back; Action::ALL.len() + 2];
    let mut row = 0;
    while row < Action::ALL.len() {
        items[row] = PauseItem::Bind(Action::ALL[row]);
        row += 1;
    }
    items[Action::ALL.len()] = PauseItem::ResetKeybinds;
    items
};

impl PauseScreen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Main | Self::RunMain => "Paused",
            Self::Settings => "Settings",
            Self::Confirm => "Abandon this run?",
        }
    }

    /// The buttons of this screen, top to bottom.
    pub fn items(self) -> &'static [PauseItem] {
        match self {
            Self::Main => &[PauseItem::Unpause, PauseItem::Settings, PauseItem::Exit],
            Self::RunMain => &[
                PauseItem::Unpause,
                PauseItem::Settings,
                PauseItem::AbandonRun,
                PauseItem::Exit,
            ],
            Self::Settings => &SETTINGS_ITEMS,
            Self::Confirm => &[PauseItem::ConfirmAbandon, PauseItem::KeepPlaying],
        }
    }
}

/// What the front end must do after a button press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseOutcome {
    Continue,
    /// The program should end.
    Exit,
    /// The keybinds should go back to the shipped keys (`GameState` does it and saves).
    ResetKeybinds,
    /// The run should be given up (`GameState` does it and shows the result).
    AbandonRun,
}

/// The pause menu. Closed until `escape` opens it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PauseMenu {
    screen: Option<PauseScreen>,
    /// Index into the open screen's items.
    selected: usize,
    /// Which key of a keybind row is highlighted (Left/Right or the mouse choose it).
    slot: Slot,
    /// The key slot that is waiting for its new key. While it is set, the selection is locked
    /// and Escape cancels the wait instead of leaving the screen.
    listening: Option<(Action, Slot)>,
    /// A one-line message for the Settings screen: what a rebind swapped, why a key was
    /// refused, or that saving failed.
    note: Option<String>,
    /// A run is going, so the main screen lists Abandon run. Set before the menu opens
    /// (`set_in_run`) and forgotten when it closes.
    in_run: bool,
}

impl PauseMenu {
    pub fn is_open(&self) -> bool {
        self.screen.is_some()
    }

    /// The open screen, or `None` while the menu is closed.
    pub fn screen(&self) -> Option<PauseScreen> {
        self.screen
    }

    /// The selected button of the open screen.
    pub fn selected_item(&self) -> Option<PauseItem> {
        self.screen
            .and_then(|screen| screen.items().get(self.selected).copied())
    }

    /// The highlighted key of a keybind row (meaningful while a `Bind` row is selected).
    pub fn slot(&self) -> Slot {
        self.slot
    }

    /// The key slot waiting for its new key, if any.
    pub fn listening(&self) -> Option<(Action, Slot)> {
        self.listening
    }

    /// The Settings screen's message line, if there is a message.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }

    pub fn set_note(&mut self, note: Option<String>) {
        self.note = note;
    }

    /// Chooses which main screen the menu shows: with Abandon run while a run is going.
    pub fn set_in_run(&mut self, in_run: bool) {
        self.in_run = in_run;
    }

    fn main_screen(&self) -> PauseScreen {
        if self.in_run {
            PauseScreen::RunMain
        } else {
            PauseScreen::Main
        }
    }

    /// Stops waiting for a key (the key was bound, or the wait was cancelled).
    pub fn stop_listening(&mut self) {
        self.listening = None;
    }

    /// The Escape key: opens the menu, cancels a wait for a key, closes the menu from the main
    /// screen, goes back from Settings to the main screen (with Settings selected) and from the
    /// confirm screen to the main screen (with Abandon run selected).
    pub fn escape(&mut self) {
        if self.listening.take().is_some() {
            self.note = None;
            return;
        }
        match self.screen {
            None => self.show(self.main_screen(), PauseItem::Unpause),
            Some(PauseScreen::Main | PauseScreen::RunMain) => self.close(),
            Some(PauseScreen::Settings) => self.show(self.main_screen(), PauseItem::Settings),
            Some(PauseScreen::Confirm) => self.show(PauseScreen::RunMain, PauseItem::AbandonRun),
        }
    }

    /// Moves the selection by `steps` buttons (negative is up), wrapping around the ends.
    /// Does nothing while the menu is closed or waiting for a key.
    pub fn step(&mut self, steps: i32) {
        let Some(screen) = self.screen else { return };
        if self.listening.is_some() {
            return;
        }
        let count = screen.items().len() as i32;
        self.selected = (self.selected as i32 + steps).rem_euclid(count) as usize;
    }

    /// Moves the highlight to the other key of a keybind row (negative is toward the primary
    /// key); it stops at the ends. Does nothing off a keybind row or while waiting for a key.
    pub fn step_slot(&mut self, steps: i32) {
        if self.listening.is_some() || !matches!(self.selected_item(), Some(PauseItem::Bind(_))) {
            return;
        }
        let index = (self.slot.index() as i32 + steps).clamp(0, Slot::ALL.len() as i32 - 1);
        self.slot = Slot::from_index(index as usize);
    }

    /// Selects `item` (the mouse resting on it). An item the open screen does not have is
    /// ignored, and so is everything while the menu waits for a key.
    pub fn select(&mut self, item: PauseItem) {
        if self.listening.is_some() {
            return;
        }
        if let Some(index) = self.index_of(item) {
            self.selected = index;
        }
    }

    /// Selects one key of a keybind row (the mouse resting on its button).
    pub fn select_slot(&mut self, action: Action, slot: Slot) {
        if self.listening.is_some() || self.index_of(PauseItem::Bind(action)).is_none() {
            return;
        }
        self.select(PauseItem::Bind(action));
        self.slot = slot;
    }

    /// Presses one key of a keybind row (a mouse click on its button): the menu waits for a
    /// new key for it, even if it was waiting for another one.
    pub fn activate_slot(&mut self, action: Action, slot: Slot) -> PauseOutcome {
        let item = PauseItem::Bind(action);
        let Some(index) = self.index_of(item) else {
            return PauseOutcome::Continue;
        };
        self.selected = index;
        self.slot = slot;
        self.listening = Some((action, slot));
        self.note = None;
        PauseOutcome::Continue
    }

    /// Presses the selected button (Enter).
    pub fn activate_selected(&mut self) -> PauseOutcome {
        match self.selected_item() {
            Some(item) => self.activate(item),
            None => PauseOutcome::Continue,
        }
    }

    /// Presses `item` (a mouse click). An item the open screen does not have, or any press
    /// while the menu is closed, does nothing.
    pub fn activate(&mut self, item: PauseItem) -> PauseOutcome {
        let Some(index) = self.index_of(item) else {
            return PauseOutcome::Continue;
        };
        match item {
            PauseItem::Unpause => self.close(),
            PauseItem::Settings => self.show(PauseScreen::Settings, PauseItem::Back),
            PauseItem::Back => self.show(self.main_screen(), PauseItem::Settings),
            PauseItem::Exit => return PauseOutcome::Exit,
            // The safe button starts selected, so a stray Enter keeps the run.
            PauseItem::AbandonRun => self.show(PauseScreen::Confirm, PauseItem::KeepPlaying),
            PauseItem::KeepPlaying => self.show(PauseScreen::RunMain, PauseItem::AbandonRun),
            PauseItem::ConfirmAbandon => {
                self.close();
                return PauseOutcome::AbandonRun;
            }
            PauseItem::Bind(action) => {
                self.selected = index;
                self.listening = Some((action, self.slot));
                self.note = None;
            }
            PauseItem::ResetKeybinds => {
                self.selected = index;
                self.listening = None;
                self.note = None;
                return PauseOutcome::ResetKeybinds;
            }
        }
        PauseOutcome::Continue
    }

    fn index_of(&self, item: PauseItem) -> Option<usize> {
        self.screen?.items().iter().position(|&other| other == item)
    }

    /// Closes the menu from outside (the result screen takes over); the same as pressing Unpause.
    pub fn close(&mut self) {
        self.screen = None;
        self.in_run = false;
        self.slot = Slot::default();
        self.listening = None;
        self.note = None;
    }

    fn show(&mut self, screen: PauseScreen, selected: PauseItem) {
        self.screen = Some(screen);
        self.selected = screen
            .items()
            .iter()
            .position(|&item| item == selected)
            .unwrap_or(0);
        self.slot = Slot::default();
        self.listening = None;
        self.note = None;
    }
}

#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod tests;
