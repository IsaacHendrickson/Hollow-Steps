//! The one place a world is built from the assets folder: the rooms (the three fixed ones and
//! the saved custom ones) with their enemies, bosses, coins, items and lying charms, checked
//! against each other, plus the terrain look presets they choose from.
//!
//! Start-up (`try_new`), the F5 reload and a run's reset all call `load_world`, so a room is
//! rebuilt from its file the same way each time. The difficulty layer goes over the rooms here
//! and nowhere below: `load_rooms` and `load_extra_rooms` (and so the editor) never see a tier.
//! The dungeon flip is the same kind of thing: when the layer says flip, the loaders are handed
//! the room documents already mirrored (`mirror::FlippedDocuments`, every room but the hub) and
//! build from them as they always do. The editor and the files never see it.
use std::path::Path;

use super::tier_link::Layer;
use super::{terrain_link, GameState};
use crate::charms::CharmCatalog;
use crate::editor_files::{load_extra_rooms_from, read_document, ReadDocument};
use crate::enemy::EnemyDefinitions;
use crate::mirror::FlippedDocuments;
use crate::room::{load_rooms_from, Room, RoomId};
use crate::terrain_styles::TerrainStyles;

/// Every room built fresh from its file with the difficulty `layer` over it, and the terrain
/// presets. When `layer.flip` is set every room but `hub` is built mirrored (a game with no hub
/// passes `None`, and never sets it). Nothing is checked about the player or the run here; an
/// error names the file that failed.
pub(super) fn load_world(
    assets: &Path,
    charms: &CharmCatalog,
    layer: &Layer,
    hub: Option<RoomId>,
) -> Result<(Vec<Room>, TerrainStyles), String> {
    let flipped = layer.flip.then(|| FlippedDocuments::read(assets, hub));
    let flipped_read = |path: &Path| flipped.as_ref().map(|set| set.document(path));
    let read: ReadDocument = &|path| flipped_read(path).unwrap_or_else(|| read_document(path));
    let mut rooms = load_rooms_from(assets, read)?
        .into_iter()
        .collect::<Vec<_>>();
    let definitions = EnemyDefinitions::load(assets)?;
    rooms.extend(load_extra_rooms_from(assets, &definitions, read)?);
    GameState::validate_destinations(&rooms, &definitions)?;
    GameState::validate_room_charms(&rooms, charms)?;
    let terrain_styles = terrain_link::load(assets, &rooms)?;
    layer.apply(&mut rooms);
    Ok((rooms, terrain_styles))
}
