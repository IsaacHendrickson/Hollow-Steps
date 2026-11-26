//! Loading rooms for the preview: the only part of it that reads files. A room is loaded by the
//! code the game uses (`load_rooms` for the three built-in rooms, `RoomDraft::build` and
//! `place_boss` for a saved one, the body of `load_extra_rooms`), so the picture is what the game
//! would load, and a room the game refuses is refused here with the game's message.
use std::path::{Path, PathBuf};

use super::Scene;
use crate::boss_definition::BossDefinitions;
use crate::editor::RoomDraft;
use crate::editor_files::{load_extra_rooms, read_document, room_filename};
use crate::enemy::EnemyDefinitions;
use crate::room::{load_rooms, place_boss, EnemyPlacement, Room, RoomId};

/// A room as the game loaded it, with the enemy placements of its file (the patrol endpoints
/// the game does not keep on its enemies).
#[derive(Debug)]
pub struct Loaded {
    pub id: RoomId,
    pub room: Room,
    pub placements: Vec<EnemyPlacement>,
}

impl Loaded {
    pub fn scene(&self) -> Scene {
        Scene::from_room(&self.room, &self.placements)
    }
}

/// A room id from text: `first`, `second`, `boss`, `room_3` or just `3` (a saved room), or `hub`
/// (the hub room of `run.ron` in `assets`).
pub fn parse_id(assets: &Path, text: &str) -> Result<RoomId, String> {
    let lower = text.trim().to_lowercase();
    match lower.as_str() {
        "first" => return Ok(RoomId::First),
        "second" => return Ok(RoomId::Second),
        "boss" => return Ok(RoomId::Boss),
        "hub" => {
            let config = crate::run::RunConfig::load(assets)?;
            return config
                .hub_room
                .ok_or_else(|| "run.ron names no hub room".to_owned());
        }
        _ => {}
    }
    let number = lower.strip_prefix("room_").unwrap_or(&lower);
    match number.parse::<usize>() {
        Ok(id) if id.to_string() == number => Ok(RoomId::Custom(id)),
        _ => Err(format!(
            "{text:?} is not a room: use first, second, boss, hub, a number or room_<number>, or the path of a room file"
        )),
    }
}

/// The room id a room file's name stands for and the assets folder it lies in
/// (`<assets>/rooms/<file>`).
pub fn id_of_path(path: &Path) -> Result<(RoomId, PathBuf), String> {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    let is_ron = path.extension().is_some_and(|extension| extension == "ron");
    let assets = path.parent().and_then(Path::parent);
    match (is_ron, assets) {
        (true, Some(assets)) => {
            let id = parse_id(assets, stem)
                .ok()
                .filter(|id| room_filename(*id) == format!("{stem}.ron"));
            id.map(|id| (id, assets.to_owned())).ok_or_else(|| {
                format!(
                    "{}: not a room file name (first, second, boss or room_<number>.ron)",
                    path.display()
                )
            })
        }
        _ => Err(format!(
            "{}: a room file lies in <assets>/rooms/ and ends in .ron",
            path.display()
        )),
    }
}

fn placements_of(assets: &Path, id: RoomId) -> Result<Vec<EnemyPlacement>, String> {
    Ok(read_document(&assets.join("rooms").join(room_filename(id)))?.enemies)
}

/// One room of `assets`, loaded as the game loads it.
pub fn load_one(assets: &Path, id: RoomId) -> Result<Loaded, String> {
    let room = match id {
        RoomId::First | RoomId::Second | RoomId::Boss => {
            let [first, second, boss] = load_rooms(assets)?;
            match id {
                RoomId::First => first,
                RoomId::Second => second,
                _ => boss,
            }
        }
        RoomId::Custom(_) => {
            let path = assets.join("rooms").join(room_filename(id));
            let document = read_document(&path)?;
            let layout = document
                .layout
                .ok_or_else(|| format!("{}: new room needs layout", path.display()))?;
            let placement = document.boss.clone();
            let definitions = EnemyDefinitions::load(assets)?;
            let mut room = RoomDraft {
                enemies: document.enemies,
                boss: document.boss,
                layout,
            }
            .build(id, &definitions)
            .map_err(|error| format!("{}: {error}", path.display()))?;
            let bosses = BossDefinitions::load(assets)?;
            place_boss(&mut room, placement.as_ref(), &bosses)
                .map_err(|error| format!("{}: {error}", path.display()))?;
            room
        }
    };
    Ok(Loaded {
        id,
        placements: placements_of(assets, id)?,
        room,
    })
}

/// Every room of `assets` (the three built-in rooms, then the saved ones in id order).
pub fn load_all(assets: &Path) -> Result<Vec<Loaded>, String> {
    let definitions = EnemyDefinitions::load(assets)?;
    let mut rooms: Vec<Room> = load_rooms(assets)?.into();
    rooms.extend(load_extra_rooms(assets, &definitions)?);
    rooms
        .into_iter()
        .map(|room| {
            Ok(Loaded {
                id: room.id,
                placements: placements_of(assets, room.id)?,
                room,
            })
        })
        .collect()
}

/// The width of the widest room, in px: what the default cell is chosen for.
pub fn widest(rooms: &[Loaded]) -> f32 {
    rooms
        .iter()
        .map(|loaded| loaded.room.bounds.w)
        .fold(0.0, f32::max)
}
