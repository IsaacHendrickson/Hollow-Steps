use std::path::Path;

use macroquad::math::{Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::behavior::read_ron;
use crate::boss_definition::BossDefinitions;
use crate::boss_special::BossSpecials;
use crate::coin::{Coin, COIN_RADIUS};
use crate::enemy::{Enemy, EnemyDefinitions};
use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::player::WorldBounds;

pub const FLOOR_Y: f32 = 460.0;
pub const EXIT_DEPTH: f32 = 32.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

impl Exit {
    pub fn zone(&self, bounds: Rect) -> Rect {
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
    pub bounds: Rect,
    pub floor_y: f32,
    /// Solid rectangles that block the player (platforms and walls).
    pub terrain: crate::terrain::Terrain,
    pub exits: Vec<Exit>,
    /// Default player-center respawn point for this room.
    pub spawn: Vec2,
    pub enemies: Vec<Enemy>,
    pub boss_special: Option<BossSpecials>,
    pub(crate) coins: Vec<Coin>,
    pub(crate) items: Vec<WorldItem>,
}

impl Room {
    pub fn world_bounds(&self) -> WorldBounds {
        WorldBounds::new(self.bounds.x, self.bounds.right(), self.floor_y).with_top(self.bounds.y)
    }

    pub fn exit_at(&self, position: Vec2, size: Vec2) -> Option<&Exit> {
        if self.exits_locked() {
            return None;
        }
        let body = Rect::new(
            position.x - size.x / 2.0,
            position.y - size.y / 2.0,
            size.x,
            size.y,
        );
        self.exits
            .iter()
            .find(|exit| exit.zone(self.bounds).overlaps(&body))
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
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossPlacement {
    pub definition: String,
    pub position: (f32, f32),
}

pub fn place_boss(
    room: &mut Room,
    placement: Option<&BossPlacement>,
    definitions: &BossDefinitions,
) -> Result<(), String> {
    room.boss_special = placement
        .map(|placement| {
            let boss = definitions.spawn(&placement.definition, placement.position.into())?;
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
    let room: RoomEnemies = read_ron(path)?;
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
            exits: vec![Exit {
                edge: RoomEdge::Right,
                offset: FLOOR_Y - 96.0,
                length: 96.0,
                target_room: RoomId::Second,
                spawn: Vec2::new(160.0, 376.0),
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
        },
        Room {
            id: RoomId::Second,
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
            exits: vec![
                Exit {
                    edge: RoomEdge::Left,
                    offset: 264.0,
                    length: 96.0,
                    target_room: RoomId::First,
                    spawn: Vec2::new(860.0, 436.0),
                },
                Exit {
                    edge: RoomEdge::Right,
                    offset: 264.0,
                    length: 96.0,
                    target_room: RoomId::Boss,
                    spawn: Vec2::new(160.0, 436.0),
                },
            ],
            items: vec![],
        },
        Room {
            id: RoomId::Boss,
            bounds: Rect::new(0.0, 0.0, 960.0, FLOOR_Y),
            floor_y: FLOOR_Y,
            terrain: Default::default(),
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
            }],
            items: vec![],
        },
    ]
}

pub fn load_rooms(assets: &Path) -> Result<[Room; 3], String> {
    let definitions = EnemyDefinitions::load(assets)?;
    let bosses = BossDefinitions::load(assets)?;
    let mut errors = Vec::new();
    let mut rooms = default_rooms();
    for (room, file) in rooms
        .iter_mut()
        .zip(["first.ron", "second.ron", "boss.ron"])
    {
        let path = assets.join("rooms").join(file);
        match load_room_enemies(&path, &definitions, room.world_bounds()) {
            Ok(enemies) => room.enemies = enemies,
            Err(error) => errors.push(error),
        }
        if let Ok(document) = read_ron::<RoomEnemies>(&path) {
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
