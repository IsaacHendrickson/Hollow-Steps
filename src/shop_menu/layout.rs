//! Where the shop panel, its rows and its text lines are, in virtual-screen coordinates. The
//! drawing and the mouse hit test share these, so the mouse lands on what is on screen.
//!
//! The panel is as tall as its content needs and is centered with `camera::anchored`. When there
//! are more upgrades than fit on the screen, the list shows a window of rows and the menu scrolls
//! it (`ShopMenu`). Nothing here depends on the window: only on the virtual `view`.

use macroquad::math::{vec2, Rect, Vec2};

use crate::camera::{anchored, Anchor};

pub const PANEL_WIDTH: f32 = 720.0;
const SIDE_PADDING: f32 = 32.0;
/// The panel stays at least this far from the top and bottom of the view, so a long list gets a
/// scrolling window instead of running off the screen.
const SCREEN_MARGIN: f32 = 12.0;

// Distances below the panel's top edge; text positions are baselines.
const TITLE_BASELINE: f32 = 50.0;
const COINS_BASELINE: f32 = 88.0;
const HEADER_BASELINE: f32 = 126.0;
const LIST_TOP: f32 = 136.0;
pub const ROW_HEIGHT: f32 = 32.0;
const ROW_GAP: f32 = 4.0;
/// From the list's heading to the note that says nothing is for sale.
const EMPTY_NOTE_GAP: f32 = 34.0;
/// From the bottom of the list to the first line of the selected upgrade's description.
const DETAIL_GAP: f32 = 28.0;
/// How many lines of description there is room for; a longer one ends in "...".
pub const DESCRIPTION_LINES: usize = 2;
/// How many lines of message there is room for.
pub const MESSAGE_LINES: usize = 2;
const DETAIL_STEP: f32 = 22.0;
const MESSAGE_GAP: f32 = 30.0;
const FOOTER_GAP: f32 = 34.0;
const BOTTOM_PADDING: f32 = 20.0;

// Columns of a row, from its left edge.
const NAME_OFFSET: f32 = 14.0;
const NAME_WIDTH: f32 = 170.0;
const LEVEL_OFFSET: f32 = 196.0;
const COST_OFFSET: f32 = 262.0;
const EFFECT_OFFSET: f32 = 342.0;
const EFFECT_INSET: f32 = 12.0;

/// One visible row of the list: which upgrade (an index into the catalog's list) and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShopRowRect {
    pub index: usize,
    pub rect: Rect,
}

/// Where everything in the open shop goes.
#[derive(Clone, Debug, PartialEq)]
pub struct ShopLayout {
    pub panel: Rect,
    /// Text baselines, at the left edge of the text.
    pub title: Vec2,
    pub coins: Vec2,
    /// The column headings sit on this baseline, over the rows' columns.
    pub header: Vec2,
    /// Where "nothing for sale" goes; only drawn when the list is empty.
    pub empty_note: Vec2,
    /// The rows that are on screen, top to bottom; none when nothing is for sale.
    pub rows: Vec<ShopRowRect>,
    /// Upgrades above and below the visible rows.
    pub more_above: bool,
    pub more_below: bool,
    /// The selected upgrade's description lines and the message lines.
    pub description: [Vec2; DESCRIPTION_LINES],
    pub message: [Vec2; MESSAGE_LINES],
    /// How wide those lines may be.
    pub text_width: f32,
    pub footer: Vec2,
}

/// The parts of one row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowColumns {
    pub name_x: f32,
    pub name_width: f32,
    pub level_x: f32,
    pub cost_x: f32,
    pub effect_x: f32,
    pub effect_width: f32,
}

/// Distances below the panel's top for a list that shows `visible` rows.
struct Offsets {
    empty_note: f32,
    description: [f32; DESCRIPTION_LINES],
    message: [f32; MESSAGE_LINES],
    footer: f32,
    height: f32,
}

