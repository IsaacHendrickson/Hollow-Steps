use macroquad::math::Vec2;

use crate::item::WorldItem;
use crate::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use crate::room::{test_rooms, Room, RoomId};

pub use crate::room::FLOOR_Y;

pub const VIEW_WIDTH: f32 = 960.0;
pub const VIEW_HEIGHT: f32 = 540.0;

/// The top-level gameplay state for the current prototype.
///
/// Future systems can be added here as separate fields without making the
/// rendering loop responsible for gameplay rules.
#[derive(Debug)]
pub struct GameState {
    player: Player,
    rooms: [Room; 2],
    current_room: RoomId,
}

impl Default for GameState {
    fn default() -> Self {
        Self::new()
    }
}
impl GameState {
    pub fn new() -> Self {
        let rooms = test_rooms();
        let world_bounds = rooms[RoomId::First.index()].world_bounds();
        let mut player = Player::new(Vec2::new(180.0, 0.0), PlayerConfig::default());
        player.place_on_floor(180.0, world_bounds);

        Self {
            player,
            rooms,
            current_room: RoomId::First,
        }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn items(&self) -> &[WorldItem] {
        &self.room().items
    }

    pub fn room(&self) -> &Room {
        &self.rooms[self.current_room.index()]
    }

    pub fn world_bounds(&self) -> WorldBounds {
        self.room().world_bounds()
    }

    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32) {
        self.player
            .update(input, delta_seconds, self.world_bounds());
        if input.pickup_pressed {
            let room = &mut self.rooms[self.current_room.index()];
            if let Some(index) = room
                .items
                .iter()
                .position(|item| item.can_pick_up(&self.player))
            {
                let item = room.items.remove(index);
                self.player.collect_item(item.item);
            }
        }

        if let Some(exit) = self
            .room()
            .exit_at(self.player.position(), self.player.size())
            .copied()
        {
            self.current_room = exit.target_room;
            self.player.place_at(exit.spawn, self.world_bounds());
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

    #[test]
    fn transition_honors_an_airborne_spawn() {
        let mut game = GameState::new();
        let spawn = Vec2::new(300.0, 200.0);
        game.rooms[RoomId::First.index()].exits[0].spawn = spawn;

        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            10.0,
        );

        assert_eq!(game.room().id, RoomId::Second);
        assert_eq!(game.player().position(), spawn);
        assert_eq!(game.player().velocity(), Vec2::ZERO);
        assert!(!game.player().is_grounded());
    }
}
