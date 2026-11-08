use macroquad::math::Vec2;

use crate::coin::Coin;
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
    coin_count: u32,
    inventory_open: bool,
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
        player.place_at(rooms[RoomId::First.index()].spawn, world_bounds);

        Self {
            player,
            rooms,
            current_room: RoomId::First,
            coin_count: 0,
            inventory_open: false,
        }
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn items(&self) -> &[WorldItem] {
        &self.room().items
    }

    pub fn coins(&self) -> &[Coin] {
        &self.room().coins
    }

    pub fn coin_count(&self) -> u32 {
        self.coin_count
    }

    pub fn inventory_open(&self) -> bool {
        self.inventory_open
    }

    pub fn toggle_inventory(&mut self) {
        self.inventory_open = !self.inventory_open;
    }

    pub fn room(&self) -> &Room {
        &self.rooms[self.current_room.index()]
    }

    pub fn world_bounds(&self) -> WorldBounds {
        self.room().world_bounds()
    }

    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32) {
        if self.inventory_open {
            return;
        }
        self.player
            .update(input, delta_seconds, self.world_bounds());
        let room = &mut self.rooms[self.current_room.index()];
        let bounds = room.world_bounds();
        for enemy in &mut room.enemies {
            enemy.update(self.player.position(), delta_seconds, bounds);
        }
        if let Some(hitbox) = self
            .player
            .attack()
            .hitbox(self.player.position(), self.player.size())
        {
            for enemy in &mut room.enemies {
                if enemy.touches(hitbox.center(), hitbox.size()) {
                    enemy.hit(self.player.attack().id(), self.player.attack().direction());
                }
            }
        }
        room.enemies.retain(|enemy| !enemy.is_dead());
        if room
            .enemies
            .iter()
            .any(|enemy| enemy.touches(self.player.position(), self.player.size()))
        {
            self.player.take_damage();
        }
        if self.player.health().current() == 0 {
            self.player.respawn(room.spawn, bounds);
            return;
        }
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

        let coins = &mut self.rooms[self.current_room.index()].coins;
        coins.retain(|coin| {
            if coin.touches(&self.player) {
                self.coin_count = self.coin_count.saturating_add(1);
                false
            } else {
                true
            }
        });

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
    fn contact_collects_all_touching_coins_once_without_pickup_input() {
        let mut game = GameState::new();
        let position = game.player.position();
        game.rooms[0].coins = vec![Coin { position }; 2];
        game.update(PlayerInput::default(), 0.0);
        assert_eq!(game.coin_count(), 2);
        assert!(game.coins().is_empty());
        game.update(PlayerInput::default(), 0.0);
        assert_eq!(game.coin_count(), 2);
    }

    #[test]
    fn distant_coins_are_not_collected_even_with_pickup_input() {
        let mut game = GameState::new();
        game.update(
            PlayerInput {
                pickup_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
        );
        assert_eq!(game.coin_count(), 0);
        assert_eq!(game.coins().len(), 3);
    }

    #[test]
    fn airborne_contact_collects_coins() {
        let mut game = GameState::new();
        game.player
            .place_at(Vec2::new(360.0, 250.0), game.world_bounds());
        game.rooms[0].coins = vec![Coin {
            position: game.player.position(),
        }];
        assert!(!game.player.is_grounded());
        game.update(PlayerInput::default(), 0.0);
        assert_eq!(game.coin_count(), 1);
    }

    #[test]
    fn coins_persist_through_room_travel_and_respawn() {
        let mut game = GameState::new();
        game.rooms[0].coins = vec![Coin {
            position: game.player.position(),
        }];
        game.update(PlayerInput::default(), 0.0);
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            10.0,
        );
        assert_eq!(game.room().id, RoomId::Second);
        assert_eq!(game.coin_count(), 1);
        for _ in 0..5 {
            game.player.take_damage();
            game.player
                .update(PlayerInput::default(), 1.0, game.world_bounds());
        }
        game.update(PlayerInput::default(), 0.0);
        assert_eq!(game.coin_count(), 1);
        assert_eq!(game.player.health().current(), 5);
        game.update(
            PlayerInput {
                move_axis: -1.0,
                ..PlayerInput::default()
            },
            10.0,
        );
        assert_eq!(game.room().id, RoomId::First);
        assert!(game.coins().is_empty());
        assert_eq!(game.coin_count(), 1);
    }

    #[test]
    fn inventory_pauses_simulation_until_closed() {
        let mut game = GameState::new();
        game.rooms[0].coins = vec![Coin {
            position: game.player.position(),
        }];
        let player = game.player.clone();
        let enemy_position = game.room().enemies[0].position();
        game.toggle_inventory();
        assert!(game.inventory_open());
        game.update(
            PlayerInput {
                move_axis: 1.0,
                jump_pressed: true,
                pickup_pressed: true,
                attack_pressed: true,
                jump_held: true,
                dash_pressed: true,
            },
            10.0,
        );
        assert_eq!(game.player, player);
        assert_eq!(game.room().enemies[0].position(), enemy_position);
        assert_eq!(game.coin_count(), 0);
        assert_eq!(game.room().id, RoomId::First);
        game.toggle_inventory();
        assert!(!game.inventory_open());
        game.update(PlayerInput::default(), 0.0);
        assert_eq!(game.coin_count(), 1);
    }

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
    fn death_uses_current_room_spawn_and_inactive_enemies_stay_frozen() {
        let mut game = GameState::new();
        game.current_room = RoomId::Second;
        let first_enemy_position = game.rooms[RoomId::First.index()].enemies[0].position();
        let bounds = game.world_bounds();
        let room = &mut game.rooms[RoomId::Second.index()];
        room.spawn = Vec2::new(350.0, 200.0);
        room.enemies
            .push(crate::enemy::Enemy::new(500.0, 500.0, 0.0, 0.0, bounds));
        game.player.place_on_floor(500.0, bounds);
        for expected in [4, 3, 2, 1, 5] {
            game.update(PlayerInput::default(), 1.0);
            assert_eq!(game.player.health().current(), expected);
        }
        assert_eq!(game.room().id, RoomId::Second);
        assert_eq!(game.player.position(), game.room().spawn);
        assert!(!game.player.is_grounded());
        assert_eq!(game.player.velocity(), Vec2::ZERO);
        assert_eq!(
            game.rooms[RoomId::First.index()].enemies[0].position(),
            first_enemy_position
        );
    }

    #[test]
    fn melee_respects_range_facing_and_hits_each_enemy_once_per_swing() {
        let mut game = GameState::new();
        let bounds = game.world_bounds();
        game.player.place_on_floor(200.0, bounds);
        game.rooms[0].enemies = vec![
            crate::enemy::Enemy::new(250.0, 250.0, 0.0, 0.0, bounds),
            crate::enemy::Enemy::new(150.0, 150.0, 0.0, 0.0, bounds),
            crate::enemy::Enemy::new(400.0, 400.0, 0.0, 0.0, bounds),
        ];
        let swing = PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        };
        game.update(swing, 0.0);
        game.update(swing, 0.0);
        assert_eq!(game.room().enemies[0].health(), 2);
        assert_eq!(game.room().enemies[1].health(), 3);
        assert_eq!(game.room().enemies[2].health(), 3);
        game.update(PlayerInput::default(), crate::combat::ATTACK_COOLDOWN);
        game.update(
            PlayerInput {
                move_axis: -1.0,
                attack_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
        );
        assert_eq!(game.room().enemies[1].health(), 2);
    }

    #[test]
    fn lethal_swing_removes_enemy_before_contact_damage_and_death_persists() {
        let mut game = GameState::new();
        let bounds = game.world_bounds();
        game.player.place_on_floor(200.0, bounds);
        game.rooms[0].enemies = vec![crate::enemy::Enemy::new(230.0, 230.0, 0.0, 0.0, bounds)];
        game.rooms[0].enemies[0].hit(100, 1.0);
        game.rooms[0].enemies[0].hit(101, 1.0);
        game.update(
            PlayerInput {
                attack_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
        );
        assert!(game.room().enemies.is_empty());
        assert_eq!(game.player.health().current(), 5);
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            10.0,
        );
        assert_eq!(game.room().id, RoomId::Second);
        game.update(
            PlayerInput {
                move_axis: -1.0,
                ..PlayerInput::default()
            },
            10.0,
        );
        assert_eq!(game.room().id, RoomId::First);
        assert!(game.room().enemies.is_empty());
    }

    #[test]
    fn room_transition_clears_active_dash_and_preserves_ability() {
        let mut game = GameState::new();
        game.player.collect_item(crate::item::Item::Dash);
        game.player.place_on_floor(900.0, game.world_bounds());
        game.update(
            PlayerInput {
                dash_pressed: true,
                ..PlayerInput::default()
            },
            0.05,
        );

        assert_eq!(game.room().id, RoomId::Second);
        let spawn = game.player.position();
        game.update(PlayerInput::default(), 0.05);
        assert_eq!(game.player.position(), spawn);
        assert_eq!(game.player.velocity(), Vec2::ZERO);
        game.update(
            PlayerInput {
                dash_pressed: true,
                ..PlayerInput::default()
            },
            0.05,
        );
        assert_eq!(game.player.position().x, spawn.x + 40.0);
    }

    #[test]
    fn death_clears_active_dash_and_preserves_collected_ability() {
        let mut game = GameState::new();
        let bounds = game.world_bounds();
        for _ in 0..4 {
            assert!(game.player.take_damage());
            game.player.update(PlayerInput::default(), 1.0, bounds);
        }
        game.player.collect_item(crate::item::Item::Dash);
        game.player.place_on_floor(500.0, bounds);
        game.rooms[0].enemies = vec![crate::enemy::Enemy::new(500.0, 500.0, 0.0, 0.0, bounds)];
        game.update(
            PlayerInput {
                dash_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
        );

        assert_eq!(game.player.health().current(), crate::health::MAX_HEALTH);
        assert!(game.player.health().is_invincible());
        let spawn = game.room().spawn;
        assert_eq!(game.player.position(), spawn);
        game.update(PlayerInput::default(), 0.05);
        assert_eq!(game.player.position(), spawn);
        assert_eq!(game.player.velocity(), Vec2::ZERO);
        assert_eq!(game.player.collected_items(), &[crate::item::Item::Dash]);
        game.update(
            PlayerInput {
                dash_pressed: true,
                ..PlayerInput::default()
            },
            0.05,
        );
        assert_eq!(game.player.position().x, spawn.x + 40.0);
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
