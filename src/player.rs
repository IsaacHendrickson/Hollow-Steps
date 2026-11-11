use macroquad::math::Vec2;

use crate::combat::{Attack, StrikeConfig, StrikeDirection};
use crate::events::{EventQueue, GameEvent};
use crate::health::Health;
use crate::item::Item;
use crate::terrain::Terrain;

/// The shipped dash values, kept so existing tests and callers still compile. The game reads
/// `PlayerConfig` (from `data/player.ron`); a test keeps these equal to that file.
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
    /// Up is held: a swing started now strikes upward.
    pub up_held: bool,
    /// Down is held: a swing started now in the air strikes downward.
    pub down_held: bool,
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
    pub dash_speed: f32,
    pub dash_duration: f32,
    /// Minimum time between dash starts.
    pub dash_cooldown: f32,
    /// Melee hitbox sizes per direction and the pogo bounce.
    pub strike: StrikeConfig,
}

/// The shipped values from `data/player.ron`.
impl Default for PlayerConfig {
    fn default() -> Self {
        crate::tuning::TuningConfig::default().player
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
    /// True from the moment the player starts falling until it touches down, so a landing
    /// is only reported after a real fall (not after being placed on a platform).
    falling: bool,
    /// Events raised by `update_with_terrain`, until `take_events` hands them over.
    events: EventQueue,
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
            falling: false,
            events: EventQueue::default(),
        }
    }

    /// Removes and returns the events raised since the last call: jump, landing, dash and
    /// swing start, oldest first.
    pub fn take_events(&mut self) -> Vec<GameEvent> {
        self.events.take()
    }

    pub fn config(&self) -> &PlayerConfig {
        &self.config
    }

    /// Swaps in new tunables mid-game. Position, velocity, timers and items are kept; a
    /// height change keeps the feet where they are so the player is never pushed into the floor.
    pub fn set_config(&mut self, config: PlayerConfig) {
        self.position.y += (self.config.size.y - config.size.y) / 2.0;
        self.config = config;
    }

    pub fn attack(&self) -> &Attack {
        &self.attack
    }

    /// The active melee hitbox for the current swing direction, sized by this player's config.
    pub fn strike_hitbox(&self) -> Option<macroquad::math::Rect> {
        self.attack
            .hitbox_with(self.position, self.config.size, &self.config.strike)
    }

    /// Bounces the player upward after a downward strike hit something, and restores the air
    /// dash. Once per swing, and never on the floor. Returns whether it bounced.
    pub fn pogo(&mut self) -> bool {
        if self.grounded || !self.attack.take_pogo() {
            return false;
        }
        self.velocity.y = -self.config.strike.pogo_speed;
        self.coyote_remaining = 0.0;
        self.air_dash_used = false;
        true
    }

    pub fn health(&self) -> &Health {
        &self.health
    }

    pub fn take_damage(&mut self) -> bool {
        self.health.take_damage()
    }

    pub fn take_damage_amount(&mut self, amount: u16) -> bool {
        self.health.take_damage_amount(amount)
    }

    /// Attack damage shares contact damage's immunity gate; rejected hits never move the player.
    pub fn take_attack_damage(&mut self, hit: crate::enemy_attack::DamageHit) -> bool {
        if !hit.knockback.is_finite() || !hit.knockback_secs.is_finite() || hit.knockback_secs < 0.0
        {
            return false;
        }
        if self.take_damage_amount(hit.damage.clamp(0, i32::from(u16::MAX)) as u16) {
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

    /// Last horizontal facing: `1.0` right, `-1.0` left.
    pub fn facing(&self) -> f32 {
        self.facing
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
        self.falling = false;
        self.resolve_horizontal_bounds(bounds);
        self.resolve_floor(bounds, &Terrain::default());
    }

    /// Advances the player by one simulation step with no solids in the room.
    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32, bounds: WorldBounds) {
        self.update_with_terrain(input, delta_seconds, bounds, &Terrain::default());
    }

    /// Advances the player by one simulation step, colliding with `terrain`
    /// in addition to the world bounds and floor.
    pub fn update_with_terrain(
        &mut self,
        input: PlayerInput,
        delta_seconds: f32,
        bounds: WorldBounds,
        terrain: &Terrain,
    ) {
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
        let strike = StrikeDirection::from_aim(input.up_held, input.down_held, !self.grounded);
        let swing_before = self.attack.id();
        self.attack
            .update_directed(input.attack_pressed, self.facing, strike, delta_seconds);
        if self.attack.id() != swing_before {
            self.events.push(GameEvent::PlayerAttacked);
        }
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
        self.settle_on_floor(bounds, terrain);
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
            self.dash_remaining = self.config.dash_duration;
            self.dash_cooldown = self.config.dash_cooldown;
            self.air_dash_used |= !self.grounded;
            self.events.push(GameEvent::PlayerDashed);
        }

        // Split long frames so dash speed never lasts beyond its duration.
        let dash_seconds = delta_seconds.min(self.dash_remaining);
        if self.dash_remaining > 0.0 {
            self.velocity = Vec2::new(self.facing * self.config.dash_speed, 0.0);
            let contacts = self.displace(self.velocity * dash_seconds, terrain, bounds);
            if contacts.wall {
                self.dash_remaining = 0.0;
            }
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
            self.falling |= !self.grounded && self.velocity.y > 0.0;
            self.displace(self.velocity * normal_seconds, terrain, bounds);
        }
        self.dash_cooldown = (self.dash_cooldown - delta_seconds).max(0.0);
        if self.dash_cooldown < f32::EPSILON {
            self.dash_cooldown = 0.0;
        }

        // Attack displacement is additive; walking, jumping, and dash rules retain their normal path.
        let knockback_dt = delta_seconds.min(self.attack_knockback_remaining);
        let knockback_x = (f64::from(self.position.x)
            + f64::from(self.attack_knockback_x) * f64::from(knockback_dt))
            as f32;
        if terrain.is_empty() {
            self.position.x = knockback_x;
        } else {
            self.displace(
                Vec2::new(knockback_x - self.position.x, 0.0),
                terrain,
                bounds,
            );
        }
        self.attack_knockback_remaining =
            (self.attack_knockback_remaining - delta_seconds).max(0.0);
        self.resolve_horizontal_bounds(bounds);
        self.settle_on_floor(bounds, terrain);
        if self.dash_remaining == 0.0 && self.grounded && self.jump_buffer_remaining > 0.0 {
            self.start_jump();
        }
    }

    fn start_jump(&mut self) {
        self.velocity.y = -self.config.jump_speed;
        self.grounded = false;
        self.coyote_remaining = 0.0;
        self.jump_buffer_remaining = 0.0;
        self.events.push(GameEvent::PlayerJumped);
    }

    /// `resolve_floor`, raising `PlayerLanded` when the player touches down after a fall.
    fn settle_on_floor(&mut self, bounds: WorldBounds, terrain: &Terrain) {
        let was_grounded = self.grounded;
        self.resolve_floor(bounds, terrain);
        if self.grounded {
            if !was_grounded && self.falling {
                self.events.push(GameEvent::PlayerLanded);
            }
            self.falling = false;
        }
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

    /// Moves by `displacement`. Without solids this is plain addition; with
    /// solids each axis is resolved separately in substeps.
    fn displace(
        &mut self,
        displacement: Vec2,
        terrain: &Terrain,
        bounds: WorldBounds,
    ) -> crate::terrain::Contacts {
        if terrain.is_empty() {
            self.position += displacement;
            return Default::default();
        }
        let contacts = terrain.move_body(
            &mut self.position,
            self.config.size,
            displacement,
            crate::terrain::Limits {
                left: bounds.left,
                right: bounds.right,
                floor_y: bounds.floor_y,
            },
        );
        if contacts.wall {
            self.velocity.x = 0.0;
        }
        if (contacts.floor && self.velocity.y > 0.0) || (contacts.ceiling && self.velocity.y < 0.0)
        {
            self.velocity.y = 0.0;
        }
        contacts
    }

    fn resolve_floor(&mut self, bounds: WorldBounds, terrain: &Terrain) {
        let mut floor_position = bounds.floor_y - self.config.size.y / 2.0;
        let mut on_solid = false;
        if let Some(top) = terrain.support_top(self.position, self.config.size) {
            floor_position = floor_position.min(top - self.config.size.y / 2.0);
            on_solid = true;
        }

        if (on_solid || self.position.y >= floor_position) && self.velocity.y >= 0.0 {
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
mod dash_tests;
#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod strike_tests;
#[cfg(test)]
mod tests;
