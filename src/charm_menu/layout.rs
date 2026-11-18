//! Where the inventory panel, its items and its Charms section are, in virtual-screen
//! coordinates. The drawing and the mouse hit test share these, so the mouse lands on what is
//! on screen.
//!
//! The panel is as tall as its content needs and is centered with `camera::anchored`. When
//! there are more charms than fit on the screen, the list shows a window of rows and the menu
//! scrolls it (`CharmMenu`). Nothing here depends on the window: only on the virtual `view`.

use macroquad::math::{vec2, Rect, Vec2};

use crate::camera::{anchored, Anchor};

pub const PANEL_WIDTH: f32 = 640.0;
const SIDE_PADDING: f32 = 32.0;
/// The panel stays at least this far from the top and bottom of the view, so a long list gets a
/// scrolling window instead of running off the screen.
const SCREEN_MARGIN: f32 = 12.0;

// Distances below the panel's top edge; text positions are baselines.
const TITLE_BASELINE: f32 = 50.0;
const COINS_BASELINE: f32 = 94.0;
const ITEMS_HEADER_BASELINE: f32 = 140.0;
const ITEMS_FIRST_BASELINE: f32 = 176.0;
/// The distance between two collected items.
pub const ITEM_STEP: f32 = 28.0;
/// From the last item's baseline to the Charms heading's baseline.
const HEADER_GAP: f32 = 40.0;
/// From the Charms heading's baseline to the top of the first row.
const LIST_TOP_GAP: f32 = 10.0;
pub const ROW_HEIGHT: f32 = 28.0;
const ROW_GAP: f32 = 4.0;
/// From the Charms heading's baseline to the note that says no charm is owned.
const EMPTY_NOTE_GAP: f32 = 30.0;
/// From the bottom of the list to the first line of the selected charm's description.
const DETAIL_GAP: f32 = 24.0;
/// How many lines of description there is room for; a longer one ends in "...".
pub const DESCRIPTION_LINES: usize = 2;
/// How many lines of refusal message there is room for.
pub const MESSAGE_LINES: usize = 2;
const DETAIL_STEP: f32 = 22.0;
const MANA_GAP: f32 = 28.0;
const FOOTER_GAP: f32 = 30.0;
const BOTTOM_PADDING: f32 = 20.0;

// Columns of a charm row, from its left edge.
const SWATCH_SIZE: f32 = 16.0;
const SWATCH_INSET: f32 = 8.0;
const NAME_OFFSET: f32 = 34.0;
const NAME_WIDTH: f32 = 180.0;
const COST_OFFSET: f32 = 222.0;
const EFFECT_OFFSET: f32 = 306.0;
const MARKER_WIDTH: f32 = 88.0;
const MARKER_INSET: f32 = 10.0;
const COLUMN_GAP: f32 = 8.0;

/// One visible row of the charm list: which owned charm (an index into the owned list) and where.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharmRow {
    pub index: usize,
    pub rect: Rect,
}

/// Where everything in the open inventory goes.
#[derive(Clone, Debug, PartialEq)]
pub struct InventoryLayout {
    pub panel: Rect,
    /// Text baselines, at the left edge of the text.
    pub title: Vec2,
    pub coins: Vec2,
    pub items_header: Vec2,
    pub items_first: Vec2,
    pub charms_header: Vec2,
    /// Where "no charm owned" goes; only drawn when the list is empty.
    pub empty_note: Vec2,
    /// The rows that are on screen, top to bottom; none when no charm is owned.
    pub rows: Vec<CharmRow>,
    /// Owned charms above and below the visible rows.
    pub more_above: bool,
    pub more_below: bool,
    /// The selected charm's description lines, its mana line and the refusal message lines.
    pub description: [Vec2; DESCRIPTION_LINES],
    pub mana: Vec2,
    pub message: [Vec2; MESSAGE_LINES],
    /// How wide those lines may be.
    pub text_width: f32,
    pub footer: Vec2,
}

impl InventoryLayout {
    /// The baseline of collected item number `index`.
    pub fn item(&self, index: usize) -> Vec2 {
        self.items_first + vec2(0.0, index as f32 * ITEM_STEP)
    }
}

/// The parts of one charm row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowColumns {
    pub swatch: Rect,
    pub name_x: f32,
    pub name_width: f32,
    pub cost_x: f32,
    pub effect_x: f32,
    pub effect_width: f32,
    /// The right edge of the "Equipped" marker.
    pub marker_right: f32,
}

/// Distances below the panel's top for a list that shows `visible` rows.
struct Offsets {
    charms_header: f32,
    empty_note: f32,
    list_top: f32,
    description: [f32; DESCRIPTION_LINES],
    mana: f32,
    message: [f32; MESSAGE_LINES],
    footer: f32,
    height: f32,
}

