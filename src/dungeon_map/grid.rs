//! Where the cells of the map go on the screen: the panel the map is drawn in, and the scale that
//! fits a block of cells into it. Pure: the drawing (`hud_map` in play, the Dungeon map window in
//! the editor) and any mouse hit test share these numbers. Nothing here depends on the window,
//! only on the virtual `view` or the rectangle it is given.
use macroquad::math::{vec2, Rect, Vec2};

use super::Cell;
use crate::camera::{anchored, Anchor};

/// The largest a cell can be drawn: a room's box plus the space around it for links. A map of
/// one or two rooms stays this size instead of filling the panel.
pub const MAX_PITCH: f32 = 112.0;
/// The share of a cell a room's box takes; the rest is the gap links cross.
pub const BOX_FRACTION: f32 = 0.62;

/// The panel stays this far from every edge of the view.
const SCREEN_MARGIN: f32 = 36.0;
const TITLE_HEIGHT: f32 = 60.0;
const FOOTER_HEIGHT: f32 = 44.0;
const SIDE_PADDING: f32 = 24.0;
/// From the panel's top edge to the title's baseline.
const TITLE_BASELINE: f32 = 40.0;
/// From the panel's bottom edge up to the footer's baseline.
const FOOTER_BASELINE: f32 = 16.0;

/// The map screen: the panel over the world, and where its pieces sit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapScreen {
    pub panel: Rect,
    /// Where the cells are drawn, inside the panel between the title and the footer.
    pub area: Rect,
    /// Text baselines at the left edge of the text.
    pub title: Vec2,
    pub footer: Vec2,
}

/// The map screen on a `view`-sized screen: the whole view less a margin, centered.
pub fn screen_layout(view: Vec2) -> MapScreen {
    let size = (view - Vec2::splat(2.0 * SCREEN_MARGIN)).max(Vec2::ONE);
    let top_left = anchored(view, Anchor::Center, size, Vec2::ZERO);
    let panel = Rect::new(top_left.x, top_left.y, size.x, size.y);
    let area = Rect::new(
        panel.x + SIDE_PADDING,
        panel.y + TITLE_HEIGHT,
        (panel.w - 2.0 * SIDE_PADDING).max(0.0),
        (panel.h - TITLE_HEIGHT - FOOTER_HEIGHT).max(0.0),
    );
    MapScreen {
        panel,
        area,
        title: vec2(panel.x + SIDE_PADDING, panel.y + TITLE_BASELINE),
        footer: vec2(panel.x + SIDE_PADDING, panel.bottom() - FOOTER_BASELINE),
    }
}

/// A block of cells scaled to fit an area and centered in it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapGrid {
    /// The column and row at the block's top-left.
    min: Cell,
    pitch: f32,
    /// Top-left of the block.
    origin: Vec2,
}

impl MapGrid {
    /// Fits the smallest block holding every cell of `cells` into `area`: as large as fits, but
    /// no cell larger than `MAX_PITCH`, and the block centered. No cells, or an area with no
    /// size, give a grid of size zero.
    pub fn fit(area: Rect, cells: &[Cell]) -> Self {
        let min = cells
            .iter()
            .copied()
            .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1)))
            .unwrap_or((0, 0));
        let max = cells
            .iter()
            .copied()
            .reduce(|a, b| (a.0.max(b.0), a.1.max(b.1)))
            .unwrap_or(min);
        let columns = (i64::from(max.0) - i64::from(min.0) + 1) as f32;
        let rows = (i64::from(max.1) - i64::from(min.1) + 1) as f32;
        let pitch = if cells.is_empty() {
            0.0
        } else {
            (area.w / columns).min(area.h / rows).clamp(0.0, MAX_PITCH)
        };
        let block = vec2(columns * pitch, rows * pitch);
        Self {
            min,
            pitch,
            origin: area.center() - block / 2.0,
        }
    }

    /// One cell's width and height, including the gap around its box.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// The side of a room's box.
    pub fn box_size(&self) -> f32 {
        self.pitch * BOX_FRACTION
    }

    /// The center of `cell`'s box.
    pub fn center(&self, cell: Cell) -> Vec2 {
        let column = (i64::from(cell.0) - i64::from(self.min.0)) as f32;
        let row = (i64::from(cell.1) - i64::from(self.min.1)) as f32;
        self.origin + vec2(column + 0.5, row + 0.5) * self.pitch
    }

    /// The box of the room in `cell`.
    pub fn room_rect(&self, cell: Cell) -> Rect {
        let side = self.box_size();
        let center = self.center(cell);
        Rect::new(center.x - side / 2.0, center.y - side / 2.0, side, side)
    }
}
