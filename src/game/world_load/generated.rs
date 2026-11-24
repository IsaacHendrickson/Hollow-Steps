//! The world of a generated run (`docs/dungeon-gen.md`): the files read for the generator, and
//! the rooms built from the plan it returns. Called from `world_load`, so a generated world is
//! built, checked and layered the way every other world is, and nowhere else.
//!
//! Only files are read here, never the rooms in play: the pool is every room file whose layout
//! carries a piece mark, as the file holds it, so an unsaved editor draft or a room the player
//! has changed in play never reaches the generator. A planned room is built from a `RoomDraft`
//! made of the plan's layout and the enemy and boss placements of the piece's own file, mirrored
//! with `mirror_document` when the plan placed the piece mirrored, with `RoomDraft::build` and
//! `place_boss` exactly as `load_extra_rooms_from` builds a saved room.
use std::path::Path;

use super::super::tier_link::Layer;
use super::super::{terrain_link, GameState};
use crate::boss_definition::BossDefinitions;
use crate::charms::CharmCatalog;
use crate::dungeon_gen::DungeonPlan;
use crate::editor::{RoomDraft, RoomLayout};
use crate::editor_files::{read_document, room_filename};
use crate::enemy::EnemyDefinitions;
use crate::mirror::{mirror_document, room_ids};
use crate::piece::PieceRole;
use crate::room::{place_boss, Room, RoomEnemies, RoomId};
use crate::terrain_styles::TerrainStyles;

/// What the generator is given, read from the room files.
#[derive(Debug)]
pub(in crate::game) struct Sources {
    /// Every room whose file carries a piece mark, with its layout as the file has it.
    pub(in crate::game) pieces: Vec<(RoomId, RoomLayout)>,
    /// The hub room and its layout.
    pub(in crate::game) hub: (RoomId, RoomLayout),
}

impl Sources {
    /// The pool has a Start piece and a Boss piece, the least a plan is built from.
    pub(in crate::game) fn has_ends(&self) -> bool {
        [PieceRole::Start, PieceRole::Boss].into_iter().all(|role| {
            self.pieces
                .iter()
                .any(|(_, layout)| layout.piece.is_some_and(|piece| piece.role == role))
        })
    }
}

/// Reads every room file under `assets/rooms` and sorts out the pieces and the hub. A file that
/// cannot be read stops the run from starting with the reader's own error, like it does for the
/// fixed dungeon. A hub whose file has no layout is taken from `live_hub`, the hub in play.
pub(in crate::game) fn read_sources(
    assets: &Path,
    hub: RoomId,
    live_hub: Option<&Room>,
) -> Result<Sources, String> {
    let folder = assets.join("rooms");
    let mut pieces = Vec::new();
    let mut hub_layout = None;
    for id in room_ids(&folder) {
        let Some(layout) = read_document(&folder.join(room_filename(id)))?.layout else {
            continue;
        };
        if id == hub {
            hub_layout = Some(layout.clone());
        }
        if layout.piece.is_some() {
            pieces.push((id, layout));
        }
    }
    let hub_layout = hub_layout
        .or_else(|| live_hub.map(RoomLayout::from_room))
        .ok_or_else(|| format!("the hub room {hub:?} has no room file to build a dungeon from"))?;
    Ok(Sources {
        pieces,
        hub: (hub, hub_layout),
    })
}

/// The world of `plan`: the plan's rewired hub and its rooms, in that order, and nothing else.
/// Every other room is absent for the run. Built from the plan's layouts with the piece files'
/// own enemies and bosses, checked like any world (destinations, charms, terrain looks), and
/// layered with `layer` (min tier, groups, chances and the difficulty scale). An error names the
/// file of the piece that failed.
pub(super) fn load_generated(
    assets: &Path,
    charms: &CharmCatalog,
    layer: &Layer,
    plan: &DungeonPlan,
) -> Result<(Vec<Room>, TerrainStyles), String> {
    let definitions = EnemyDefinitions::load(assets)?;
    let bosses = BossDefinitions::load(assets)?;
    let folder = assets.join("rooms");
    let mut rooms = Vec::with_capacity(plan.rooms.len() + 1);

    let hub_path = folder.join(room_filename(plan.hub_room));
    let hub_document = read_document(&hub_path)?;
    rooms.push(build(
        plan.hub_room,
        hub_document,
        &plan.hub,
        &Parts {
            definitions: &definitions,
            bosses: &bosses,
            path: &hub_path,
        },
    )?);
    for planned in &plan.rooms {
        let path = folder.join(room_filename(planned.piece));
        let document = read_document(&path)?;
        let document = if planned.mirrored {
            mirror_document(&document)
        } else {
            document
        };
        rooms.push(build(
            planned.instance,
            document,
            &planned.layout,
            &Parts {
                definitions: &definitions,
                bosses: &bosses,
                path: &path,
            },
        )?);
    }

    GameState::validate_destinations(&rooms, &definitions)?;
    GameState::validate_room_charms(&rooms, charms)?;
    let piece_of = |id: RoomId| plan.room(id).map_or(id, |planned| planned.piece);
    let terrain_styles = terrain_link::load_named(assets, &rooms, &piece_of)?;
    layer.apply(&mut rooms);
    Ok((rooms, terrain_styles))
}

/// What building one room needs besides its document and layout.
struct Parts<'a> {
    definitions: &'a EnemyDefinitions,
    bosses: &'a BossDefinitions,
    /// The file the placements came from, named in an error.
    path: &'a Path,
}

/// One room: `layout` (the plan's) with the enemies and boss of `document`, at the id `id`.
fn build(
    id: RoomId,
    document: RoomEnemies,
    layout: &RoomLayout,
    parts: &Parts,
) -> Result<Room, String> {
    let named = |error: String| format!("{}: {error}", parts.path.display());
    let placement = document.boss.clone();
    let mut room = RoomDraft {
        enemies: document.enemies,
        boss: document.boss,
        layout: layout.clone(),
    }
    .build(id, parts.definitions)
    .map_err(named)?;
    place_boss(&mut room, placement.as_ref(), parts.bosses).map_err(named)?;
    Ok(room)
}
