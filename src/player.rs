use macroquad::math::Vec2;

use crate::combat::Attack;
use crate::health::Health;
use crate::item::Item;

pub const DASH_SPEED: f32 = 800.0;
pub const DASH_DURATION: f32 = 0.2;
/// Minimum time between dash starts.
pub const DASH_COOLDOWN: f32 = 0.5;

/// A single frame of player intent.
///
/// `jump_pressed` is an edge-triggered input: it should be true only on the
/// frame the jump key was pressed, not while the key is held down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput {
    pub move_axis: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    /// Edge-triggered melee intent (X).
    pub attack_pressed: bool,
    /// Edge-triggered pickup intent (Up).
    pub pickup_pressed: bool,
    /// Edge-triggered dash intent (Shift).
    pub dash_pressed: bool,
}

/// Tunable values that define how the player feels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerConfig {
    pub move_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub max_fall_speed: f32,
    pub held_jump_gravity_multiplier: f32,
    pub coyote_seconds: f32,
    pub jump_buffer_seconds: f32,
    pub size: Vec2,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            move_speed: 220.0,
            jump_speed: 480.0,
            gravity: 1_400.0,
            max_fall_speed: 900.0,
            held_jump_gravity_multiplier: 0.5,
            coyote_seconds: 0.1,
            jump_buffer_seconds: 0.12,
            size: Vec2::new(32.0, 48.0),
        }
    }
}
/// The playable rectangular area for the first prototype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBounds {
    pub left: f32,
    /// Ceiling metadata used by enemy attacks; normal player movement is unchanged.
    pub top: f32,
    pub right: f32,
    /// The y coordinate of the top of the floor.
    pub floor_y: f32,
}

impl WorldBounds {
    pub const fn with_top(mut self, top: f32) -> Self {
        self.top = top;
        self
    }
    pub const fn new(left: f32, right: f32, floor_y: f32) -> Self {
        Self {
            left,
            top: 0.0,
            right,
            floor_y,
        }
    }
}

/// Player gameplay state.
#[derive(Clone, Debug, PartialEq)]
pub struct Player {
    position: Vec2,
    velocity: Vec2,
    config: PlayerConfig,
    grounded: bool,
    collected_items: Vec<Item>,
    facing: f32,
    dash_remaining: f32,
    dash_cooldown: f32,
    air_dash_used: bool,
    health: Health,
    attack: Attack,
    coyote_remaining: f32,
    jump_buffer_remaining: f32,
    attack_knockback_x: f32,
    attack_knockback_remaining: f32,
}

