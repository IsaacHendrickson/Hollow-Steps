use macroquad::math::Vec2;

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
    world_bounds: WorldBounds,
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
            world_bounds,
        }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn world_bounds(&self) -> WorldBounds {
        self.world_bounds
    }

    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32) {
        self.player.update(input, delta_seconds, self.world_bounds);
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
            },
            0.25,
        );

        assert_eq!(game.player().position().x, 125.0);
    }
}
