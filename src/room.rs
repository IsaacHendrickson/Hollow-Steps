use std::path::Path;

use macroquad::math::{Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::behavior::read_ron;
use crate::boss_definition::BossDefinitions;
use crate::boss_special::BossSpecials;
use crate::coin::{Coin, COIN_RADIUS};
use crate::editor_files::ReadDocument;
use crate::enemy::{Enemy, EnemyDefinitions};
use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::player::WorldBounds;

pub const FLOOR_Y: f32 = 460.0;
pub const EXIT_DEPTH: f32 = 32.0;

/// The rectangle of a body centered at `position`.
fn body_rect(position: Vec2, size: Vec2) -> Rect {
    Rect::new(
        position.x - size.x / 2.0,
        position.y - size.y / 2.0,
        size.x,
        size.y,
    )
}

/// Size of a door placed in the editor (width, height); its size can be changed there.
pub const DOOR_SIZE: Vec2 = Vec2::new(48.0, 72.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RoomId {
    First,
    Second,
    Boss,
    Custom(usize),
}

impl RoomId {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::First => 0,
            Self::Second => 1,
            Self::Boss => 2,
            Self::Custom(id) => 3 + id,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoomEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Exit {
    pub edge: RoomEdge,
    /// Distance from the top (side exits) or left (top/bottom exits).
    pub offset: f32,
    pub length: f32,
    pub target_room: RoomId,
    /// Player-center coordinate in the destination room.
    pub spawn: Vec2,
    /// A door: a free rectangle in room coordinates, anywhere in the room, used by pressing
    /// Up in front of it. `edge`, `offset` and `length` mean nothing for a door.
    pub door: Option<Rect>,
    /// The key that opens this exit (`Some(3)` is Key 3), or `None` for a free exit. Using the
    /// key takes the lock off the live exit for the rest of the run (`crate::exit_lock`).
    pub locked_by: Option<u8>,
}

impl Exit {
    /// A door standing at `zone` (room coordinates) that leads to `target_room`.
    pub fn doorway(zone: Rect, target_room: RoomId, spawn: Vec2) -> Self {
        Self {
            edge: RoomEdge::Bottom,
            offset: 0.0,
            length: 0.0,
            target_room,
            spawn,
            door: Some(zone),
            locked_by: None,
        }
    }

    pub fn is_door(&self) -> bool {
        self.door.is_some()
    }

    pub fn zone(&self, bounds: Rect) -> Rect {
        if let Some(zone) = self.door {
            return zone;
        }
        match self.edge {
            RoomEdge::Left => Rect::new(bounds.x, bounds.y + self.offset, EXIT_DEPTH, self.length),
            RoomEdge::Right => Rect::new(
                bounds.right() - EXIT_DEPTH,
                bounds.y + self.offset,
                EXIT_DEPTH,
                self.length,
            ),
            RoomEdge::Top => Rect::new(bounds.x + self.offset, bounds.y, self.length, EXIT_DEPTH),
            RoomEdge::Bottom => Rect::new(
                bounds.x + self.offset,
                bounds.bottom() - EXIT_DEPTH,
                self.length,
                EXIT_DEPTH,
            ),
        }
    }
}

#[derive(Debug)]
pub struct Room {
    pub id: RoomId,
    /// The room's own name (`name` in its file); `None` shows the default display name.
    pub name: Option<String>,
    pub bounds: Rect,
    pub floor_y: f32,
    /// Solid rectangles that block the player (platforms and walls).
    pub terrain: crate::terrain::Terrain,
    /// Hazards that hurt and knock back the player on touch (`hazard`).
    pub hazards: Vec<crate::hazard::Hazard>,
    /// Breakable walls, switches and barriers, with what play has done to them (`puzzle`).
    pub puzzles: crate::puzzle::RoomPuzzles,
    /// Palette and switches for how the solids look (drawing only).
    pub terrain_style: crate::terrain_styles::RoomStyle,
    /// Parallax background layers, drawn first (back to front) behind everything else.
    pub background: Vec<crate::background::BackgroundLayer>,
    pub exits: Vec<Exit>,
    /// Marks of what crossing needs (`crate::gates`). Carried so a layout rebuilt from the room
    /// keeps them; the game never reads them.
    pub(crate) gates: Vec<crate::gates::Gate>,
    /// Where the dungeon map puts the room (`crate::dungeon_map`); `None` places it by its exits.
    /// Carried like `gates`; the game's rules never read it.
    pub(crate) map_cell: Option<(i32, i32)>,
    /// The dungeon-piece mark and pickup slots (`crate::piece`), carried like `gates`.
    pub(crate) piece: Option<crate::piece::PieceInfo>,
    pub(crate) slots: Vec<crate::piece::PickupSlot>,
    /// A run's seed may mirror this room (`crate::variety::room_flip`), carried like `piece`.
    pub(crate) flippable: bool,
    /// Default player-center respawn point for this room.
    pub spawn: Vec2,
    pub enemies: Vec<Enemy>,
    pub boss_special: Option<BossSpecials>,
    pub(crate) coins: Vec<Coin>,
    pub(crate) items: Vec<WorldItem>,
    /// Charms lying in the room, until picked up (`room_charms`).
    pub(crate) charms: Vec<crate::room_charms::WorldCharm>,
}

impl Room {
    pub fn world_bounds(&self) -> WorldBounds {
        WorldBounds::new(self.bounds.x, self.bounds.right(), self.floor_y).with_top(self.bounds.y)
    }

    /// The edge exit a body at `position` is walking into. Doors never trigger this way.
    pub fn exit_at(&self, position: Vec2, size: Vec2) -> Option<&Exit> {
        if self.exits_locked() {
            return None;
        }
        let body = body_rect(position, size);
        self.exits
            .iter()
            .find(|exit| !exit.is_door() && exit.zone(self.bounds).overlaps(&body))
    }

    /// The door a body at `position` stands in front of: its body reaches the door and its
    /// center is within the door's width. `None` while the exits are locked.
    pub fn door_at(&self, position: Vec2, size: Vec2) -> Option<&Exit> {
        if self.exits_locked() {
            return None;
        }
        let body = body_rect(position, size);
        self.exits.iter().find(|exit| {
            exit.door.is_some_and(|zone| {
                zone.overlaps(&body) && (zone.x..=zone.right()).contains(&position.x)
            })
        })
    }

    pub fn exits_locked(&self) -> bool {
        self.boss_special
            .as_ref()
            .is_some_and(|boss| !boss.is_defeated())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoomEnemies {
    pub enemies: Vec<EnemyPlacement>,
    #[serde(default)]
    pub boss: Option<BossPlacement>,
    #[serde(default)]
    pub layout: Option<crate::editor::RoomLayout>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyPlacement {
    pub definition: String,
    pub patrol_left: f32,
    pub patrol_right: f32,
    /// The lowest difficulty tier the enemy is in the room at; absent (0) in older room files
    /// means every tier. Only the game's world loading leaves a placement out; the editor and
    /// Save keep it.
    #[serde(
        default,
        skip_serializing_if = "is_zero",
        deserialize_with = "read_min_tier"
    )]
    pub min_tier: u32,
    /// The group (1 to 9) the enemy is in; absent means none. In a run exactly one placement of
    /// each group of a room stays (`crate::variety`). Only the game's world loading chooses.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::variety::read_group"
    )]
    pub group: Option<u8>,
    /// How likely the enemy is in the room in a run, 0 to 100 percent; absent means 100. A
    /// grouped placement must have 100 (`EnemyPlacement::check_variety`).
    #[serde(
        default = "crate::variety::full_chance",
        skip_serializing_if = "crate::variety::is_full_chance",
        deserialize_with = "crate::variety::read_chance"
    )]
    pub chance: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossPlacement {
    pub definition: String,
    pub position: (f32, f32),
    /// Like `EnemyPlacement::min_tier`: a boss left out of the room does not lock its exits.
    #[serde(
        default,
        skip_serializing_if = "is_zero",
        deserialize_with = "read_min_tier"
    )]
    pub min_tier: u32,
}