impl Player {
    /// Creates a player whose position is the center of its body.
    pub fn new(position: Vec2, config: PlayerConfig) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            config,
            grounded: false,
            collected_items: Vec::new(),
            facing: 1.0,
            dash_remaining: 0.0,
            dash_cooldown: 0.0,
            air_dash_used: false,
            health: Health::default(),
            attack: Attack::default(),
            coyote_remaining: 0.0,
            jump_buffer_remaining: 0.0,
            attack_knockback_x: 0.0,
            attack_knockback_remaining: 0.0,
        }
    }

    pub fn attack(&self) -> &Attack {
        &self.attack
    }

    pub fn health(&self) -> &Health {
        &self.health
    }

    pub fn take_damage(&mut self) -> bool {
        self.health.take_damage()
    }

    pub fn take_damage_amount(&mut self, amount: u8) -> bool {
        self.health.take_damage_amount(amount)
    }

    /// Attack damage shares contact damage's immunity gate; rejected hits never move the player.
    pub fn take_attack_damage(&mut self, hit: crate::enemy_attack::DamageHit) -> bool {
        if !hit.knockback.is_finite() || !hit.knockback_secs.is_finite() || hit.knockback_secs < 0.0
        {
            return false;
        }
        if self.take_damage_amount(hit.damage.clamp(0, i32::from(u8::MAX)) as u8) {
            self.velocity += hit.knockback;
            self.attack_knockback_x = hit.knockback.x;
            self.attack_knockback_remaining = hit.knockback_secs;
            if hit.knockback.y < 0.0 {
                self.grounded = false;
            }
            true
        } else {
            false
        }
    }

    pub fn respawn(&mut self, spawn: Vec2, bounds: WorldBounds) {
        self.place_at(spawn, bounds);
        self.health.respawn();
    }

    pub fn collected_items(&self) -> &[Item] {
        &self.collected_items
    }

    pub(crate) fn collect_item(&mut self, item: Item) {
        self.collected_items.push(item);
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    pub fn size(&self) -> Vec2 {
        self.config.size
    }

    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    /// Places the player on the floor and clears movement left over from a
    /// previous life or room transition.
    pub fn place_on_floor(&mut self, x: f32, bounds: WorldBounds) {
        self.place_at(
            Vec2::new(x, bounds.floor_y - self.config.size.y / 2.0),
            bounds,
        );
    }

    /// Places the player at a room spawn, preserving inventory and clearing
    /// velocity. Spawns above the floor remain airborne.
    pub fn place_at(&mut self, position: Vec2, bounds: WorldBounds) {
        self.position = position;
        self.velocity = Vec2::ZERO;
        self.attack_knockback_x = 0.0;
        self.attack_knockback_remaining = 0.0;
        self.attack.cancel();
        self.coyote_remaining = 0.0;
        self.jump_buffer_remaining = 0.0;
        self.dash_remaining = 0.0;
        self.dash_cooldown = 0.0;
        self.air_dash_used = false;
        self.resolve_horizontal_bounds(bounds);
        self.resolve_floor(bounds);
    }

    /// Advances the player by one simulation step.
    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32, bounds: WorldBounds) {
        let delta_seconds = if delta_seconds.is_finite() {
            delta_seconds.max(0.0)
        } else {
            0.0
        };
        self.health.update(delta_seconds);
        let move_axis = if input.move_axis.is_finite() {
            input.move_axis.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        if self.dash_remaining == 0.0 && move_axis != 0.0 {
            self.facing = move_axis.signum();
        }
        self.attack
            .update(input.attack_pressed, self.facing, delta_seconds);
        self.velocity.x = move_axis * self.config.move_speed;

        self.coyote_remaining = if self.grounded {
            self.config.coyote_seconds.max(0.0)
        } else {
            (self.coyote_remaining - delta_seconds).max(0.0)
        };
        self.jump_buffer_remaining = if input.jump_pressed {
            self.config.jump_buffer_seconds.max(f32::EPSILON)
        } else {
            (self.jump_buffer_remaining - delta_seconds).max(0.0)
        };
        // Recheck support so stepping off a ledge uses the grace timer.
        self.resolve_floor(bounds);
        if self.dash_remaining == 0.0
            && self.jump_buffer_remaining > 0.0
            && (self.grounded || self.coyote_remaining > 0.0)
        {
            self.start_jump();
        }

        if input.jump_pressed {
            // A new airborne press must also expire during a long landing step.
            self.jump_buffer_remaining = (self.jump_buffer_remaining - delta_seconds).max(0.0);
        }

        if input.dash_pressed
            && self.collected_items.contains(&Item::Dash)
            && self.dash_remaining == 0.0
            && self.dash_cooldown == 0.0
            && (self.grounded || !self.air_dash_used)
        {
            self.dash_remaining = DASH_DURATION;
            self.dash_cooldown = DASH_COOLDOWN;
            self.air_dash_used |= !self.grounded;
        }

        // Split long frames so dash speed never lasts beyond its duration.
        let dash_seconds = delta_seconds.min(self.dash_remaining);
        if self.dash_remaining > 0.0 {
            self.velocity = Vec2::new(self.facing * DASH_SPEED, 0.0);
            self.position += self.velocity * dash_seconds;
            self.dash_remaining = (self.dash_remaining - dash_seconds).max(0.0);
            if self.dash_remaining < f32::EPSILON {
                self.dash_remaining = 0.0;
            }
        }
        let normal_seconds = delta_seconds - dash_seconds;
        if self.dash_remaining == 0.0 {
            self.velocity.x = move_axis * self.config.move_speed;
            let gravity_multiplier = if input.jump_held && self.velocity.y < 0.0 {
                self.config.held_jump_gravity_multiplier.clamp(0.0, 1.0)
            } else {
                1.0
            };
            self.velocity.y = (self.velocity.y
                + self.config.gravity * gravity_multiplier * normal_seconds)
                .min(self.config.max_fall_speed);
            self.position += self.velocity * normal_seconds;
        }
        self.dash_cooldown = (self.dash_cooldown - delta_seconds).max(0.0);
        if self.dash_cooldown < f32::EPSILON {
            self.dash_cooldown = 0.0;
        }

        // Attack displacement is additive; walking, jumping, and dash rules retain their normal path.
        let knockback_dt = delta_seconds.min(self.attack_knockback_remaining);
        self.position.x = (f64::from(self.position.x)
            + f64::from(self.attack_knockback_x) * f64::from(knockback_dt))
            as f32;
        self.attack_knockback_remaining =
            (self.attack_knockback_remaining - delta_seconds).max(0.0);
        self.resolve_horizontal_bounds(bounds);
        self.resolve_floor(bounds);
        if self.dash_remaining == 0.0 && self.grounded && self.jump_buffer_remaining > 0.0 {
            self.start_jump();
        }
    }

    fn start_jump(&mut self) {
        self.velocity.y = -self.config.jump_speed;
        self.grounded = false;
        self.coyote_remaining = 0.0;
        self.jump_buffer_remaining = 0.0;
    }

    fn resolve_horizontal_bounds(&mut self, bounds: WorldBounds) {
        let half_width = self.config.size.x / 2.0;
        let min_x = bounds.left + half_width;
        let max_x = bounds.right - half_width;

        if self.position.x < min_x {
            self.position.x = min_x;
        } else if self.position.x > max_x {
            self.position.x = max_x;
        }
    }

    fn resolve_floor(&mut self, bounds: WorldBounds) {
        let floor_position = bounds.floor_y - self.config.size.y / 2.0;

        if self.position.y >= floor_position && self.velocity.y >= 0.0 {
            self.position.y = floor_position;
            self.velocity.y = 0.0;
            self.grounded = true;
            self.air_dash_used = false;
        } else {
            self.grounded = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

    fn player_on_floor() -> Player {
        let config = PlayerConfig::default();
        let mut player = Player::new(Vec2::ZERO, config);
        player.place_on_floor(200.0, BOUNDS);
        player
    }

    #[test]
    fn horizontal_input_moves_at_configured_speed() {
        let mut player = player_on_floor();

        player.update(
            PlayerInput {
                move_axis: 1.0,
                jump_pressed: false,
                ..PlayerInput::default()
            },
            0.5,
            BOUNDS,
        );

        assert!((player.position().x - 310.0).abs() < f32::EPSILON);
        assert!((player.velocity().x - 220.0).abs() < f32::EPSILON);
    }

    #[test]
    fn horizontal_input_is_clamped_to_the_world() {
        let mut player = player_on_floor();

        player.update(
            PlayerInput {
                move_axis: 1.0,
                jump_pressed: false,
                ..PlayerInput::default()
            },
            10.0,
            BOUNDS,
        );

        assert_eq!(player.position().x, 944.0);
    }

    #[test]
    fn jump_applies_upward_velocity_and_gravity() {
        let mut player = player_on_floor();

        player.update(
            PlayerInput {
                move_axis: 0.0,
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
            BOUNDS,
        );

        assert!(!player.is_grounded());
        assert!((player.velocity().y + 480.0).abs() < f32::EPSILON);

        player.update(PlayerInput::default(), 0.1, BOUNDS);

        assert!((player.velocity().y + 340.0).abs() < f32::EPSILON);
        assert!(player.position().y < 436.0);
    }

    #[test]
    fn jump_input_is_ignored_while_airborne() {
        let mut player = player_on_floor();
        player.update(
            PlayerInput {
                move_axis: 0.0,
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
            BOUNDS,
        );

        player.update(
            PlayerInput {
                move_axis: 0.0,
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.1,
            BOUNDS,
        );

        assert!((player.velocity().y + 340.0).abs() < f32::EPSILON);
    }

    #[test]
    fn falling_player_lands_on_the_floor() {
        let mut player = player_on_floor();
        player.update(PlayerInput::default(), 1.0, BOUNDS);

        assert_eq!(player.position().y, 436.0);
        assert_eq!(player.velocity().y, 0.0);
        assert!(player.is_grounded());
    }

    #[test]
    fn room_spawn_preserves_height_and_inventory_but_clears_movement() {
        let mut player = player_on_floor();
        player.collect_item(Item::AmberShard);
        player.update(
            PlayerInput {
                move_axis: 1.0,
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.1,
            BOUNDS,
        );

        player.place_at(Vec2::new(100.0, 200.0), BOUNDS);

        assert_eq!(player.position(), Vec2::new(100.0, 200.0));
        assert_eq!(player.velocity(), Vec2::ZERO);
        assert!(!player.is_grounded());
        assert_eq!(player.collected_items(), &[Item::AmberShard]);
    }
}

#[cfg(test)]
mod dash_tests {
    use super::*;

    const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
    const DASH: PlayerInput = PlayerInput {
        dash_pressed: true,
        move_axis: 0.0,
        jump_pressed: false,
        pickup_pressed: false,
        jump_held: false,
        attack_pressed: false,
    };

    fn equipped_player() -> Player {
        let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
        player.place_on_floor(400.0, BOUNDS);
        player.collect_item(Item::Dash);
        player
    }

    #[test]
    fn dash_requires_pickup() {
        let mut player = equipped_player();
        player.collected_items.clear();
        player.update(DASH, 0.1, BOUNDS);
        assert_eq!(player.position.x, 400.0);
    }

    #[test]
    fn dash_uses_last_facing_and_expires_after_point_two_seconds() {
        let mut player = equipped_player();
        player.update(
            PlayerInput {
                move_axis: -1.0,
                ..PlayerInput::default()
            },
            0.0,
            BOUNDS,
        );
        player.update(DASH, 0.1, BOUNDS);
        assert_eq!(player.position, Vec2::new(320.0, 436.0));
        assert_eq!(player.velocity.x, -DASH_SPEED);
        // Opposite input cannot reverse a dash in progress.
        player.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            0.1,
            BOUNDS,
        );
        assert_eq!(player.position.x, 240.0);
        player.update(PlayerInput::default(), 0.1, BOUNDS);
        assert_eq!(player.position.x, 240.0);
    }

    #[test]
    fn cooldown_blocks_another_dash_until_half_a_second_after_start() {
        let mut player = equipped_player();
        player.update(DASH, 0.25, BOUNDS);
        let x = player.position.x;
        player.update(DASH, 0.25, BOUNDS);
        assert_eq!(player.position.x, x);
        player.update(DASH, 0.1, BOUNDS);
        assert_eq!(player.position.x, x + 80.0);
    }

    #[test]
    fn air_dash_is_horizontal_and_only_landing_restores_it() {
        let mut player = equipped_player();
        player.update(
            PlayerInput {
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
            BOUNDS,
        );
        for _ in 0..20 {
            player.update(PlayerInput::default(), 0.01, BOUNDS);
        }
        let y = player.position.y;
        player.update(DASH, 0.2, BOUNDS);
        assert_eq!(player.position.y, y);
        for _ in 0..30 {
            player.update(PlayerInput::default(), 0.01, BOUNDS);
        }
        assert!(!player.is_grounded());
        let x = player.position.x;
        player.update(DASH, 0.0, BOUNDS);
        assert_eq!(player.dash_remaining, 0.0);
        assert_eq!(player.position.x, x);
        player.update(PlayerInput::default(), 1.0, BOUNDS);
        assert!(player.is_grounded());
        player.update(
            PlayerInput {
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.1,
            BOUNDS,
        );
        player.update(DASH, 0.1, BOUNDS);
        assert_eq!(player.velocity.x, DASH_SPEED);
        assert_eq!(player.velocity.y, 0.0);
        assert!(!player.is_grounded());
    }

    #[test]
    fn buffered_jump_waits_for_ground_dash_to_finish() {
        let mut player = equipped_player();
        player.update(DASH, 0.1, BOUNDS);
        player.update(
            PlayerInput {
                jump_pressed: true,
                jump_held: true,
                ..PlayerInput::default()
            },
            0.05,
            BOUNDS,
        );
        assert_eq!(player.position.y, 436.0);
        assert_eq!(player.velocity.y, 0.0);
        player.update(
            PlayerInput {
                jump_held: true,
                ..PlayerInput::default()
            },
            0.05,
            BOUNDS,
        );
        assert_eq!(player.position.x, 560.0);
        assert!(!player.is_grounded());
        assert_eq!(player.velocity.y, -player.config.jump_speed);
    }

    #[test]
    fn attack_during_dash_keeps_dash_and_swing_direction() {
        let mut player = equipped_player();
        player.update(DASH, 0.05, BOUNDS);
        player.update(
            PlayerInput {
                move_axis: -1.0,
                attack_pressed: true,
                ..PlayerInput::default()
            },
            0.05,
            BOUNDS,
        );
        assert_eq!(player.position.x, 480.0);
        assert_eq!(player.attack.direction(), 1.0);
        assert!(player
            .attack
            .hitbox(player.position, player.size())
            .is_some());
    }

    #[test]
    fn zero_and_large_steps_preserve_duration_and_bounds() {
        let mut player = equipped_player();
        player.update(DASH, 0.0, BOUNDS);
        assert_eq!(player.position.x, 400.0);
        player.update(PlayerInput::default(), 10.0, BOUNDS);
        assert_eq!(player.position.x, 560.0);
        player.place_on_floor(900.0, BOUNDS);
        player.update(DASH, 0.2, BOUNDS);
        assert_eq!(player.position.x, 944.0);
        player.place_on_floor(30.0, BOUNDS);
        player.update(
            PlayerInput {
                move_axis: -1.0,
                ..DASH
            },
            10.0,
            BOUNDS,
        );
        assert_eq!(player.position.x, 16.0);
        assert!(player.is_grounded());
    }
}