fn offsets(items: usize, charms: usize, visible: usize) -> Offsets {
    let last_item = ITEMS_FIRST_BASELINE + (items.max(1) - 1) as f32 * ITEM_STEP;
    let charms_header = last_item + HEADER_GAP;
    let list_top = charms_header + LIST_TOP_GAP;
    let empty_note = charms_header + EMPTY_NOTE_GAP;
    let mut description = [0.0; DESCRIPTION_LINES];
    let mut message = [empty_note; MESSAGE_LINES];
    let (mana, last_line);
    if charms == 0 {
        mana = empty_note;
        last_line = empty_note;
    } else {
        let rows = visible.max(1) as f32;
        let list_bottom = list_top + rows * ROW_HEIGHT + (rows - 1.0) * ROW_GAP;
        for (line, baseline) in description.iter_mut().enumerate() {
            *baseline = list_bottom + DETAIL_GAP + line as f32 * DETAIL_STEP;
        }
        mana = description[DESCRIPTION_LINES - 1] + MANA_GAP;
        for (line, baseline) in message.iter_mut().enumerate() {
            *baseline = mana + (line + 1) as f32 * DETAIL_STEP;
        }
        last_line = message[MESSAGE_LINES - 1];
    }
    let footer = last_line + FOOTER_GAP;
    Offsets {
        charms_header,
        empty_note,
        list_top,
        description,
        mana,
        message,
        footer,
        height: footer + BOTTOM_PADDING,
    }
}

/// How many charm rows the list shows: all of them when the panel fits the view, else as many
/// as fit (at least one). None when no charm is owned.
pub fn visible_rows(view: Vec2, items: usize, charms: usize) -> usize {
    if charms == 0 {
        return 0;
    }
    // `max` also turns a NaN size into 0.
    let budget = (view.y - 2.0 * SCREEN_MARGIN).max(0.0);
    (1..=charms)
        .rev()
        .find(|&rows| offsets(items, charms, rows).height <= budget)
        .unwrap_or(1)
}

/// The inventory of a game with `items` collected items and `charms` owned charms, on a
/// `view`-sized screen. `first` is the owned charm at the top of the list when it scrolls
/// (`CharmMenu::first`); a value the list cannot start at is pulled back.
pub fn layout(view: Vec2, items: usize, charms: usize, first: usize) -> InventoryLayout {
    let visible = visible_rows(view, items, charms);
    let offsets = offsets(items, charms, visible);
    let size = vec2(PANEL_WIDTH, offsets.height);
    let top_left = anchored(view, Anchor::Center, size, Vec2::ZERO);
    let panel = Rect::new(top_left.x, top_left.y, size.x, size.y);
    let x = panel.x + SIDE_PADDING;
    let at = |down: f32| vec2(x, panel.y + down);
    let first = first.min(charms - visible);
    let text_width = PANEL_WIDTH - 2.0 * SIDE_PADDING;
    let rows = (0..visible)
        .map(|row| CharmRow {
            index: first + row,
            rect: Rect::new(
                x,
                panel.y + offsets.list_top + row as f32 * (ROW_HEIGHT + ROW_GAP),
                text_width,
                ROW_HEIGHT,
            ),
        })
        .collect();
    InventoryLayout {
        panel,
        title: at(TITLE_BASELINE),
        coins: at(COINS_BASELINE),
        items_header: at(ITEMS_HEADER_BASELINE),
        items_first: at(ITEMS_FIRST_BASELINE),
        charms_header: at(offsets.charms_header),
        empty_note: at(offsets.empty_note),
        rows,
        more_above: first > 0,
        more_below: first + visible < charms,
        description: offsets.description.map(at),
        mana: at(offsets.mana),
        message: offsets.message.map(at),
        text_width,
        footer: at(offsets.footer),
    }
}

/// The owned charm whose row is under `point` (virtual-screen coordinates), if any.
pub fn charm_at(layout: &InventoryLayout, point: Vec2) -> Option<usize> {
    layout
        .rows
        .iter()
        .find(|row| row.rect.contains(point))
        .map(|row| row.index)
}

/// The columns of `row`: color swatch, name, mana cost, effect and the "Equipped" marker.
pub fn row_columns(row: Rect) -> RowColumns {
    let marker_right = row.right() - MARKER_INSET;
    let effect_x = row.x + EFFECT_OFFSET;
    RowColumns {
        swatch: Rect::new(
            row.x + SWATCH_INSET,
            row.center().y - SWATCH_SIZE / 2.0,
            SWATCH_SIZE,
            SWATCH_SIZE,
        ),
        name_x: row.x + NAME_OFFSET,
        name_width: NAME_WIDTH,
        cost_x: row.x + COST_OFFSET,
        effect_x,
        effect_width: marker_right - MARKER_WIDTH - COLUMN_GAP - effect_x,
        marker_right,
    }
}
