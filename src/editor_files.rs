//! Room files: their names, the staged save and loading the rooms beyond the three built in.
//! Split from `editor` (which re-exports these) to keep that file small.
use std::path::Path;

use crate::behavior::{read_ron, ron_files};
use crate::editor::{RoomDraft, RoomLayout};
use crate::enemy::EnemyDefinitions;
use crate::room::{Room, RoomEnemies, RoomId};

pub fn room_filename(id: RoomId) -> String {
    match id {
        RoomId::First => "first.ron".into(),
        RoomId::Second => "second.ron".into(),
        RoomId::Boss => "boss.ron".into(),
        RoomId::Custom(id) => format!("room_{id}.ron"),
    }
}

/// Compatibility save boundary, using the same staged writer as definition saves.
pub fn save_room(
    path: &Path,
    draft: &RoomDraft,
    definitions: &EnemyDefinitions,
    rooms: &[(RoomId, RoomLayout)],
    new: bool,
) -> Result<(), String> {
    let source = draft.serialize(definitions, rooms)?;
    if new && path.exists() {
        return Err("new room already exists".into());
    }
    if path.with_extension("ron.tmp").exists() {
        return Err("room staging file already exists".into());
    }
    let assets = path
        .parent()
        .and_then(Path::parent)
        .ok_or("invalid room path")?;
    let content = crate::definition_editor::DefinitionDraft::load(assets)?;
    let document = ron::from_str(&source).map_err(|e| e.to_string())?;
    crate::definition_editor::write_files(&content.plan(&[(path.to_owned(), document)])?)
}

/// The error for a `room_*.ron` file whose number part is not a plain number without leading
/// zeros. The rule is deliberate (the editor always writes `room_<id>.ron`), so the message
/// says what to do about the file instead of relaxing it.
fn bad_room_file_name(path: &Path) -> String {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    format!(
        "assets/rooms/{file} is not a valid room file name: room files are named \
         room_<number>.ron with no leading zeros; rename or move it"
    )
}

pub fn load_extra_rooms(
    assets: &Path,
    definitions: &EnemyDefinitions,
) -> Result<Vec<Room>, String> {
    let mut rooms = Vec::new();
    let bosses = crate::boss_definition::BossDefinitions::load(assets)?;
    for path in ron_files(&assets.join("rooms"))? {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if let Some(number) = stem.strip_prefix("room_") {
            let id = number
                .parse::<usize>()
                .map_err(|_| bad_room_file_name(&path))?;
            if stem != format!("room_{id}") {
                return Err(bad_room_file_name(&path));
            }
            let document: RoomEnemies = read_ron(&path)?;
            let layout = document
                .layout
                .ok_or_else(|| format!("{}: new room needs layout", path.display()))?;
            let placement = document.boss.clone();
            let mut room = RoomDraft {
                enemies: document.enemies,
                boss: document.boss,
                layout,
            }
            .build(RoomId::Custom(id), definitions)?;
            crate::room::place_boss(&mut room, placement.as_ref(), &bosses)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            rooms.push(room);
        }
    }
    Ok(rooms)
}
