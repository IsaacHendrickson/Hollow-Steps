//! The room editor's open session: the draft being edited, the definitions it is checked
//! against, the catalog of other rooms and the terrain look presets. The draft and its edits
//! live in `crate::editor`; this is what ties them to the files on disk and to the unsaved flag.
use std::path::{Path, PathBuf};

use macroquad::math::Vec2;

use crate::behavior::read_ron;
use crate::editor::{room_filename, Placement, RoomDraft, RoomLayout, Selection};
use crate::enemy::EnemyDefinitions;
use crate::player_editor::PlayerDraft;
use crate::room::{Room, RoomEnemies, RoomId};
use crate::terrain_styles::TerrainStyles;

#[derive(Debug)]
pub struct EditorSession {
    pub id: RoomId,
    pub path: PathBuf,
    pub draft: RoomDraft,
    pub definitions: EnemyDefinitions,
    pub content: crate::definition_editor::DefinitionDraft,
    pub rooms: Vec<(RoomId, RoomLayout)>,
    /// The terrain look presets the room's look is chosen from (`editor_look`).
    pub styles: TerrainStyles,
    /// The Player panel's working copy of the player tunables; `GameState` gives it the game's
    /// draft when the session opens and takes its values into effect each frame.
    pub player: PlayerDraft,
    pub new: bool,
    pub dirty: bool,
    pub message: String,
}

impl EditorSession {
    pub fn place(&mut self, placement: Placement, position: Vec2) -> Result<Selection, String> {
        match placement {
            Placement::Boss(name) => {
                self.draft
                    .place_boss(&name, position, &self.content.boss_definitions()?)
            }
            other => self.draft.place(other, position, &self.definitions),
        }
    }

    pub fn move_selection(&mut self, selection: Selection, position: Vec2) -> Result<(), String> {
        if selection == Selection::Boss {
            self.draft
                .move_boss(position, &self.content.boss_definitions()?)
        } else {
            self.draft.move_to(selection, position, &self.definitions)
        }
    }

    pub fn update_boss(
        &mut self,
        path: &Path,
        boss: crate::boss_definition::BossDefinition,
    ) -> Result<(), String> {
        let previous = self.content.bosses.get(path).ok_or("missing boss")?;
        if previous == &boss {
            return Ok(());
        }
        let previous_name = previous.name.clone();
        let mut next = self.content.clone();
        next.bosses.insert(path.to_owned(), boss);
        let definitions = next.boss_definitions()?;
        let mut draft = self.draft.clone();
        let mut document = RoomEnemies {
            enemies: Vec::new(),
            boss: draft.boss.clone(),
            layout: None,
        };
        // Also handle repeated unsaved renames of the current placement.
        if let Some(p) = &mut document.boss {
            if p.definition == previous_name {
                p.definition = next.bosses[path].name.clone();
            }
        }
        next.rename_placements(&mut document);
        draft.boss = document.boss;
        draft.validate_boss(&definitions)?;
        self.content = next;
        self.draft = draft;
        self.dirty = true;
        Ok(())
    }

    pub fn open(assets: &Path, room: &Room, rooms: &[Room]) -> Result<Self, String> {
        let path = assets.join("rooms").join(room_filename(room.id));
        let definitions = EnemyDefinitions::load(assets)?;
        let document = read_ron(&path)?;
        let authored = crate::room::load_rooms(assets)?;
        let fallback = authored.iter().find(|r| r.id == room.id).unwrap_or(room);
        // Read authored layouts rather than session pickups, moved enemies, or reload snapshots.
        let catalog = rooms
            .iter()
            .map(|room| {
                let document: RoomEnemies =
                    read_ron(&assets.join("rooms").join(room_filename(room.id)))?;
                Ok((
                    room.id,
                    document.layout.unwrap_or_else(|| {
                        RoomLayout::from_room(
                            authored.iter().find(|r| r.id == room.id).unwrap_or(room),
                        )
                    }),
                ))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(Self {
            id: room.id,
            path,
            draft: RoomDraft::from_document(document, fallback),
            definitions,
            content: crate::definition_editor::DefinitionDraft::load(assets)?,
            rooms: catalog,
            styles: TerrainStyles::load(assets)?,
            player: PlayerDraft::default(),
            new: false,
            dirty: false,
            message: String::new(),
        })
    }

    pub fn catalog(&self) -> Vec<(RoomId, RoomLayout)> {
        let mut rooms = self.rooms.clone();
        rooms.retain(|(id, _)| *id != self.id);
        rooms.push((self.id, self.draft.layout.clone()));
        rooms
    }

    /// Keep preview definitions and unsaved placements in step with valid entity edits.
    pub fn update_entity(
        &mut self,
        path: &Path,
        entity: crate::enemy::EnemyDefinition,
    ) -> Result<(), String> {
        let old = self.content.entities.get(path).ok_or("missing entity")?;
        if old == &entity {
            return Ok(());
        }
        let old = self
            .content
            .entities
            .insert(path.to_owned(), entity.clone())
            .expect("entity");
        match self.content.definitions() {
            Ok(definitions) => {
                for placement in &mut self.draft.enemies {
                    if placement.definition == old.name {
                        placement.definition = entity.name.clone();
                    }
                }
                self.definitions = definitions;
                self.dirty = true;
                Ok(())
            }
            Err(error) => {
                self.content.entities.insert(path.to_owned(), old);
                Err(error)
            }
        }
    }

    /// Include unsaved placements when checking definition references.
    pub fn delete_entity(&mut self, path: &Path) -> Result<(), String> {
        let entity = self.content.entities.get(path).ok_or("missing entity")?;
        for placement in &self.draft.enemies {
            if placement.definition == entity.name
                || self.content.entity_path(&placement.definition).as_deref() == Some(path)
            {
                return Err(format!(
                    "{}: entity is referenced by the current room draft",
                    self.path.display()
                ));
            }
        }
        self.content.delete_entity(path)
    }

    pub fn new_room(&mut self) {
        let mut id = 0;
        while self
            .rooms
            .iter()
            .any(|(room, _)| *room == RoomId::Custom(id))
            || self
                .path
                .with_file_name(room_filename(RoomId::Custom(id)))
                .exists()
        {
            id += 1;
        }
        self.id = RoomId::Custom(id);
        self.path.set_file_name(room_filename(self.id));
        self.draft = RoomDraft::new_room(&self.draft.layout);
        self.new = true;
        self.dirty = true;
        self.message = "New room draft; Save to create it".into();
    }
}
