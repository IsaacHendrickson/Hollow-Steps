//! Room preview: a dev tool that prints a room as text (`docs/room-tools.md`). It is not part of
//! the game. `render` is a pure function from a [`Scene`] (a layout plus what a loaded room adds)
//! and a [`Ruler`] to a `String`; `load` is the only part that reads files, and it uses the
//! game's own room loading, so the picture is what the game would load.
//!
//! The text has four parts: a header (room, size, cell, the ruler with the player and the reach
//! numbers), the picture, a table with one row per placed thing, and the legend.
mod grid;
pub mod load;
mod ruler;
mod scene;
mod table;
#[cfg(test)]
mod tests;

pub use grid::columns;
pub use ruler::Ruler;
pub use scene::{room_label, BossView, EnemyView, Scene};

/// The smallest cell: one pixel per character.
pub const MIN_CELL: f32 = 1.0;
/// The widest picture the default cell allows, in characters.
pub const DEFAULT_MAX_COLUMNS: usize = 200;
/// The most characters wide or high a picture may be (`check_cell`).
pub const MAX_PICTURE_SIDE: usize = 1000;
/// The cells `default_cell` chooses from, in pixels per character.
const NICE_CELLS: [f32; 5] = [4.0, 8.0, 16.0, 32.0, 64.0];

/// The default cell for a set of rooms whose widest is `widest` px: the smallest of 4, 8, 16, 32
/// and 64 px per character that keeps it within [`DEFAULT_MAX_COLUMNS`] characters (a wider set
/// gets a cell that fits exactly).
pub fn default_cell(widest: f32) -> f32 {
    NICE_CELLS
        .into_iter()
        .find(|cell| columns(widest, *cell) <= DEFAULT_MAX_COLUMNS)
        .unwrap_or_else(|| (widest / DEFAULT_MAX_COLUMNS as f32).ceil().max(MIN_CELL))
}

/// `cell` if it is a usable number of pixels per character for `scene`'s room, else why not.
pub fn check_cell(scene: &Scene, cell: f32) -> Result<f32, String> {
    if !cell.is_finite() || cell < MIN_CELL {
        return Err(format!(
            "--cell must be a number of at least {MIN_CELL} px, not {cell}"
        ));
    }
    let bounds = scene.layout.rect();
    let (cols, rows) = (columns(bounds.w, cell), columns(bounds.h, cell));
    if cols > MAX_PICTURE_SIDE || rows > MAX_PICTURE_SIDE {
        return Err(format!(
            "--cell {cell} would draw {cols} x {rows} characters (at most {MAX_PICTURE_SIDE} each way); use a bigger cell"
        ));
    }
    Ok(cell)
}

/// The room as text. A `cell` that is not at least [`MIN_CELL`] is treated as that.
pub fn render(scene: &Scene, ruler: &Ruler, cell: f32) -> String {
    let cell = if cell.is_finite() {
        cell.max(MIN_CELL)
    } else {
        MIN_CELL
    };
    let things = scene.things();
    let picture = grid::draw(scene, &things, cell);
    let mut out = header(scene, ruler, cell);
    out.push(String::new());
    out.extend(picture);
    out.push(String::new());
    out.push("Placed things (x, y, w, h in px):".to_owned());
    out.extend(table::table(&things));
    out.push(String::new());
    out.extend(table::LEGEND.iter().map(|line| (*line).to_owned()));
    out.join("\n") + "\n"
}

fn header(scene: &Scene, ruler: &Ruler, cell: f32) -> Vec<String> {
    let layout = &scene.layout;
    let name = match (scene.id, &layout.name) {
        (Some(id), Some(name)) => format!("{} ({name:?})", room_label(id)),
        (Some(id), None) => room_label(id),
        (None, Some(name)) => format!("{name:?}"),
        (None, None) => "(unsaved room)".to_owned(),
    };
    let (x, y, w, h) = layout.bounds;
    let mut lines = vec![
        format!(
            "Room {name}: {w:.0} x {h:.0} px at ({x:.0}, {y:.0}), floor y {:.0}",
            layout.floor_y
        ),
        format!(
            "Cell: {cell} px per character; the picture is {} x {} characters",
            columns(w, cell),
            columns(h, cell)
        ),
    ];
    lines.extend(ruler.lines(cell));
    lines
}
