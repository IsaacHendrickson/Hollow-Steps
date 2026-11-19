use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::player::Player;
use crate::room::RoomId;

pub const ITEM_SIZE: Vec2 = Vec2::new(16.0, 20.0);
pub const PICKUP_REACH: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Item {
    AmberShard,
    Dash,
}

impl Item {
    pub const ALL: &'static [Self] = &[Self::AmberShard, Self::Dash];

    pub fn name(self) -> &'static str {
        match self {
            Self::AmberShard => "Amber Shard",
            Self::Dash => "Dash",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldItem {
    pub item: Item,
    /// Center of the item's rectangle.
    pub position: Vec2,
}

impl WorldItem {
    /// Grounded players can reach items within this distance of their body.
    pub fn can_pick_up(&self, player: &Player) -> bool {
        within_reach(self.position, ITEM_SIZE, player)
    }
}

/// The pickup rule: a grounded player whose body is within `PICKUP_REACH` of the rectangle
/// of `size` centered on `position`. Items and charms share it.
pub fn within_reach(position: Vec2, size: Vec2, player: &Player) -> bool {
    let gap = ((position - player.position()).abs() - (size + player.size()) / 2.0).max(Vec2::ZERO);
    player.is_grounded() && gap.length_squared() <= PICKUP_REACH * PICKUP_REACH
}

/// An item the player picked up during the run, as its room file places it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TakenItem {
    pub room: RoomId,
    pub item: Item,
    pub position: Vec2,
}

/// The items taken during the run, so that a room rebuilt from its file (the editor's Save)
/// does not give them back. Like coins and charms they last for the run and are never written
/// to disk. Items stack on purpose (a room may hold several Amber Shards), so nothing is
/// de-duplicated: each entry stands for exactly one item.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TakenItems {
    taken: Vec<TakenItem>,
}

impl TakenItems {
    /// Remembers that `item` was taken from room `room`.
    pub fn record(&mut self, room: RoomId, item: WorldItem) {
        self.taken.push(TakenItem {
            room,
            item: item.item,
            position: item.position,
        });
    }

    pub fn len(&self) -> usize {
        self.taken.len()
    }

    pub fn is_empty(&self) -> bool {
        self.taken.is_empty()
    }

    /// Room `room` was just rebuilt from its file and `items` is what the file places. Removes
    /// one item for each entry of that room that matches exactly (same kind, same position).
    /// An entry that matches nothing is forgotten: the author moved or deleted that item, so
    /// whatever lies there from now on is a new item and stays.
    pub fn apply_to_rebuilt(&mut self, room: RoomId, items: &mut Vec<WorldItem>) {
        self.taken.retain(|taken| {
            if taken.room != room {
                return true;
            }
            let found = items
                .iter()
                .position(|item| item.item == taken.item && item.position == taken.position);
            if let Some(index) = found {
                items.remove(index);
            }
            found.is_some()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{PlayerConfig, WorldBounds};

    fn shard(x: f32) -> WorldItem {
        WorldItem {
            item: Item::AmberShard,
            position: Vec2::new(x, 450.0),
        }
    }

    fn dash(x: f32) -> WorldItem {
        WorldItem {
            item: Item::Dash,
            position: Vec2::new(x, 450.0),
        }
    }

    #[test]
    fn a_rebuilt_room_loses_exactly_the_items_that_were_taken_from_it() {
        let mut taken = TakenItems::default();
        taken.record(RoomId::First, shard(300.0));
        let mut items = vec![shard(300.0), dash(500.0), shard(340.0)];

        taken.apply_to_rebuilt(RoomId::First, &mut items);

        assert_eq!(items, [dash(500.0), shard(340.0)]);
        assert_eq!(taken.len(), 1, "still remembered for the next rebuild");
        let mut again = vec![shard(300.0), dash(500.0), shard(340.0)];
        taken.apply_to_rebuilt(RoomId::First, &mut again);
        assert_eq!(again, [dash(500.0), shard(340.0)]);
    }

    #[test]
    fn identical_items_are_removed_one_per_entry() {
        let mut taken = TakenItems::default();
        taken.record(RoomId::First, shard(300.0));
        let mut items = vec![shard(300.0), shard(300.0)];
        taken.apply_to_rebuilt(RoomId::First, &mut items);
        assert_eq!(items, [shard(300.0)]);

        taken.record(RoomId::First, shard(300.0));
        let mut items = vec![shard(300.0), shard(300.0), shard(300.0)];
        taken.apply_to_rebuilt(RoomId::First, &mut items);
        assert_eq!(items, [shard(300.0)]);
    }

    #[test]
    fn only_the_rebuilt_room_is_touched_and_a_moved_item_is_forgotten() {
        let mut taken = TakenItems::default();
        taken.record(RoomId::First, shard(300.0));
        taken.record(RoomId::Second, shard(300.0));

        // The second room is rebuilt: its entry matches, the first room's is left alone.
        let mut second = vec![shard(300.0)];
        taken.apply_to_rebuilt(RoomId::Second, &mut second);
        assert!(second.is_empty());
        assert_eq!(taken.len(), 2);

        // The author moved the first room's shard: it is new, and the old entry is dropped, so
        // putting a shard back on the old spot later is new as well.
        let mut first = vec![shard(340.0)];
        taken.apply_to_rebuilt(RoomId::First, &mut first);
        assert_eq!(first, [shard(340.0)]);
        assert_eq!(taken.len(), 1);
        let mut first = vec![shard(300.0)];
        taken.apply_to_rebuilt(RoomId::First, &mut first);
        assert_eq!(first, [shard(300.0)]);
    }

    #[test]
    fn the_kind_and_the_exact_position_must_both_match() {
        let mut taken = TakenItems::default();
        taken.record(RoomId::First, shard(300.0));
        let mut items = vec![dash(300.0), shard(300.1), shard(299.9)];
        taken.apply_to_rebuilt(RoomId::First, &mut items);
        assert_eq!(items.len(), 3);
        assert!(taken.is_empty());
    }

    #[test]
    fn pickup_reach_includes_boundary_but_not_beyond_it() {
        let bounds = WorldBounds::new(0.0, 960.0, 460.0);
        let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
        player.place_on_floor(200.0, bounds);
        let mut item = WorldItem {
            item: Item::AmberShard,
            position: Vec2::new(200.0 + 16.0 + ITEM_SIZE.x / 2.0 + PICKUP_REACH, 450.0),
        };
        assert!(item.can_pick_up(&player));
        item.position.x += 0.1;
        assert!(!item.can_pick_up(&player));
        item.position = Vec2::new(200.0, 300.0);
        assert!(!item.can_pick_up(&player));
    }
}