fn is_zero(tier: &u32) -> bool {
    *tier == 0
}

/// Reads a `min_tier`, refusing one above the highest tier there is (`MAX_MIN_TIER`).
fn read_min_tier<'de, D: serde::Deserializer<'de>>(reader: D) -> Result<u32, D::Error> {
    let tier = <u32 as Deserialize>::deserialize(reader)?;
    if tier > MAX_MIN_TIER {
        return Err(serde::de::Error::custom(format!(
            "min_tier {tier} is above {MAX_MIN_TIER}, the highest tier"
        )));
    }
    Ok(tier)
}

/// The highest `min_tier` a placement can have: the highest tier the rules can count.
pub const MAX_MIN_TIER: u32 = crate::difficulty::MAX_EFFECTIVE_TIER;

impl EnemyPlacement {
    /// A placement shown at every tier.
    pub fn new(definition: impl Into<String>, patrol_left: f32, patrol_right: f32) -> Self {
        Self {
            definition: definition.into(),
            patrol_left,
            patrol_right,
            min_tier: 0,
            group: None,
            chance: crate::variety::FULL_CHANCE,
        }
    }
}

impl BossPlacement {
    /// A placement shown at every tier.
    pub fn new(definition: impl Into<String>, position: (f32, f32)) -> Self {
        Self {
            definition: definition.into(),
            position,
            min_tier: 0,
        }
    }
}

