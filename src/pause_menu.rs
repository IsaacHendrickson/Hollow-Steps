//! The pause menu: which screen is open, which button is selected and what a press does.
//!
//! Pure state and layout, no window. `GameState` owns one and stops the simulation while it is
//! open; `main.rs` turns keys and the mouse into the calls below and draws the result.

use macroquad::math::{Rect, Vec2};

use crate::camera::{anchored, Anchor};

/// A button of the pause menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseItem {
    Unpause,
    Settings,
    /// Ends the program for now; later it will return to the main menu.
    Exit,
    /// Leaves the settings screen.
    Back,
}

impl PauseItem {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unpause => "Unpause",
            Self::Settings => "Settings",
            Self::Exit => "Exit",
            Self::Back => "Back",
        }
    }
}

/// The screen the menu is showing while it is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseScreen {
    Main,
    /// Blank for now: it only has a way back.
    Settings,
}

impl PauseScreen {
    pub fn title(self) -> &'static str {
        match self {
            Self::Main => "Paused",
            Self::Settings => "Settings",
        }
    }

    /// The buttons of this screen, top to bottom.
    pub fn items(self) -> &'static [PauseItem] {
        match self {
            Self::Main => &[PauseItem::Unpause, PauseItem::Settings, PauseItem::Exit],
            Self::Settings => &[PauseItem::Back],
        }
    }
}

/// What the front end must do after a button press.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PauseOutcome {
    Continue,
    /// The program should end.
    Exit,
}

/// The pause menu. Closed until `escape` opens it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PauseMenu {
    screen: Option<PauseScreen>,
    /// Index into the open screen's items.
    selected: usize,
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

    /// The Escape key: opens the menu, closes it from the main screen, and goes back from
    /// Settings to the main screen (with Settings selected).
    pub fn escape(&mut self) {
        match self.screen {
            None => self.show(PauseScreen::Main, PauseItem::Unpause),
            Some(PauseScreen::Main) => self.screen = None,
            Some(PauseScreen::Settings) => self.show(PauseScreen::Main, PauseItem::Settings),
        }
    }

    /// Moves the selection by `steps` buttons (negative is up), wrapping around the ends.
    /// Does nothing while the menu is closed.
    pub fn step(&mut self, steps: i32) {
        let Some(screen) = self.screen else { return };
        let count = screen.items().len() as i32;
        self.selected = (self.selected as i32 + steps).rem_euclid(count) as usize;
    }

    /// Selects `item` (the mouse resting on it). An item the open screen does not have is
    /// ignored.
    pub fn select(&mut self, item: PauseItem) {
        if let Some(index) = self.index_of(item) {
            self.selected = index;
        }
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
        if self.index_of(item).is_none() {
            return PauseOutcome::Continue;
        }
        match item {
            PauseItem::Unpause => self.screen = None,
            PauseItem::Settings => self.show(PauseScreen::Settings, PauseItem::Back),
            PauseItem::Back => self.show(PauseScreen::Main, PauseItem::Settings),
            PauseItem::Exit => return PauseOutcome::Exit,
        }
        PauseOutcome::Continue
    }

    fn index_of(&self, item: PauseItem) -> Option<usize> {
        self.screen?.items().iter().position(|&other| other == item)
    }

    fn show(&mut self, screen: PauseScreen, selected: PauseItem) {
        self.screen = Some(screen);
        self.selected = screen
            .items()
            .iter()
            .position(|&item| item == selected)
            .unwrap_or(0);
    }
}

const PANEL_WIDTH: f32 = 360.0;
const TITLE_HEIGHT: f32 = 84.0;
const BOTTOM_PADDING: f32 = 32.0;
const BUTTON_SIZE: Vec2 = Vec2::new(280.0, 48.0);
const BUTTON_GAP: f32 = 16.0;

/// The menu panel on a `view`-sized screen: centered, and the same size on every screen so the
/// menu does not jump when Settings opens.
pub fn panel_rect(view: Vec2) -> Rect {
    let rows = PauseScreen::Main.items().len() as f32;
    let height = TITLE_HEIGHT + rows * BUTTON_SIZE.y + (rows - 1.0) * BUTTON_GAP + BOTTOM_PADDING;
    let top_left = anchored(
        view,
        Anchor::Center,
        Vec2::new(PANEL_WIDTH, height),
        Vec2::ZERO,
    );
    Rect::new(top_left.x, top_left.y, PANEL_WIDTH, height)
}

/// Where the title baseline of the panel sits, as a distance below the panel's top edge.
pub fn title_baseline() -> f32 {
    TITLE_HEIGHT - 28.0
}

/// The buttons of `screen` with their rectangles, top to bottom, inside the panel.
pub fn button_rects(screen: PauseScreen, view: Vec2) -> Vec<(PauseItem, Rect)> {
    let panel = panel_rect(view);
    let x = panel.x + (PANEL_WIDTH - BUTTON_SIZE.x) / 2.0;
    screen
        .items()
        .iter()
        .enumerate()
        .map(|(row, &item)| {
            let y = panel.y + TITLE_HEIGHT + row as f32 * (BUTTON_SIZE.y + BUTTON_GAP);
            (item, Rect::new(x, y, BUTTON_SIZE.x, BUTTON_SIZE.y))
        })
        .collect()
}

/// The button of `screen` under `point` (virtual-screen coordinates), if any.
pub fn item_at(screen: PauseScreen, view: Vec2, point: Vec2) -> Option<PauseItem> {
    button_rects(screen, view)
        .into_iter()
        .find(|(_, rect)| rect.contains(point))
        .map(|(item, _)| item)
}

#[cfg(test)]
mod tests;
