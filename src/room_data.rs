//! Synchronous, window-free room parsing, validation, and construction.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

use macroquad::math::{Rect, Vec2};
use serde::Deserialize;

use crate::boss_special::{BossSpecials, SpecialAttack, SUMMON_INSET};
use crate::coin::{Coin, COIN_RADIUS};
use crate::enemy::{Enemy, ENEMY_SIZE};
use crate::game::SPECIAL_BOSS_SIZE;
use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::player::PlayerConfig;
use crate::room::{Exit, Room, RoomEdge, RoomId};

pub const DEFAULT_ROOMS_DIR: &str = "assets/rooms";
pub type Rooms = BTreeMap<RoomId, Room>;

#[derive(Debug)]
pub struct RoomLoadError(String);

impl RoomLoadError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for RoomLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for RoomLoadError {}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RoomFile {
    id: String,
    origin: [f32; 2],
    size: [f32; 2],
    floor_y: f32,
    default_spawn: String,
    spawns: Vec<SpawnFile>,
    exits: Vec<ExitFile>,
    entities: Vec<EntityFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpawnFile {
    name: String,
    position: [f32; 2],
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExitFile {
    edge: RoomEdge,
    offset: f32,
    length: f32,
    target_room: String,
    target_spawn: String,
}

impl ExitFile {
    fn resolve(&self, spawn: Vec2) -> Exit {
        Exit {
            edge: self.edge,
            offset: self.offset,
            length: self.length,
            target_room: self.target_room.clone().into(),
            spawn,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum EntityFile {
    Enemy {
        position: [f32; 2],
        patrol: [f32; 2],
        speed: f32,
        detection_radius: f32,
    },
    Item {
        kind: Item,
        position: [f32; 2],
    },
    Coin {
        position: [f32; 2],
    },
    ArenaBoss {
        position: [f32; 2],
    },
}

/// Load every `.ron` file directly inside a directory. No graphics APIs are used.
pub fn load_rooms(directory: impl AsRef<Path>) -> Result<Rooms, RoomLoadError> {
    let directory = directory.as_ref();
    let io_error = |error| RoomLoadError::new(format!("{}: {error}", directory.display()));
    let mut paths = fs::read_dir(directory)
        .map_err(io_error)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(io_error)?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "ron"));
    paths.sort();
    let mut sources = Vec::new();
    for path in paths {
        let source = fs::read_to_string(&path)
            .map_err(|error| RoomLoadError::new(format!("{}: {error}", path.display())))?;
        sources.push((path.display().to_string(), source));
    }
    if sources.is_empty() {
        return Err(RoomLoadError::new(format!(
            "{}: no .ron room files found",
            directory.display()
        )));
    }
    parse_rooms(
        sources
            .iter()
            .map(|(path, source)| (path.as_str(), source.as_str())),
    )
}

/// Parse labeled RON sources and validate the entire graph before constructing rooms.
/// Labels (usually filenames) are included in errors; input order has no meaning.
pub fn parse_rooms<'a>(
    sources: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<Rooms, RoomLoadError> {
    let mut files = BTreeMap::new();
    for (label, source) in sources {
        let file: RoomFile = ron::from_str(source)
            .map_err(|error| RoomLoadError::new(format!("{label}: {error}")))?;
        let context =
            |message| RoomLoadError::new(format!("{label}: room '{}': {message}", file.id));
        file.validate().map_err(context)?;
        if let Some((previous, _)) = files.get(&file.id) {
            return Err(context(format!("duplicate room id (also in {previous})")));
        }
        files.insert(file.id.clone(), (label.to_owned(), file));
    }
    if files.is_empty() {
        return Err(RoomLoadError::new("no room files supplied"));
    }

    let mut rooms = BTreeMap::new();
    for (label, file) in files.values() {
        let context =
            |message| RoomLoadError::new(format!("{label}: room '{}': {message}", file.id));
        let mut exits = Vec::new();
        for (index, exit) in file.exits.iter().enumerate() {
            let (_, target) = files.get(&exit.target_room).ok_or_else(|| {
                context(format!(
                    "exit {index}: missing target room '{}'",
                    exit.target_room
                ))
            })?;
            let spawn = target.spawn(&exit.target_spawn).ok_or_else(|| {
                context(format!(
                    "exit {index}: target room '{}' has no spawn point '{}'",
                    exit.target_room, exit.target_spawn
                ))
            })?;
            exits.push(exit.resolve(spawn));
        }
        let id: RoomId = file.id.clone().into();
        let mut room = Room {
            id: id.clone(),
            bounds: file.bounds(),
            floor_y: file.floor_y,
            // The local validation already checked this reference.
            spawn: file
                .spawn(&file.default_spawn)
                .expect("validated default spawn"),
            exits,
            enemies: Vec::new(),
            items: Vec::new(),
            coins: Vec::new(),
            boss: None,
            boss_special: None,
            special_attack_timer: 0.0,
            next_special_attack: SpecialAttack::Slam,
            last_special_hit: None,
            spawned_enemy_count: 0,
            spawned_coin_count: 0,
        };
        for entity in &file.entities {
            match entity {
                EntityFile::Enemy {
                    position,
                    patrol,
                    speed,
                    detection_radius,
                } => {
                    room.enemies.push(Enemy::placed(
                        Vec2::from(*position),
                        *patrol,
                        *speed,
                        *detection_radius,
                        room.world_bounds(),
                    ));
                }
                EntityFile::Item { kind, position } => room.items.push(WorldItem {
                    item: *kind,
                    position: Vec2::from(*position),
                }),
                EntityFile::Coin { position } => room.coins.push(Coin {
                    position: Vec2::from(*position),
                }),
                EntityFile::ArenaBoss { position } => {
                    room.boss_special = Some(BossSpecials::new(Vec2::from(*position)));
                }
            }
        }
        rooms.insert(id, room);
    }
    Ok(rooms)
}

impl RoomFile {
    fn bounds(&self) -> Rect {
        Rect::new(self.origin[0], self.origin[1], self.size[0], self.size[1])
    }

    fn spawn(&self, name: &str) -> Option<Vec2> {
        self.spawns
            .iter()
            .find(|spawn| spawn.name == name)
            .map(|spawn| Vec2::from(spawn.position))
    }

    fn fits(&self, position: [f32; 2], size: Vec2) -> bool {
        let [x, y] = position;
        let bounds = self.bounds();
        x.is_finite()
            && y.is_finite()
            && x - size.x / 2.0 >= bounds.x
            && x + size.x / 2.0 <= bounds.right()
            && y - size.y / 2.0 >= bounds.y
            && y + size.y / 2.0 <= self.floor_y
    }

    fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("id must not be empty".into());
        }
        let bounds = self.bounds();
        let player_size = PlayerConfig::default().size;
        if !self
            .origin
            .iter()
            .chain(&self.size)
            .all(|value| value.is_finite())
            || self.size[0] < player_size.x
            || self.size[1] < player_size.y
            || !bounds.right().is_finite()
            || !bounds.bottom().is_finite()
        {
            return Err(
                "origin and size must be finite; size must fit the player (32 x 48)".into(),
            );
        }
        if !self.floor_y.is_finite()
            || self.floor_y < bounds.y + player_size.y
            || self.floor_y > bounds.bottom()
        {
            return Err(
                "floor_y must be finite, inside the room, and leave space for the player".into(),
            );
        }
        for (index, exit) in self.exits.iter().enumerate() {
            let edge_length = match exit.edge {
                RoomEdge::Left | RoomEdge::Right => bounds.h,
                RoomEdge::Top | RoomEdge::Bottom => bounds.w,
            };
            if !exit.offset.is_finite()
                || !exit.length.is_finite()
                || exit.offset < 0.0
                || exit.length <= 0.0
                || exit.offset + exit.length > edge_length
            {
                return Err(format!(
                    "exit {index}: offset and positive length must fit its edge"
                ));
            }
        }
        let mut names = BTreeSet::new();
        for spawn in &self.spawns {
            if spawn.name.trim().is_empty() || !names.insert(&spawn.name) {
                return Err(format!(
                    "spawn point '{}': name is empty or duplicated",
                    spawn.name
                ));
            }
            if !self.fits(spawn.position, player_size) {
                return Err(format!(
                    "spawn point '{}': player body must fit above the floor inside the room",
                    spawn.name
                ));
            }
            let position = Vec2::from(spawn.position);
            let body = Rect::new(
                position.x - player_size.x / 2.0,
                position.y - player_size.y / 2.0,
                player_size.x,
                player_size.y,
            );
            if self
                .exits
                .iter()
                .any(|exit| exit.resolve(position).zone(bounds).overlaps(&body))
            {
                return Err(format!(
                    "spawn point '{}': overlaps an exit and would immediately transition",
                    spawn.name
                ));
            }
        }
        if self.spawn(&self.default_spawn).is_none() {
            return Err(format!(
                "default_spawn '{}' has no matching spawn point",
                self.default_spawn
            ));
        }
        let mut bosses = 0;
        for (index, entity) in self.entities.iter().enumerate() {
            let (position, size) = match entity {
                EntityFile::Enemy {
                    position,
                    patrol,
                    speed,
                    detection_radius,
                } => {
                    if !patrol.iter().all(|value| value.is_finite())
                        || patrol[0] > patrol[1]
                        || patrol[0] < bounds.x + ENEMY_SIZE.x / 2.0
                        || patrol[1] > bounds.right() - ENEMY_SIZE.x / 2.0
                        || !speed.is_finite()
                        || *speed < 0.0
                        || !detection_radius.is_finite()
                        || *detection_radius < 0.0
                        || position[0] < patrol[0]
                        || position[0] > patrol[1]
                        || position[1] != self.floor_y - ENEMY_SIZE.y / 2.0
                    {
                        return Err(format!("entity {index} (Enemy): invalid patrol, speed, detection radius, or floor-bound position"));
                    }
                    (*position, ENEMY_SIZE)
                }
                EntityFile::Item { position, .. } => (*position, ITEM_SIZE),
                EntityFile::Coin { position } => (*position, Vec2::splat(COIN_RADIUS * 2.0)),
                EntityFile::ArenaBoss { position } => {
                    if bounds.w < SUMMON_INSET + ENEMY_SIZE.x / 2.0 {
                        return Err(format!("entity {index} (ArenaBoss): room is too narrow for summoned enemies (minimum width {})", SUMMON_INSET + ENEMY_SIZE.x / 2.0));
                    }
                    bosses += 1;
                    if bosses > 1 {
                        return Err(format!(
                            "entity {index}: only one ArenaBoss is allowed per room"
                        ));
                    }
                    (*position, SPECIAL_BOSS_SIZE)
                }
            };
            if !self.fits(position, size) {
                return Err(format!("entity {index}: position must be finite and its body must fit above the floor inside the room"));
            }
        }
        Ok(())
    }
}