pub fn place_boss(
    room: &mut Room,
    placement: Option<&BossPlacement>,
    definitions: &BossDefinitions,
) -> Result<(), String> {
    room.boss_special = placement
        .map(|placement| {
            let boss = definitions
                .spawn(&placement.definition, placement.position.into())?
                .with_min_tier(placement.min_tier);
            let p = boss.position();
            let half = boss.size() / 2.0;
            if p.x - half.x < room.bounds.x
                || p.x + half.x > room.bounds.right()
                || p.y - half.y < room.bounds.y
                || p.y + half.y > room.floor_y
            {
                return Err(format!(
                    "boss {:?}: position/body must fit within room",
                    placement.definition
                ));
            }
            Ok(boss)
        })
        .transpose()?;
    Ok(())
}

pub fn load_room_enemies(
    path: &Path,
    definitions: &EnemyDefinitions,
    bounds: WorldBounds,
) -> Result<Vec<Enemy>, String> {
    load_room_enemies_from(path, definitions, bounds, &read_ron::<RoomEnemies>)
}

/// `load_room_enemies` with the document coming from `read` (`editor_files::ReadDocument`).
pub fn load_room_enemies_from(
    path: &Path,
    definitions: &EnemyDefinitions,
    bounds: WorldBounds,
    read: ReadDocument,
) -> Result<Vec<Enemy>, String> {
    let room = read(path)?;
    for (index, placement) in room.enemies.iter().enumerate() {
        let checked = placement.check_variety(index);
        checked.map_err(|error| format!("{}: {error}", path.display()))?;
    }
    let bounds = if let Some(layout) = &room.layout {
        crate::editor::RoomDraft {
            enemies: room.enemies.clone(),
            boss: room.boss.clone(),
            layout: layout.clone(),
        }
        .validate_local(definitions)
        .map_err(|e| format!("{}: {e}", path.display()))?;
        layout.world_bounds()
    } else {
        bounds
    };
    room.enemies
        .into_iter()
        .enumerate()
        .map(|(index, spawn)| {
            definitions
                .spawn(
                    &spawn.definition,
                    spawn.patrol_left,
                    spawn.patrol_right,
                    bounds,
                )
                .map(|enemy| enemy.with_placement(&spawn))
                .map_err(|error| format!("{}: enemy {index}: {error}", path.display()))
        })
        .collect()
}

