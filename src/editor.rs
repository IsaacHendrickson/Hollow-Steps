//! Pure authored-room editing. Only the explicitly named load/save functions do I/O.
use std::path::{Path, PathBuf};

use macroquad::math::{Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::behavior::{read_ron, ron_files};
use crate::enemy::EnemyDefinitions;
use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::player::WorldBounds;
use crate::room::{EnemyPlacement, Exit, Room, RoomEdge, RoomEnemies, RoomId};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemPlacement {
    pub item: Item,
    pub position: (f32, f32),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExitPlacement {
    pub edge: RoomEdge,
    pub offset: f32,
    pub length: f32,
    pub target_room: RoomId,
    pub spawn: (f32, f32),
}

impl ExitPlacement {
    pub fn exit(&self) -> Exit {
        Exit {
            edge: self.edge,
            offset: self.offset,
            length: self.length,
            target_room: self.target_room,
            spawn: self.spawn.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoomLayout {
    pub bounds: (f32, f32, f32, f32),
    pub floor_y: f32,
    pub spawn: (f32, f32),
    pub items: Vec<ItemPlacement>,
    pub exits: Vec<ExitPlacement>,
}

impl RoomLayout {
    pub fn from_room(room: &Room) -> Self {
        Self {
            bounds: (room.bounds.x, room.bounds.y, room.bounds.w, room.bounds.h),
            floor_y: room.floor_y,
            spawn: room.spawn.into(),
            items: room
                .items
                .iter()
                .map(|item| ItemPlacement {
                    item: item.item,
                    position: item.position.into(),
                })
                .collect(),
            exits: room
                .exits
                .iter()
                .map(|exit| ExitPlacement {
                    edge: exit.edge,
                    offset: exit.offset,
                    length: exit.length,
                    target_room: exit.target_room,
                    spawn: exit.spawn.into(),
                })
                .collect(),
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.bounds.0, self.bounds.1, self.bounds.2, self.bounds.3)
    }

    pub fn world_bounds(&self) -> WorldBounds {
        WorldBounds::new(self.bounds.0, self.bounds.0 + self.bounds.2, self.floor_y)
    }

    fn contains_body(&self, center: (f32, f32), size: Vec2) -> bool {
        let p = Vec2::from(center);
        let r = self.rect();
        p.is_finite()
            && p.x - size.x / 2.0 >= r.x
            && p.x + size.x / 2.0 <= r.right()
            && p.y - size.y / 2.0 >= r.y
            && p.y + size.y / 2.0 <= self.floor_y
    }

    pub fn validate_geometry(&self) -> Result<(), String> {
        let (x, y, w, h) = self.bounds;
        if ![x, y, w, h, x + w, y + h, self.floor_y]
            .iter()
            .all(|n| n.is_finite())
            || w < 32.0
            || h < 48.0
            || self.floor_y > y + h
            || self.floor_y < y + 48.0
        {
            return Err("invalid room bounds or floor".into());
        }
        if !self.contains_body(self.spawn, Vec2::new(32.0, 48.0)) {
            return Err("spawn must keep the player's body inside the room above the floor".into());
        }
        for item in &self.items {
            if !self.contains_body(item.position, ITEM_SIZE) {
                return Err("item must be inside the room above the floor".into());
            }
        }
        for exit in &self.exits {
            let span = match exit.edge {
                RoomEdge::Left | RoomEdge::Right => h,
                _ => w,
            };
            if !exit.offset.is_finite()
                || !exit.length.is_finite()
                || exit.offset < 0.0
                || exit.length <= 0.0
                || exit.offset + exit.length > span
                || !Vec2::from(exit.spawn).is_finite()
            {
                return Err("exit span or destination spawn is invalid".into());
            }
        }
        Ok(())
    }

    pub fn apply(&self, room: &mut Room) -> Result<(), String> {
        self.validate_geometry()?;
        room.bounds = self.rect();
        room.floor_y = self.floor_y;
        room.spawn = self.spawn.into();
        room.items = self
            .items
            .iter()
            .map(|p| WorldItem {
                item: p.item,
                position: p.position.into(),
            })
            .collect();
        room.exits = self.exits.iter().map(ExitPlacement::exit).collect();
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Enemy(usize),
    Item(usize),
    Spawn,
    Exit(usize),
    Destination(usize),
}

#[derive(Clone, Debug)]
pub enum Placement {
    Enemy(String),
    Item(Item),
}

#[derive(Clone, Debug)]
pub struct RoomDraft {
    pub enemies: Vec<EnemyPlacement>,
    pub layout: RoomLayout,
}

impl RoomDraft {
    pub fn new_room(template: &RoomLayout) -> Self {
        let mut layout = template.clone();
        layout.items.clear();
        layout.exits.clear();
        Self {
            enemies: Vec::new(),
            layout,
        }
    }

    pub fn from_document(document: RoomEnemies, fallback: &Room) -> Self {
        Self {
            enemies: document.enemies,
            layout: document
                .layout
                .unwrap_or_else(|| RoomLayout::from_room(fallback)),
        }
    }

    pub fn validate_local(&self, definitions: &EnemyDefinitions) -> Result<(), String> {
        self.layout.validate_geometry()?;
        for enemy in &self.enemies {
            let size = definitions.size(&enemy.definition)?;
            if size.y > self.layout.floor_y - self.layout.bounds.1 {
                return Err("enemy is taller than the room".into());
            }
            definitions.spawn(
                &enemy.definition,
                enemy.patrol_left,
                enemy.patrol_right,
                self.layout.world_bounds(),
            )?;
        }
        Ok(())
    }

    pub fn validate(
        &self,
        definitions: &EnemyDefinitions,
        rooms: &[(RoomId, RoomLayout)],
    ) -> Result<(), String> {
        self.validate_local(definitions)?;
        for exit in &self.layout.exits {
            let target = rooms
                .iter()
                .find(|(id, _)| *id == exit.target_room)
                .ok_or_else(|| format!("exit targets missing room {:?}", exit.target_room))?;
            if !target.1.contains_body(exit.spawn, Vec2::new(32.0, 48.0)) {
                return Err(format!(
                    "exit spawn outside destination {:?}",
                    exit.target_room
                ));
            }
            if target.1.exits.iter().any(|e| {
                e.exit().zone(target.1.rect()).overlaps(&Rect::new(
                    exit.spawn.0 - 16.0,
                    exit.spawn.1 - 24.0,
                    32.0,
                    48.0,
                ))
            }) {
                return Err("exit destination overlaps an exit (immediate return trip)".into());
            }
        }
        Ok(())
    }

    pub fn serialize(
        &self,
        definitions: &EnemyDefinitions,
        rooms: &[(RoomId, RoomLayout)],
    ) -> Result<String, String> {
        self.validate(definitions, rooms)?;
        ron::ser::to_string_pretty(
            &RoomEnemies {
                enemies: self.enemies.clone(),
                layout: Some(self.layout.clone()),
            },
            ron::ser::PrettyConfig::default(),
        )
        .map_err(|e| e.to_string())
    }

    pub fn place(
        &mut self,
        placement: Placement,
        position: Vec2,
        definitions: &EnemyDefinitions,
    ) -> Result<Selection, String> {
        if !position.is_finite() {
            return Err("position must be finite".into());
        }
        let mut next = self.clone();
        let selection = match placement {
            Placement::Enemy(definition) => {
                let index = next.enemies.len();
                next.enemies.push(EnemyPlacement {
                    definition,
                    patrol_left: position.x,
                    patrol_right: position.x,
                });
                Selection::Enemy(index)
            }
            Placement::Item(item) => {
                let index = next.layout.items.len();
                next.layout.items.push(ItemPlacement {
                    item,
                    position: position.into(),
                });
                Selection::Item(index)
            }
        };
        next.validate_local(definitions)?;
        *self = next;
        Ok(selection)
    }

    pub fn move_to(
        &mut self,
        selection: Selection,
        position: Vec2,
        definitions: &EnemyDefinitions,
    ) -> Result<(), String> {
        if !position.is_finite() {
            return Err("position must be finite".into());
        }
        let mut next = self.clone();
        match selection {
            Selection::Enemy(i) => {
                let enemy = next.enemies.get_mut(i).ok_or("missing enemy")?;
                let delta = position.x - enemy.patrol_left;
                enemy.patrol_left += delta;
                enemy.patrol_right += delta;
            }
            Selection::Item(i) => {
                next.layout.items.get_mut(i).ok_or("missing item")?.position = position.into()
            }
            Selection::Spawn => next.layout.spawn = position.into(),
            Selection::Exit(i) => {
                let exit = next.layout.exits.get_mut(i).ok_or("missing exit")?;
                exit.offset = match exit.edge {
                    RoomEdge::Left | RoomEdge::Right => {
                        position.y - next.layout.bounds.1 - exit.length / 2.0
                    }
                    _ => position.x - next.layout.bounds.0 - exit.length / 2.0,
                };
            }
            Selection::Destination(i) => {
                next.layout.exits.get_mut(i).ok_or("missing exit")?.spawn = position.into()
            }
        }
        next.validate_local(definitions)?;
        *self = next;
        Ok(())
    }

    pub fn add_exit(&mut self, edge: RoomEdge, target_room: RoomId, spawn: Vec2) -> Selection {
        let length = match edge {
            RoomEdge::Left | RoomEdge::Right => self.layout.bounds.3,
            _ => self.layout.bounds.2,
        }
        .min(96.0);
        let index = self.layout.exits.len();
        self.layout.exits.push(ExitPlacement {
            edge,
            offset: 0.0,
            length,
            target_room,
            spawn: spawn.into(),
        });
        Selection::Exit(index)
    }

    pub fn set_exit_target(
        &mut self,
        index: usize,
        target_room: RoomId,
        spawn: Vec2,
    ) -> Result<(), String> {
        if !spawn.is_finite() {
            return Err("destination must be finite".into());
        }
        let exit = self.layout.exits.get_mut(index).ok_or("missing exit")?;
        exit.target_room = target_room;
        exit.spawn = spawn.into();
        Ok(())
    }

    pub fn delete(&mut self, selection: Selection) -> Result<(), String> {
        match selection {
            Selection::Enemy(i) if i < self.enemies.len() => {
                self.enemies.remove(i);
            }
            Selection::Item(i) if i < self.layout.items.len() => {
                self.layout.items.remove(i);
            }
            _ => return Err("select an enemy or item to delete".into()),
        }
        Ok(())
    }

    pub fn build(&self, id: RoomId, definitions: &EnemyDefinitions) -> Result<Room, String> {
        self.validate_local(definitions)?;
        let mut room = Room {
            id,
            bounds: self.layout.rect(),
            floor_y: self.layout.floor_y,
            spawn: self.layout.spawn.into(),
            enemies: self
                .enemies
                .iter()
                .map(|p| {
                    definitions.spawn(
                        &p.definition,
                        p.patrol_left,
                        p.patrol_right,
                        self.layout.world_bounds(),
                    )
                })
                .collect::<Result<_, _>>()?,
            exits: Vec::new(),
            items: Vec::new(),
            coins: Vec::new(),
            boss: None,
            boss_special: None,
        };
        self.layout.apply(&mut room)?;
        Ok(room)
    }
}

pub fn room_filename(id: RoomId) -> String {
    match id {
        RoomId::First => "first.ron".into(),
        RoomId::Second => "second.ron".into(),
        RoomId::Boss => "boss.ron".into(),
        RoomId::Custom(id) => format!("room_{id}.ron"),
    }
}

/// Validate/serialize before even opening a file. Rename leaves the previous file intact on failure.
pub fn save_room(
    path: &Path,
    draft: &RoomDraft,
    definitions: &EnemyDefinitions,
    rooms: &[(RoomId, RoomLayout)],
    new: bool,
) -> Result<(), String> {
    use std::io::Write;
    let source = draft.serialize(definitions, rooms)?;
    let temporary = path.with_extension("ron.tmp");
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|e| e.to_string())?;
        let write_result = (|| {
            file.write_all(source.as_bytes())
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
            drop(file);
            if new {
                // Hard-link is an atomic create-if-absent; never replace another new room.
                std::fs::hard_link(&temporary, path).map_err(|e| e.to_string())?;
                std::fs::remove_file(&temporary).map_err(|e| e.to_string())
            } else {
                std::fs::rename(&temporary, path).map_err(|e| e.to_string())
            }
        })();
        if write_result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        write_result
    })();
    result.map_err(|e| format!("{}: {e}", path.display()))
}

pub fn load_extra_rooms(
    assets: &Path,
    definitions: &EnemyDefinitions,
) -> Result<Vec<Room>, String> {
    let mut rooms = Vec::new();
    for path in ron_files(&assets.join("rooms"))? {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if let Some(number) = stem.strip_prefix("room_") {
            let id = number
                .parse::<usize>()
                .map_err(|_| format!("invalid room filename {stem}"))?;
            if stem != format!("room_{id}") {
                return Err(format!("noncanonical room filename {stem}"));
            }
            let document: RoomEnemies = read_ron(&path)?;
            let layout = document
                .layout
                .ok_or_else(|| format!("{}: new room needs layout", path.display()))?;
            rooms.push(
                RoomDraft {
                    enemies: document.enemies,
                    layout,
                }
                .build(RoomId::Custom(id), definitions)?,
            );
        }
    }
    Ok(rooms)
}

#[derive(Debug)]
pub struct EditorSession {
    pub id: RoomId,
    pub path: PathBuf,
    pub draft: RoomDraft,
    pub definitions: EnemyDefinitions,
    pub content: crate::definition_editor::DefinitionDraft,
    pub rooms: Vec<(RoomId, RoomLayout)>,
    pub new: bool,
    pub dirty: bool,
    pub message: String,
}

impl EditorSession {
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
            new: false,
            dirty: false,
            message: String::new(),
        })
    }

    pub fn save_content(&mut self) -> Result<(), String> {
        if self.new && self.path.exists() {
            return Err("new room already exists".into());
        }
        let mut document = RoomEnemies {
            enemies: self.draft.enemies.clone(),
            layout: Some(self.draft.layout.clone()),
        };
        let files = self
            .content
            .plan(&[(self.path.clone(), document.clone())])?;
        self.content.rename_placements(&mut document);
        let definitions = self.content.definitions()?;
        crate::definition_editor::write_files(&files)?;
        let assets = self
            .path
            .parent()
            .and_then(Path::parent)
            .ok_or("invalid asset path")?;
        self.content = crate::definition_editor::DefinitionDraft::load(assets)?;
        self.draft.enemies = document.enemies;
        self.definitions = definitions;
        Ok(())
    }

    pub fn catalog(&self) -> Vec<(RoomId, RoomLayout)> {
        let mut rooms = self.rooms.clone();
        rooms.retain(|(id, _)| *id != self.id);
        rooms.push((self.id, self.draft.layout.clone()));
        rooms
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
