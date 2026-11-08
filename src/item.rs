use macroquad::math::Vec2;

use crate::player::Player;

pub const ITEM_SIZE: Vec2 = Vec2::new(16.0, 20.0);
pub const PICKUP_REACH: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize)]
pub enum Item {
    AmberShard,
    Dash,
}

impl Item {
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
        let gap = ((self.position - player.position()).abs() - (ITEM_SIZE + player.size()) / 2.0)
            .max(Vec2::ZERO);
        player.is_grounded() && gap.length_squared() <= PICKUP_REACH * PICKUP_REACH
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::{PlayerConfig, WorldBounds};

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
