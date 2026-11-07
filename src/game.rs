use macroquad::math::Vec2;

use crate::enemy::Enemy;
use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::player::{Player, PlayerConfig, PlayerInput, WorldBounds};

pub const VIEW_WIDTH: f32 = 960.0;
pub const VIEW_HEIGHT: f32 = 540.0;
pub const FLOOR_Y: f32 = 460.0;

/// The top-level gameplay state for the current prototype.
///
/// Future systems can be added here as separate fields without making the
/// rendering loop responsible for gameplay rules.
#[derive(Debug)]
pub struct GameState {
    player: Player,
    room: Room,
    items: Vec<WorldItem>,
}

/// Current room content; the spawn is a floor position expressed as center x.
#[derive(Debug)]
pub struct Room {
    pub bounds: WorldBounds,
    pub spawn_x: f32,
    pub enemies: Vec<Enemy>,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
impl GameState {
    pub fn new() -> Self {
        let world_bounds = WorldBounds::new(0.0, VIEW_WIDTH, FLOOR_Y);
        let mut player = Player::new(Vec2::new(180.0, 0.0), PlayerConfig::default());
        player.place_on_floor(180.0, world_bounds);

        Self {
            player,
            room: Room {
                bounds: world_bounds,
                spawn_x: 180.0,
                enemies: vec![Enemy::new(560.0, 800.0, 90.0, 180.0, world_bounds)],
            },
            items: vec![WorldItem {
                item: Item::AmberShard,
                position: Vec2::new(300.0, FLOOR_Y - ITEM_SIZE.y / 2.0),
            }],
        }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn items(&self) -> &[WorldItem] {
        &self.items
    }

    pub fn world_bounds(&self) -> WorldBounds {
        self.room.bounds
    }

    pub fn room(&self) -> &Room {
        &self.room
    }

    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32) {
        self.player.update(input, delta_seconds, self.room.bounds);
        for enemy in &mut self.room.enemies {
            enemy.update(self.player.position(), delta_seconds, self.room.bounds);
        }
        if self
            .room
            .enemies
            .iter()
            .any(|enemy| enemy.touches(self.player.position(), self.player.size()))
        {
            self.player.take_damage();
        }
        if self.player.health().current() == 0 {
            self.player.respawn(self.room.spawn_x, self.room.bounds);
        }
        if input.pickup_pressed {
            if let Some(index) = self
                .items
                .iter()
                .position(|item| item.can_pick_up(&self.player))
            {
                let item = self.items.remove(index);
                self.player.collect_item(item.item);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_starts_with_player_on_the_floor() {
        let game = GameState::new();

        assert!(game.player().is_grounded());
        assert_eq!(game.player().position().y, FLOOR_Y - 24.0);
    }

    #[test]
    fn game_forwards_input_to_the_player() {
        let mut game = GameState::new();

        game.update(
            PlayerInput {
                move_axis: -1.0,
                jump_pressed: false,
                ..PlayerInput::default()
            },
            0.25,
        );

        assert_eq!(game.player().position().x, 125.0);
    }
}
