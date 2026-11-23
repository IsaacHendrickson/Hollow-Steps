//! How `GameState` keeps the terrain look presets (`crate::terrain_styles`): loaded with the
//! rooms that choose from them, so a room naming a preset that does not exist stops the game
//! from loading (or reloading) instead of drawing something else, and handed to the front end
//! to resolve each room's look from.

use std::path::Path;

use super::GameState;
use crate::editor::room_filename;
use crate::room::{Room, RoomId};
use crate::terrain_styles::TerrainStyles;

/// Loads the presets from `assets` and checks that every room's style resolves against them.
/// An error names each room file that does not (an unknown preset lists the known ones).
pub(super) fn load(assets: &Path, rooms: &[Room]) -> Result<TerrainStyles, String> {
    load_named(assets, rooms, &|id| id)
}

/// `load`, naming in an error the file of the room `file_of` gives for each room's id: a
/// generated world's rooms are named by the piece they were built from, not by their own id.
pub(super) fn load_named(
    assets: &Path,
    rooms: &[Room],
    file_of: &dyn Fn(RoomId) -> RoomId,
) -> Result<TerrainStyles, String> {
    let styles = TerrainStyles::load(assets)?;
    let errors: Vec<String> = rooms
        .iter()
        .filter_map(|room| {
            let file = assets.join("rooms").join(room_filename(file_of(room.id)));
            let error = styles.resolve(&room.terrain_style).err()?;
            Some(format!("{}: {error}", file.display()))
        })
        .collect();
    if errors.is_empty() {
        Ok(styles)
    } else {
        Err(errors.join("\n"))
    }
}

impl GameState {
    /// The presets rooms choose their terrain look from: the `terrain_styles.ron` in the assets
    /// folder if there is one, otherwise the built-in `data/terrain_styles.ron`. Resolve a
    /// room's look with `TerrainStyles::resolve_or_default(&room.terrain_style)`.
    pub fn terrain_styles(&self) -> &TerrainStyles {
        &self.terrain_styles
    }
}