fn offsets(upgrades: usize, visible: usize) -> Offsets {
    let empty_note = HEADER_BASELINE + EMPTY_NOTE_GAP;
    let mut description = [empty_note; DESCRIPTION_LINES];
    let mut message = [empty_note; MESSAGE_LINES];
    let last_line = if upgrades == 0 {
        empty_note
    } else {
        let rows = visible.max(1) as f32;
        let list_bottom = LIST_TOP + rows * ROW_HEIGHT + (rows - 1.0) * ROW_GAP;
        for (line, baseline) in description.iter_mut().enumerate() {
            *baseline = list_bottom + DETAIL_GAP + line as f32 * DETAIL_STEP;
        }
        let first_message = description[DESCRIPTION_LINES - 1] + MESSAGE_GAP;
        for (line, baseline) in message.iter_mut().enumerate() {
            *baseline = first_message + line as f32 * DETAIL_STEP;
        }
        message[MESSAGE_LINES - 1]
    };
    let footer = last_line + FOOTER_GAP;
    Offsets {
        empty_note,
        description,
        message,
        footer,
        height: footer + BOTTOM_PADDING,
    }
}

/// How many rows the list shows: all of them when the panel fits the view, else as many as fit
/// (at least one). None when nothing is for sale.
pub fn visible_rows(view: Vec2, upgrades: usize) -> usize {
    if upgrades == 0 {
        return 0;
    }
    // `max` also turns a NaN size into 0.
    let budget = (view.y - 2.0 * SCREEN_MARGIN).max(0.0);
    (1..=upgrades)
        .rev()
        .find(|&rows| offsets(upgrades, rows).height <= budget)
        .unwrap_or(1)
}

/// The shop of a game with `upgrades` upgrades on sale, on a `view`-sized screen. `first` is the
/// upgrade at the top of the list when it scrolls (`ShopMenu::first`); a value the list cannot
/// start at is pulled back.
pub fn layout(view: Vec2, upgrades: usize, first: usize) -> ShopLayout {
    let visible = visible_rows(view, upgrades);
    let offsets = offsets(upgrades, visible);
    let size = vec2(PANEL_WIDTH, offsets.height);
    let top_left = anchored(view, Anchor::Center, size, Vec2::ZERO);
    let panel = Rect::new(top_left.x, top_left.y, size.x, size.y);
    let x = panel.x + SIDE_PADDING;
    let at = |down: f32| vec2(x, panel.y + down);
    let first = first.min(upgrades - visible);
    let text_width = PANEL_WIDTH - 2.0 * SIDE_PADDING;
    let rows = (0..visible)
        .map(|row| ShopRowRect {
            index: first + row,
            rect: Rect::new(
                x,
                panel.y + LIST_TOP + row as f32 * (ROW_HEIGHT + ROW_GAP),
                text_width,
                ROW_HEIGHT,
            ),
        })
        .collect();
    ShopLayout {
        panel,
        title: at(TITLE_BASELINE),
        coins: at(COINS_BASELINE),
        header: at(HEADER_BASELINE),
        empty_note: at(offsets.empty_note),
        rows,
        more_above: first > 0,
        more_below: first + visible < upgrades,
        description: offsets.description.map(at),
        message: offsets.message.map(at),
        text_width,
        footer: at(offsets.footer),
    }
}

/// The upgrade whose row is under `point` (virtual-screen coordinates), if any.
pub fn upgrade_at(layout: &ShopLayout, point: Vec2) -> Option<usize> {
    layout
        .rows
        .iter()
        .find(|row| row.rect.contains(point))
        .map(|row| row.index)
}

/// The columns of `row`: name, level, price and effect.
pub fn row_columns(row: Rect) -> RowColumns {
    let effect_x = row.x + EFFECT_OFFSET;
    RowColumns {
        name_x: row.x + NAME_OFFSET,
        name_width: NAME_WIDTH,
        level_x: row.x + LEVEL_OFFSET,
        cost_x: row.x + COST_OFFSET,
        effect_x,
        effect_width: row.right() - EFFECT_INSET - effect_x,
    }
}