/// Fixed sample rooms for behavioral tests, independent of editable game content.
pub fn test_rooms() -> [Room; 3] {
    load_rooms(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test rooms must load")
}

/// Canonical layouts for legacy placement documents without an authored layout.
pub(crate) fn default_rooms() -> [Room; 3] {
    [
        Room {
            id: RoomId::First,
            name: None,
            boss_special: None,
            coins: [360.0, 410.0, 460.0]
                .map(|x| Coin {
                    position: Vec2::new(x, FLOOR_Y - COIN_RADIUS),
                })
                .to_vec(),
            spawn: Vec2::new(180.0, FLOOR_Y - 24.0),
            enemies: Vec::new(),
            bounds: Rect::new(0.0, 0.0, 960.0, FLOOR_Y),
            floor_y: FLOOR_Y,
            terrain: Default::default(),
            hazards: Vec::new(),
            puzzles: Default::default(),
            terrain_style: Default::default(),
            background: Vec::new(),
            gates: Vec::new(),
            map_cell: None,
            piece: None,
            slots: Vec::new(),
            flippable: false,
            exits: vec![Exit {
                edge: RoomEdge::Right,
                offset: FLOOR_Y - 96.0,
                length: 96.0,
                target_room: RoomId::Second,
                spawn: Vec2::new(160.0, 376.0),
                door: None,
                locked_by: None,
            }],
            items: vec![
                WorldItem {
                    item: Item::AmberShard,
                    position: Vec2::new(300.0, FLOOR_Y - ITEM_SIZE.y / 2.0),
                },
                WorldItem {
                    item: Item::Dash,
                    position: Vec2::new(500.0, FLOOR_Y - ITEM_SIZE.y / 2.0),
                },
            ],
            charms: vec![],
        },
        Room {
            id: RoomId::Second,
            name: None,
            boss_special: None,
            coins: [300.0, 350.0, 400.0]
                .map(|x| Coin {
                    position: Vec2::new(x, 400.0 - COIN_RADIUS),
                })
                .to_vec(),
            spawn: Vec2::new(160.0, 376.0),
            enemies: Vec::new(),
            bounds: Rect::new(80.0, 40.0, 800.0, 360.0),
            floor_y: 400.0,
            terrain: Default::default(),
            hazards: Vec::new(),
            puzzles: Default::default(),
            terrain_style: Default::default(),
            background: Vec::new(),
            gates: Vec::new(),
            map_cell: None,
            piece: None,
            slots: Vec::new(),
            flippable: false,
            exits: vec![
                Exit {
                    edge: RoomEdge::Left,
                    offset: 264.0,
                    length: 96.0,
                    target_room: RoomId::First,
                    spawn: Vec2::new(860.0, 436.0),
                    door: None,
                    locked_by: None,
                },
                Exit {
                    edge: RoomEdge::Right,
                    offset: 264.0,
                    length: 96.0,
                    target_room: RoomId::Boss,
                    spawn: Vec2::new(160.0, 436.0),
                    door: None,
                    locked_by: None,
                },
            ],
            items: vec![],
            charms: vec![],
        },
        Room {
            id: RoomId::Boss,
            name: None,
            bounds: Rect::new(0.0, 0.0, 960.0, FLOOR_Y),
            floor_y: FLOOR_Y,
            terrain: Default::default(),
            hazards: Vec::new(),
            puzzles: Default::default(),
            terrain_style: Default::default(),
            background: Vec::new(),
            gates: Vec::new(),
            map_cell: None,
            piece: None,
            slots: Vec::new(),
            flippable: false,
            spawn: Vec2::new(160.0, 436.0),
            enemies: Vec::new(),
            boss_special: None,
            coins: vec![],
            exits: vec![Exit {
                edge: RoomEdge::Left,
                offset: FLOOR_Y - 96.0,
                length: 96.0,
                target_room: RoomId::Second,
                spawn: Vec2::new(780.0, 376.0),
                door: None,
                locked_by: None,
            }],
            items: vec![],
            charms: vec![],
        },
    ]
}

pub fn load_rooms(assets: &Path) -> Result<[Room; 3], String> {
    load_rooms_from(assets, &read_ron::<RoomEnemies>)
}

/// `load_rooms` with the documents coming from `read` (`editor_files::ReadDocument`).
pub fn load_rooms_from(assets: &Path, read: ReadDocument) -> Result<[Room; 3], String> {
    let definitions = EnemyDefinitions::load(assets)?;
    let bosses = BossDefinitions::load(assets)?;
    let mut errors = Vec::new();
    let mut rooms = default_rooms();
    for (room, file) in rooms
        .iter_mut()
        .zip(["first.ron", "second.ron", "boss.ron"])
    {
        let path = assets.join("rooms").join(file);
        match load_room_enemies_from(&path, &definitions, room.world_bounds(), read) {
            Ok(enemies) => room.enemies = enemies,
            Err(error) => errors.push(error),
        }
        if let Ok(document) = read(&path) {
            if let Some(layout) = document.layout {
                match layout.apply(room) {
                    Ok(()) => {}
                    Err(error) => errors.push(format!("{}: {error}", path.display())),
                }
            }
            if let Err(error) = place_boss(room, document.boss.as_ref(), &bosses) {
                errors.push(format!("{}: {error}", path.display()));
            }
        }
    }
    if errors.is_empty() {
        Ok(rooms)
    } else {
        Err(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_detection_requires_the_player_body_to_reach_the_zone() {
        let [room, _, _] = test_rooms();
        let size = Vec2::new(32.0, 48.0);
        let boundary = room.exits[0].zone(room.bounds).x - size.x / 2.0;
        let floor_center = room.floor_y - size.y / 2.0;

        assert!(room
            .exit_at(Vec2::new(boundary - 0.01, floor_center), size)
            .is_none());
        assert!(room
            .exit_at(Vec2::new(boundary + 0.01, floor_center), size)
            .is_some());
    }

    #[test]
    fn exits_are_anchored_to_all_four_edges() {
        let [mut room, _, _] = test_rooms();
        room.bounds = Rect::new(40.0, 30.0, 400.0, 240.0);
        let size = Vec2::new(16.0, 16.0);
        for (edge, zone, inside, outside) in [
            (
                RoomEdge::Left,
                Rect::new(40.0, 90.0, 32.0, 80.0),
                Vec2::new(48.0, 100.0),
                Vec2::new(48.0, 70.0),
            ),
            (
                RoomEdge::Right,
                Rect::new(408.0, 90.0, 32.0, 80.0),
                Vec2::new(432.0, 100.0),
                Vec2::new(432.0, 70.0),
            ),
            (
                RoomEdge::Top,
                Rect::new(100.0, 30.0, 80.0, 32.0),
                Vec2::new(110.0, 38.0),
                Vec2::new(80.0, 38.0),
            ),
            (
                RoomEdge::Bottom,
                Rect::new(100.0, 238.0, 80.0, 32.0),
                Vec2::new(110.0, 262.0),
                Vec2::new(80.0, 262.0),
            ),
        ] {
            room.exits[0].edge = edge;
            room.exits[0].offset = 60.0;
            room.exits[0].length = 80.0;
            assert_eq!(room.exits[0].zone(room.bounds), zone);
            assert!(room.exit_at(inside, size).is_some(), "{edge:?}");
            assert!(room.exit_at(outside, size).is_none(), "{edge:?}");
        }
    }
}
