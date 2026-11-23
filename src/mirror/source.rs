//! The room documents of an assets folder with the dungeon flipped: what `world_load` hands to the
//! loaders (`room::load_rooms_from`, `editor_files::load_extra_rooms_from`) so the rooms they
//! build are the flipped ones. Nothing is written and no room code is repeated; this only decides
//! which documents the loaders see.
//!
//! Every document the loaders would read is read here first (the three fixed rooms and every
//! `room_<n>.ron`), `flip_world` runs over them all at once (it needs the whole dungeon to make
//! the arrivals agree) and each loader then asks for its file by path. A file this pass could not
//! read or name is not in the set, and the loader reads it itself, so the error is the one the
//! plain loaders give, with the same words.
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::flip_world;
use crate::behavior::ron_files;
use crate::editor_files::{read_document, room_filename};
use crate::room::{RoomEnemies, RoomId};

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
        let rooms = assets.join("rooms");
        let mut found: Vec<(RoomId, PathBuf, RoomEnemies)> = Vec::new();
        for id in room_ids(&rooms) {
            let path = rooms.join(room_filename(id));
            if let Ok(document) = read_document(&path) {
                found.push((id, path, document));
            }
        }
        let mut documents: Vec<(RoomId, RoomEnemies)> = found
            .iter()
            .map(|(id, _, document)| (*id, document.clone()))
            .collect();
        flip_world(&mut documents, |id| Some(id) != hub);
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
fn room_ids(rooms: &Path) -> Vec<RoomId> {
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
