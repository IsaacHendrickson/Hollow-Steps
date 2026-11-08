use macroquad::math::{Rect, Vec2};

use crate::boss::Boss;
use crate::boss_special::{BossSpecials, SpecialAttack};
use crate::coin::Coin;
use crate::enemy::Enemy;
use crate::item::WorldItem;
use crate::player::WorldBounds;

pub const FLOOR_Y: f32 = 460.0;
pub const EXIT_DEPTH: f32 = 32.0;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RoomId {
    First,
    Second,
    Boss,
    /// Additional room IDs supplied by data files.
    Named(String),
}

impl From<String> for RoomId {
    fn from(name: String) -> Self {
        match name.as_str() {
            "first" => Self::First,
            "second" => Self::Second,
            "boss" => Self::Boss,
            _ => Self::Named(name),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum RoomEdge {
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq)]
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
    pub exits: Vec<Exit>,
    /// Default player-center respawn point for this room.
    pub spawn: Vec2,
    pub enemies: Vec<Enemy>,
    pub boss: Option<Boss>,
    pub boss_special: Option<BossSpecials>,
    // Encounter progress belongs to its room so inactive rooms stay paused.
    pub(crate) special_attack_timer: f32,
    pub(crate) next_special_attack: SpecialAttack,
    pub(crate) last_special_hit: Option<u64>,
    pub(crate) spawned_enemy_count: usize,
    pub(crate) spawned_coin_count: usize,
    pub(crate) coins: Vec<Coin>,
    pub(crate) items: Vec<WorldItem>,
}

impl Room {
    pub fn world_bounds(&self) -> WorldBounds {
        WorldBounds::new(self.bounds.x, self.bounds.right(), self.floor_y)
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
        self.boss.as_ref().is_some_and(|boss| !boss.is_dead())
            || self
                .boss_special
                .as_ref()
                .is_some_and(|boss| !boss.is_defeated())
    }
}

/// Load the original three rooms through the same validated loader as the game.
pub fn test_rooms() -> [Room; 3] {
    let mut rooms = crate::room_data::load_rooms(crate::room_data::DEFAULT_ROOMS_DIR)
        .unwrap_or_else(|error| panic!("{error}"));
    [RoomId::First, RoomId::Second, RoomId::Boss]
        .map(|id| rooms.remove(&id).expect("original room must exist"))
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
