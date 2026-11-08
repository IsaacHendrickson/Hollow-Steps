/// Pure player health and simulation-time damage immunity.
#[derive(Clone, Debug, PartialEq)]
pub struct Health {
    current: u8,
    invincibility_remaining: f32,
}

pub const MAX_HEALTH: u8 = 5;
pub const INVINCIBILITY_SECONDS: f32 = 1.0;

impl Default for Health {
    fn default() -> Self {
        Self {
            current: MAX_HEALTH,
            invincibility_remaining: 0.0,
        }
    }
}

impl Health {
    pub fn current(&self) -> u8 {
        self.current
    }

    pub fn is_invincible(&self) -> bool {
        self.invincibility_remaining > 0.0
    }

    pub fn flash_on(&self) -> bool {
        self.is_invincible()
            && ((self.invincibility_remaining * 10.0).ceil() as u32).is_multiple_of(2)
    }

    pub fn update(&mut self, delta_seconds: f32) {
        if delta_seconds.is_finite() {
            self.invincibility_remaining =
                (self.invincibility_remaining - delta_seconds.max(0.0)).max(0.0);
        }
    }

    pub fn take_damage(&mut self) -> bool {
        self.take_damage_amount(1)
    }

    pub fn take_damage_amount(&mut self, amount: u8) -> bool {
        if amount == 0 || self.is_invincible() || self.current == 0 {
            return false;
        }
        self.current = self.current.saturating_sub(amount);
        self.invincibility_remaining = INVINCIBILITY_SECONDS;
        true
    }

    pub fn respawn(&mut self) {
        self.current = MAX_HEALTH;
        self.invincibility_remaining = INVINCIBILITY_SECONDS;
    }
}
