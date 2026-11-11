use macroquad::math::{Rect, Vec2};

pub const SWING_SECONDS: f32 = 0.12;
pub const ATTACK_COOLDOWN: f32 = 0.35;
/// The shipped forward hitbox size, kept so existing tests and callers still compile. The game
/// reads `StrikeConfig` (from `data/player.ron`); a test keeps these equal to that file.
pub const ATTACK_REACH: f32 = 38.0;
pub const ATTACK_HEIGHT: f32 = 32.0;

/// Which way a swing points.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StrikeDirection {
    /// The way the player faces.
    #[default]
    Forward,
    Up,
    Down,
}

impl StrikeDirection {
    /// The direction for a swing started with these keys held. Up wins when both are held.
    /// Down only counts in the air; on the floor it is an ordinary forward swing.
    pub fn from_aim(up_held: bool, down_held: bool, airborne: bool) -> Self {
        if up_held {
            Self::Up
        } else if down_held && airborne {
            Self::Down
        } else {
            Self::Forward
        }
    }
}

/// Melee tunables from `data/player.ron`: one hitbox size per direction and the pogo bounce.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrikeConfig {
    /// Forward box: `width` reaches out from the player's side.
    pub forward_width: f32,
    pub forward_height: f32,
    /// Up box: sits on top of the player, centered, `height` reaches upward.
    pub up_width: f32,
    pub up_height: f32,
    /// Down box: sits under the player's feet, centered, `height` reaches downward.
    pub down_width: f32,
    pub down_height: f32,
    /// Upward speed given by a downward strike that hits something.
    pub pogo_speed: f32,
}

impl StrikeConfig {
    /// Width and height of the hitbox for `direction`.
    pub fn size(&self, direction: StrikeDirection) -> Vec2 {
        match direction {
            StrikeDirection::Forward => Vec2::new(self.forward_width, self.forward_height),
            StrikeDirection::Up => Vec2::new(self.up_width, self.up_height),
            StrikeDirection::Down => Vec2::new(self.down_width, self.down_height),
        }
    }
}

/// The shipped values from `data/player.ron`.
impl Default for StrikeConfig {
    fn default() -> Self {
        crate::tuning::TuningConfig::default().player.strike
    }
}

/// Simulation-only melee state. Each swing may damage each enemy once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attack {
    id: u64,
    active_remaining: f32,
    cooldown_remaining: f32,
    direction: f32,
    strike: StrikeDirection,
    pogo_spent: bool,
}

impl Attack {
    /// Starts a forward swing when `pressed` and off cooldown.
    pub fn update(&mut self, pressed: bool, facing: f32, dt: f32) {
        self.update_directed(pressed, facing, StrikeDirection::Forward, dt);
    }

    /// Like `update`, with the direction a new swing points. A swing already under way keeps
    /// the direction it started with.
    pub fn update_directed(
        &mut self,
        pressed: bool,
        facing: f32,
        strike: StrikeDirection,
        dt: f32,
    ) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.active_remaining = (self.active_remaining - dt).max(0.0);
        self.cooldown_remaining = (self.cooldown_remaining - dt).max(0.0);
        if pressed && self.cooldown_remaining == 0.0 {
            self.id += 1;
            self.direction = facing;
            self.strike = strike;
            self.pogo_spent = false;
            self.active_remaining = SWING_SECONDS;
            self.cooldown_remaining = ATTACK_COOLDOWN;
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    /// Horizontal facing the current swing started with (also the side of a forward swing).
    pub fn direction(&self) -> f32 {
        self.direction
    }

    /// Which way the current (or last) swing points.
    pub fn strike(&self) -> StrikeDirection {
        self.strike
    }

    /// The active hitbox using the shipped `StrikeConfig`.
    pub fn hitbox(&self, position: Vec2, size: Vec2) -> Option<Rect> {
        self.hitbox_with(position, size, &StrikeConfig::default())
    }

    /// The active hitbox for a player body at `position` (center) of `size`, sized by `config`.
    pub fn hitbox_with(&self, position: Vec2, size: Vec2, config: &StrikeConfig) -> Option<Rect> {
        (self.active_remaining > 0.0).then(|| {
            let reach = config.size(self.strike);
            match self.strike {
                StrikeDirection::Forward => Rect::new(
                    if self.direction > 0.0 {
                        position.x + size.x / 2.0
                    } else {
                        position.x - size.x / 2.0 - reach.x
                    },
                    position.y - reach.y / 2.0,
                    reach.x,
                    reach.y,
                ),
                StrikeDirection::Up => Rect::new(
                    position.x - reach.x / 2.0,
                    position.y - size.y / 2.0 - reach.y,
                    reach.x,
                    reach.y,
                ),
                StrikeDirection::Down => Rect::new(
                    position.x - reach.x / 2.0,
                    position.y + size.y / 2.0,
                    reach.x,
                    reach.y,
                ),
            }
        })
    }

    /// The horizontal knockback sign for a target hit by this swing. A forward swing pushes the
    /// way it points. Enemies only move along the floor, so an up or down swing pushes them
    /// sideways, away from the player (the facing side when exactly level with the player).
    pub fn knockback_direction(&self, player_x: f32, target_x: f32) -> f32 {
        let offset = target_x - player_x;
        match self.strike {
            StrikeDirection::Forward => self.direction,
            _ if offset.is_finite() && offset != 0.0 => offset.signum(),
            _ => self.direction,
        }
    }

    /// True once per downward swing, while it is active; the caller applies the bounce.
    pub fn take_pogo(&mut self) -> bool {
        let ready =
            self.strike == StrikeDirection::Down && self.active_remaining > 0.0 && !self.pogo_spent;
        self.pogo_spent |= ready;
        ready
    }

    pub fn cancel(&mut self) {
        self.active_remaining = 0.0;
        self.cooldown_remaining = 0.0;
    }
}
