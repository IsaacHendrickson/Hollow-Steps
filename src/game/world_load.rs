//! The one place a world is built from the assets folder: the rooms (the three fixed ones and
//! the saved custom ones) with their enemies, bosses, coins, items and lying charms, checked
//! against each other, plus the terrain look presets they choose from.
//!
//! Start-up (`try_new`), the F5 reload and a run's reset all call `load_world`, so a room is
//! rebuilt from its file the same way each time. The difficulty layer goes over the rooms here
//! and nowhere below: `load_rooms` and `load_extra_rooms` (and so the editor) never see a tier.
//! The flips are the same kind of thing: when the layer says flip the dungeon, or mirror some
//! flippable rooms, the loaders are handed the room documents already mirrored
//! (`mirror::FlippedDocuments`, never the hub) and build from them as they always do. The editor
//! and the files never see it.
use std::path::Path;

use super::tier_link::Layer;
use super::{terrain_link, GameState};
use crate::charms::CharmCatalog;
use crate::editor_files::{load_extra_rooms_from, read_document, ReadDocument};
use crate::enemy::EnemyDefinitions;
use crate::mirror::FlippedDocuments;
use crate::room::{load_rooms_from, Room, RoomId};
use crate::terrain_styles::TerrainStyles;
use crate::variety::RoomFlips;

/// Every room built fresh from its file with the difficulty `layer` over it, and the terrain
/// presets. When `layer.flip` is set every room but `hub` is built mirrored, and the flippable
/// rooms `layer.room_flips` names are mirrored over that (a game with no hub passes `None`, and
/// never sets either). Nothing is checked about the player or the run here; an
/// error names the file that failed.
pub(super) fn load_world(
    assets: &Path,
    charms: &CharmCatalog,
    layer: &Layer,
    hub: Option<RoomId>,
) -> Result<(Vec<Room>, TerrainStyles), String> {
    let mirrors = layer.flip || layer.room_flips != RoomFlips::Off;
    let flipped =
        mirrors.then(|| FlippedDocuments::read_with(assets, hub, layer.flip, layer.room_flips));
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
