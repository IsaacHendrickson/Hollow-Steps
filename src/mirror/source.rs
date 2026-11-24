//! The room documents of an assets folder with the dungeon, or some of its rooms, flipped: what `world_load` hands to the
//! loaders (`room::load_rooms_from`, `editor_files::load_extra_rooms_from`) so the rooms they
//! build are the flipped ones. Nothing is written and no room code is repeated; this only decides
//! which documents the loaders see.
//!
//! Every document the loaders would read is read here first (the three fixed rooms and every
//! `room_<n>.ron`), `flip_world` runs over them all at once (it needs the whole dungeon to make
//! the arrivals agree) and each loader then asks for its file by path. A file this pass could not
//! read or name is not in the set, and the loader reads it itself, so the error is the one the
//! plain loaders give, with the same words.
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::flip_world;
use crate::behavior::ron_files;
use crate::editor_files::{read_document, room_filename};
use crate::room::{RoomEnemies, RoomId};
use crate::variety::RoomFlips;

/// The flipped documents of one assets folder, by the path of the file they came from.
#[derive(Debug, Default)]
pub struct FlippedDocuments {
    documents: HashMap<PathBuf, RoomEnemies>,
}

impl FlippedDocuments {
    /// Reads every room document under `assets/rooms` and flips all of them except `hub`'s (a
    /// game with no hub passes `None` and flips every room). A document with no layout is left
    /// as it is, and `flip_world` leaves what points into it alone.
    pub fn read(assets: &Path, hub: Option<RoomId>) -> Self {
        Self::read_with(assets, hub, true, RoomFlips::Off)
    }

    /// Reads every room document under `assets/rooms` and mirrors the rooms a run mirrors, never
    /// the hub's. A room is mirrored when the whole dungeon is (`dungeon`) **or** it is marked
    /// `flippable` and `rooms` flips it, but not both: a marked room that `rooms` flips in a
    /// mirrored dungeon is mirrored back, so a marked room is mirrored in the share of runs
    /// `rooms` says whatever the dungeon does. The map cell follows the dungeon alone, since it
    /// is a place on the map and not a part of the room: a room mirrored by itself keeps its cell.
    pub fn read_with(assets: &Path, hub: Option<RoomId>, dungeon: bool, rooms: RoomFlips) -> Self {
        let folder = assets.join("rooms");
        let mut found: Vec<(RoomId, PathBuf, RoomEnemies)> = Vec::new();
        for id in room_ids(&folder) {
            let path = folder.join(room_filename(id));
            if let Ok(document) = read_document(&path) {
                found.push((id, path, document));
            }
        }
        let marked = |document: &RoomEnemies| document.layout.as_ref().is_some_and(|l| l.flippable);
        let mirrored: HashSet<RoomId> = found
            .iter()
            .filter(|(id, _, document)| {
                let by_itself = marked(document) && rooms.flips(*id);
                Some(*id) != hub && dungeon != by_itself
            })
            .map(|(id, _, _)| *id)
            .collect();
        let mut documents: Vec<(RoomId, RoomEnemies)> = found
            .iter()
            .map(|(id, _, document)| (*id, document.clone()))
            .collect();
        flip_world(&mut documents, |id| mirrored.contains(&id));
        for ((id, _, original), (_, document)) in found.iter().zip(documents.iter_mut()) {
            let (Some(from), Some(layout)) = (&original.layout, &mut document.layout) else {
                continue;
            };
            if Some(*id) != hub {
                layout.map_cell = from.map_cell.map(|(column, row)| {
                    (
                        if dungeon {
                            column.saturating_neg()
                        } else {
                            column
                        },
                        row,
                    )
                });
            }
        }
        let documents = found
            .into_iter()
            .zip(documents)
            .map(|((_, path, _), (_, document))| (path, document))
            .collect();
        Self { documents }
    }

    /// The document for the room file at `path`, flipped; or, for a file this set does not hold,
    /// the file read as it is, which fails the way the plain loaders fail.
    pub fn document(&self, path: &Path) -> Result<RoomEnemies, String> {
        match self.documents.get(path) {
            Some(document) => Ok(document.clone()),
            None => read_document(path),
        }
    }
}

/// The ids of the rooms that have files in `rooms`: the three fixed ones, then every
/// `room_<n>.ron` named the way the loader insists on (a number with no leading zeros). Other
/// names are the loader's to refuse.
pub(crate) fn room_ids(rooms: &Path) -> Vec<RoomId> {
    let mut ids = vec![RoomId::First, RoomId::Second, RoomId::Boss];
    for path in ron_files(rooms).unwrap_or_default() {
        let stem = path.file_stem().and_then(|s| s.to_str());
        let number = stem
            .and_then(|stem| stem.strip_prefix("room_"))
            .and_then(|digits| digits.parse::<usize>().ok());
        if let Some(id) = number.filter(|id| stem == Some(format!("room_{id}").as_str())) {
            ids.push(RoomId::Custom(id));
        }
    }
    ids
}
