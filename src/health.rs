/// Pure player health and simulation-time damage immunity.
#[derive(Clone, Debug, PartialEq)]
pub struct Health {
    current: u16,
    max: u16,
    invincibility_seconds: f32,
    invincibility_remaining: f32,
}

/// Health and damage are whole numbers where one standard hit is `HIT_DAMAGE`, so values
/// can be finer than a hit (the player starts with five hits of health).
pub const HIT_DAMAGE: u16 = 10;
/// The shipped maximum health and invincibility time. The game reads `player.max_health` and
/// `player.invincibility_seconds` from `data/player.ron` (a test keeps these equal to that file);
/// `Health::default()` uses these.
pub const MAX_HEALTH: u16 = 50;
pub const INVINCIBILITY_SECONDS: f32 = 1.0;

impl Default for Health {
    fn default() -> Self {
        Self::new(MAX_HEALTH, INVINCIBILITY_SECONDS)
    }
}

impl Health {
    /// Full health of `max`, with `invincibility_seconds` of immunity after each hit taken.
    pub fn new(max: u16, invincibility_seconds: f32) -> Self {
        Self {
            current: max,
            max,
            invincibility_seconds,
            invincibility_remaining: 0.0,
        }
    }

    pub fn current(&self) -> u16 {
        self.current
    }

    pub fn max(&self) -> u16 {
        self.max
    }

    pub fn invincibility_seconds(&self) -> f32 {
        self.invincibility_seconds
    }

    /// Swaps in new limits mid-game. Raising the maximum raises current health by the same
    /// amount (a player at full health stays at full health); lowering it cuts current health
    /// down to the new maximum. A player with no health left is not brought back. The immunity
    /// already running keeps its remaining time.
    pub fn set_limits(&mut self, max: u16, invincibility_seconds: f32) {
        if max > self.max && self.current > 0 {
            self.current = self.current.saturating_add(max - self.max).min(max);
        }
        self.current = self.current.min(max);
        self.max = max;
        self.invincibility_seconds = invincibility_seconds;
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
        self.take_damage_amount(HIT_DAMAGE)
    }

    pub fn take_damage_amount(&mut self, amount: u16) -> bool {
        if amount == 0 || self.is_invincible() || self.current == 0 {
            return false;
        }
        self.current = self.current.saturating_sub(amount);
        self.invincibility_remaining = self.invincibility_seconds;
        true
    }

    /// Sets the current health, at most the maximum, and leaves the invincibility timer alone.
    /// Healing and sword regain raise health through this; damage never does.
    pub fn set_current(&mut self, current: u16) {
        self.current = current.min(self.max);
    }

    pub fn respawn(&mut self) {
        self.current = self.max;
        self.invincibility_remaining = self.invincibility_seconds;
    }
}
