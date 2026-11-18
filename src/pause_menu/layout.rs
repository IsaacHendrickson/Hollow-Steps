//! Where the pause menu's panel, buttons, key slots and message line are, in virtual-screen
//! coordinates. The drawing and the mouse hit test share these, so the mouse lands on what is
//! on screen.

use macroquad::math::{Rect, Vec2};

use super::{PauseItem, PauseScreen};
use crate::camera::{anchored, Anchor};
use crate::keybinds::{Action, Slot};

const PANEL_WIDTH: f32 = 360.0;
const TITLE_HEIGHT: f32 = 84.0;
const BOTTOM_PADDING: f32 = 32.0;
const BUTTON_SIZE: Vec2 = Vec2::new(280.0, 48.0);
const BUTTON_GAP: f32 = 16.0;

// The Settings screen lists every action, so its panel is wider and its rows are smaller.
const SETTINGS_WIDTH: f32 = 620.0;
const SETTINGS_TITLE_HEIGHT: f32 = 72.0;
const ROW_HEIGHT: f32 = 30.0;
const ROW_GAP: f32 = 4.0;
/// The space before Reset keybinds, which sets it apart from the keybind rows.
const GROUP_GAP: f32 = 14.0;
const SIDE_PADDING: f32 = 24.0;
const SETTINGS_BUTTON_WIDTH: f32 = 280.0;
const STATUS_HEIGHT: f32 = 34.0;
const SETTINGS_BOTTOM_PADDING: f32 = 8.0;
const SLOT_WIDTH: f32 = 170.0;
const SLOT_GAP: f32 = 10.0;

/// The panel of `screen` on a `view`-sized screen, centered. The Settings panel is bigger than
/// the main one.
pub fn panel_rect(screen: PauseScreen, view: Vec2) -> Rect {
    let rows = screen.items().len() as f32;
    let (width, height) = match screen {
        PauseScreen::Main => (
            PANEL_WIDTH,
            TITLE_HEIGHT + rows * BUTTON_SIZE.y + (rows - 1.0) * BUTTON_GAP + BOTTOM_PADDING,
        ),
        PauseScreen::Settings => (
            SETTINGS_WIDTH,
            SETTINGS_TITLE_HEIGHT
                + rows * ROW_HEIGHT
                + (rows - 1.0) * ROW_GAP
                + (GROUP_GAP - ROW_GAP)
                + STATUS_HEIGHT
                + SETTINGS_BOTTOM_PADDING,
        ),
    };
    let top_left = anchored(view, Anchor::Center, Vec2::new(width, height), Vec2::ZERO);
    Rect::new(top_left.x, top_left.y, width, height)
}

/// Where the title baseline of the panel sits, as a distance below the panel's top edge.
pub fn title_baseline() -> f32 {
    TITLE_HEIGHT - 28.0
}

/// The buttons of `screen` with their rectangles, top to bottom, inside the panel. A keybind
/// row's rectangle is the whole row; its two key buttons are `slot_rects` of it.
pub fn button_rects(screen: PauseScreen, view: Vec2) -> Vec<(PauseItem, Rect)> {
    let panel = panel_rect(screen, view);
    match screen {
        PauseScreen::Main => {
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
        PauseScreen::Settings => {
            let mut y = panel.y + SETTINGS_TITLE_HEIGHT;
            let mut rects = Vec::new();
            for (row, &item) in screen.items().iter().enumerate() {
                if row == Action::ALL.len() {
                    y += GROUP_GAP - ROW_GAP;
                }
                let width = match item {
                    PauseItem::Bind(_) => panel.w - 2.0 * SIDE_PADDING,
                    _ => SETTINGS_BUTTON_WIDTH,
                };
                let x = panel.x + (panel.w - width) / 2.0;
                rects.push((item, Rect::new(x, y, width, ROW_HEIGHT)));
                y += ROW_HEIGHT + ROW_GAP;
            }
            rects
        }
    }
}

/// The primary and alternate key buttons at the right end of a keybind `row`, in `Slot::ALL`
/// order.
pub fn slot_rects(row: Rect) -> [Rect; 2] {
    let alternate = Rect::new(row.right() - SLOT_WIDTH, row.y, SLOT_WIDTH, row.h);
    let primary = Rect::new(
        alternate.x - SLOT_GAP - SLOT_WIDTH,
        row.y,
        SLOT_WIDTH,
        row.h,
    );
    [primary, alternate]
}

/// The message line of the Settings screen, under the last button.
pub fn status_rect(view: Vec2) -> Rect {
    let panel = panel_rect(PauseScreen::Settings, view);
    Rect::new(
        panel.x + SIDE_PADDING,
        panel.bottom() - SETTINGS_BOTTOM_PADDING - STATUS_HEIGHT,
        panel.w - 2.0 * SIDE_PADDING,
        STATUS_HEIGHT,
    )
}

/// The button of `screen` under `point` (virtual-screen coordinates), if any.
pub fn item_at(screen: PauseScreen, view: Vec2, point: Vec2) -> Option<PauseItem> {
    button_rects(screen, view)
        .into_iter()
        .find(|(_, rect)| rect.contains(point))
        .map(|(item, _)| item)
}

/// The key button of a keybind row under `point`, if any.
pub fn slot_at(screen: PauseScreen, view: Vec2, point: Vec2) -> Option<(Action, Slot)> {
    button_rects(screen, view)
        .into_iter()
        .find_map(|(item, row)| {
            let PauseItem::Bind(action) = item else {
                return None;
            };
            let rects = slot_rects(row);
            Slot::ALL
                .into_iter()
                .find(|slot| rects[slot.index()].contains(point))
                .map(|slot| (action, slot))
        })
}
