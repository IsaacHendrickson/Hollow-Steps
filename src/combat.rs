use macroquad::math::{Rect, Vec2};

pub const SWING_SECONDS: f32 = 0.12;
pub const ATTACK_COOLDOWN: f32 = 0.35;
pub const ATTACK_REACH: f32 = 38.0;
pub const ATTACK_HEIGHT: f32 = 32.0;

/// Simulation-only melee state. Each swing may damage each enemy once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attack {
    id: u64,
    active_remaining: f32,
    cooldown_remaining: f32,
    direction: f32,
}

impl Attack {
    pub fn update(&mut self, pressed: bool, facing: f32, dt: f32) {
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        self.active_remaining = (self.active_remaining - dt).max(0.0);
        self.cooldown_remaining = (self.cooldown_remaining - dt).max(0.0);
        if pressed && self.cooldown_remaining == 0.0 {
            self.id += 1;
            self.direction = facing;
            self.active_remaining = SWING_SECONDS;
            self.cooldown_remaining = ATTACK_COOLDOWN;
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn direction(&self) -> f32 {
        self.direction
    }

    pub fn hitbox(&self, position: Vec2, size: Vec2) -> Option<Rect> {
        (self.active_remaining > 0.0).then(|| {
            Rect::new(
                if self.direction > 0.0 {
                    position.x + size.x / 2.0
                } else {
                    position.x - size.x / 2.0 - ATTACK_REACH
                },
                position.y - ATTACK_HEIGHT / 2.0,
                ATTACK_REACH,
                ATTACK_HEIGHT,
            )
        })
    }

    pub fn cancel(&mut self) {
        self.active_remaining = 0.0;
        self.cooldown_remaining = 0.0;
    }
}
