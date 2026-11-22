//! The one place a world is built from the assets folder: the rooms (the three fixed ones and
//! the saved custom ones) with their enemies, bosses, coins, items and lying charms, checked
//! against each other, plus the terrain look presets they choose from.
//!
//! Start-up (`try_new`), the F5 reload and a run's reset all call `load_world`, so a room is
//! rebuilt from its file the same way each time. The difficulty layer goes over the rooms here
//! and nowhere below: `load_rooms` and `load_extra_rooms` (and so the editor) never see a tier.
use std::path::Path;

use super::tier_link::Layer;
use super::{terrain_link, GameState};
use crate::charms::CharmCatalog;
use crate::editor;
use crate::enemy::EnemyDefinitions;
use crate::room::{load_rooms, Room};
use crate::terrain_styles::TerrainStyles;

/// Every room built fresh from its file with the difficulty `layer` over it, and the terrain
/// presets. Nothing is checked about the player or the run here; an error names the file that
/// failed.
pub(super) fn load_world(
    assets: &Path,
    charms: &CharmCatalog,
    layer: &Layer,
) -> Result<(Vec<Room>, TerrainStyles), String> {
    let mut rooms = load_rooms(assets)?.into_iter().collect::<Vec<_>>();
    let definitions = EnemyDefinitions::load(assets)?;
    rooms.extend(editor::load_extra_rooms(assets, &definitions)?);
    GameState::validate_destinations(&rooms, &definitions)?;
    GameState::validate_room_charms(&rooms, charms)?;
    let terrain_styles = terrain_link::load(assets, &rooms)?;
    layer.apply(&mut rooms);
    Ok((rooms, terrain_styles))
}
