//! Player vitals: the trailing health bar, health regained by sword hits, mana from kills, and
//! healing.
//!
//! Pure state and rules. `GameState` owns a `VitalsState`, calls `update` once per tick, and
//! tells it about damage (`damage`), sword hits (`regain`) and sword kills (`gain_kill_mana`).
//! Nothing here reads input devices or time; every rule takes explicit values.
//!
//! Healing is a small state machine (`HealPhase`): hold the button on the ground (`Starting`),
//! mana turns into pending health (`Channeling`), letting go waits a moment (`Stopping`) and
//! then the pending health is added. Damage cancels it and loses the pending health.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::assets::read_override;
use crate::tuning::FieldSpec;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "vitals.ron";

/// What the player is doing with the heal button, in the order a heal runs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HealPhase {
    /// No heal.
    #[default]
    Idle,
    /// Button held on the ground, waiting out `heal_start_delay`. Letting go here is free.
    Starting,
    /// Mana is turning into pending health.
    Channeling,
    /// Button released, waiting out `heal_stop_delay` before the pending health is added.
    Stopping,
}

/// What the heal rules need from the player each tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HealInput {
    /// The heal button is down.
    pub held: bool,
    pub grounded: bool,
}

/// The numbers in `data/vitals.ron`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VitalsConfig {
    pub max_mana: f32,
    pub mana_start: f32,
    pub mana_per_kill: f32,
    pub regain_fraction: f32,
    pub trail_hold: f32,
    pub trail_drain: f32,
    pub heal_start_delay: f32,
    pub heal_stop_delay: f32,
    pub heal_mana_per_second: f32,
    pub health_per_mana: f32,
    pub heal_move_scale: f32,
}

impl Default for VitalsConfig {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/vitals.ron")).expect("valid vitals defaults")
    }
}

impl VitalsConfig {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let config: Self = ron::from_str(text).map_err(|error| format!("vitals: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `vitals.ron` in `assets` if there is one (it replaces the defaults whole), otherwise
    /// the embedded `data/vitals.ron`. A file that exists but cannot be read or is invalid is an
    /// error naming it, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Ok(Self::default()),
        }
    }

    /// The file text, in the pretty format the editors use. Invalid values are refused.
    pub fn to_ron(&self) -> Result<String, String> {
        self.validate()?;
        crate::definition_editor::serialize(self)
    }

    /// Every number inside its range, and the starting mana within the mana cap.
    pub fn validate(&self) -> Result<(), String> {
        for field in VitalsField::ALL {
            field.spec().check(field.get(self))?;
        }
        if self.mana_start > self.max_mana {
            return Err("vitals.mana_start must not be more than vitals.max_mana".into());
        }
        Ok(())
    }

    /// A copy with one number replaced, checked as a whole. Whole-number rounding does not apply
    /// (no vitals number is a whole number).
    pub fn with_field(&self, field: VitalsField, value: f32) -> Result<Self, String> {
        let value = field.spec().check(value)?;
        let mut next = *self;
        field.set(&mut next, value);
        next.validate()?;
        Ok(next)
    }
}

/// Writes `config` to `assets/vitals.ron`, staged in a temporary file so a failed write leaves
/// the previous file untouched. Invalid values are refused before anything is written.
pub fn save(assets: &Path, config: &VitalsConfig) -> Result<PathBuf, String> {
    let text = config.to_ron()?;
    let path = assets.join(FILE_NAME);
    crate::tuning::write_staged(&path, &text)?;
    Ok(path)
}

/// Every number in `vitals.ron`, in the order the tuning window shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VitalsField {
    MaxMana,
    ManaStart,
    ManaPerKill,
    RegainFraction,
    TrailHold,
    TrailDrain,
    HealStartDelay,
    HealStopDelay,
    HealManaPerSecond,
    HealthPerMana,
    HealMoveScale,
}

#[allow(clippy::too_many_arguments)]
const fn spec(
    key: &'static str,
    label: &'static str,
    group: &'static str,
    unit: &'static str,
    min: f32,
    max: f32,
    step: f32,
    decimals: usize,
) -> FieldSpec {
    FieldSpec {
        key,
        label,
        group,
        unit,
        min,
        max,
        step,
        decimals,
        integer: false,
    }
}

const SPECS: [FieldSpec; 11] = [
    spec(
        "vitals.max_mana",
        "Max mana",
        "Mana",
        "mana",
        1.0,
        10_000.0,
        1.0,
        0,
    ),
    spec(
        "vitals.mana_start",
        "Starting mana",
        "Mana",
        "mana",
        0.0,
        10_000.0,
        1.0,
        0,
    ),
    spec(
        "vitals.mana_per_kill",
        "Mana per kill",
        "Mana",
        "mana",
        0.0,
        10_000.0,
        1.0,
        0,
    ),
    spec(
        "vitals.regain_fraction",
        "Regain fraction",
        "Health trail",
        "",
        0.0,
        1.0,
        0.01,
        2,
    ),
    spec(
        "vitals.trail_hold",
        "Trail hold",
        "Health trail",
        "s",
        0.0,
        10.0,
        0.01,
        2,
    ),
    spec(
        "vitals.trail_drain",
        "Trail drain",
        "Health trail",
        "health/s",
        0.1,
        10_000.0,
        0.5,
        1,
    ),
    spec(
        "vitals.heal_start_delay",
        "Heal start delay",
        "Heal",
        "s",
        0.0,
        5.0,
        0.01,
        2,
    ),
    spec(
        "vitals.heal_stop_delay",
        "Heal stop delay",
        "Heal",
        "s",
        0.0,
        5.0,
        0.01,
        2,
    ),
    spec(
        "vitals.heal_mana_per_second",
        "Heal mana rate",
        "Heal",
        "mana/s",
        0.1,
        10_000.0,
        0.5,
        1,
    ),
    spec(
        "vitals.health_per_mana",
        "Health per mana",
        "Heal",
        "health/mana",
        0.01,
        100.0,
        0.01,
        2,
    ),
    spec(
        "vitals.heal_move_scale",
        "Move speed while healing",
        "Heal",
        "",
        0.0,
        1.0,
        0.01,
        2,
    ),
];

impl VitalsField {
    pub const ALL: [VitalsField; 11] = [
        VitalsField::MaxMana,
        VitalsField::ManaStart,
        VitalsField::ManaPerKill,
        VitalsField::RegainFraction,
        VitalsField::TrailHold,
        VitalsField::TrailDrain,
        VitalsField::HealStartDelay,
        VitalsField::HealStopDelay,
        VitalsField::HealManaPerSecond,
        VitalsField::HealthPerMana,
        VitalsField::HealMoveScale,
    ];

    pub fn spec(self) -> &'static FieldSpec {
        &SPECS[self as usize]
    }

    pub fn get(self, config: &VitalsConfig) -> f32 {
        match self {
            Self::MaxMana => config.max_mana,
            Self::ManaStart => config.mana_start,
            Self::ManaPerKill => config.mana_per_kill,
            Self::RegainFraction => config.regain_fraction,
            Self::TrailHold => config.trail_hold,
            Self::TrailDrain => config.trail_drain,
            Self::HealStartDelay => config.heal_start_delay,
            Self::HealStopDelay => config.heal_stop_delay,
            Self::HealManaPerSecond => config.heal_mana_per_second,
            Self::HealthPerMana => config.health_per_mana,
            Self::HealMoveScale => config.heal_move_scale,
        }
    }

    /// Writes the value without checking it; callers go through `VitalsConfig::with_field`.
    fn set(self, config: &mut VitalsConfig, value: f32) {
        match self {
            Self::MaxMana => config.max_mana = value,
            Self::ManaStart => config.mana_start = value,
            Self::ManaPerKill => config.mana_per_kill = value,
            Self::RegainFraction => config.regain_fraction = value,
            Self::TrailHold => config.trail_hold = value,
            Self::TrailDrain => config.trail_drain = value,
            Self::HealStartDelay => config.heal_start_delay = value,
            Self::HealStopDelay => config.heal_stop_delay = value,
            Self::HealManaPerSecond => config.heal_mana_per_second = value,
            Self::HealthPerMana => config.health_per_mana = value,
            Self::HealMoveScale => config.heal_move_scale = value,
        }
    }
}

/// Running vitals: health with its trailing bar, mana, and the heal in progress.
#[derive(Clone, Debug, PartialEq)]
pub struct VitalsState {
    config: VitalsConfig,
    health: f32,
    max_health: f32,
    /// Where health was before recent damage; never below `health`.
    trailing_health: f32,
    /// Seconds left before the trailing bar starts to drain.
    trail_hold: f32,
    mana: f32,
    max_mana: f32,
    /// Health bought with mana but not added yet.
    pending_heal: f32,
    heal_phase: HealPhase,
    /// Seconds left in the current `Starting` or `Stopping` phase.
    phase_left: f32,
    /// After a cancelled heal (or a respawn) the button must be let go before a new heal starts.
    needs_release: bool,
}

impl VitalsState {
    /// Full health, `mana_start` mana, no heal.
    pub fn new(config: VitalsConfig, max_health: f32) -> Self {
        Self {
            config,
            health: max_health,
            max_health,
            trailing_health: max_health,
            trail_hold: 0.0,
            mana: config.mana_start.min(config.max_mana),
            max_mana: config.max_mana,
            pending_heal: 0.0,
            heal_phase: HealPhase::Idle,
            phase_left: 0.0,
            needs_release: false,
        }
    }

    pub fn config(&self) -> &VitalsConfig {
        &self.config
    }

    /// Swaps in new tunables. Health, mana and a heal in progress keep their values; mana is cut
    /// to a smaller cap.
    pub fn set_config(&mut self, config: VitalsConfig) {
        self.config = config;
        self.max_mana = config.max_mana;
        self.mana = self.mana.min(self.max_mana);
    }

    pub fn health(&self) -> f32 {
        self.health
    }

    pub fn max_health(&self) -> f32 {
        self.max_health
    }

    pub fn trailing_health(&self) -> f32 {
        self.trailing_health
    }

    /// Seconds before the trailing bar starts draining (zero once it is draining or caught up).
    pub fn trail_hold_left(&self) -> f32 {
        self.trail_hold
    }

    pub fn mana(&self) -> f32 {
        self.mana
    }

    pub fn max_mana(&self) -> f32 {
        self.max_mana
    }

    pub fn pending_heal(&self) -> f32 {
        self.pending_heal
    }

    pub fn heal_phase(&self) -> HealPhase {
        self.heal_phase
    }

    /// Any part of a heal: waiting to start, channeling, or waiting to stop. The player is slowed
    /// and cannot jump, dash or attack meanwhile.
    pub fn heal_active(&self) -> bool {
        self.heal_phase != HealPhase::Idle
    }

    /// The button must be let go before the next heal can start.
    pub fn heal_locked(&self) -> bool {
        self.needs_release
    }

    /// Multiplier for the player's horizontal speed: `heal_move_scale` during a heal, else 1.
    pub fn move_scale(&self) -> f32 {
        if self.heal_active() {
            self.config.heal_move_scale
        } else {
            1.0
        }
    }

    /// Health drops at once; the trailing bar stays and its hold starts over. Any heal in
    /// progress is cancelled: the pending health is lost, mana is not refunded, and the button
    /// must be let go and pressed again. Amounts that are not positive numbers do nothing.
    pub fn damage(&mut self, amount: f32) {
        if !amount.is_finite() || amount <= 0.0 {
            return;
        }
        self.health = (self.health - amount).max(0.0);
        self.trail_hold = self.config.trail_hold;
        if self.heal_active() {
            self.heal_phase = HealPhase::Idle;
            self.phase_left = 0.0;
            self.pending_heal = 0.0;
            self.needs_release = true;
        }
    }

    /// A sword hit on an enemy or boss: health takes back `regain_fraction` of the gap to the
    /// trailing bar. The trailing bar does not move.
    pub fn regain(&mut self) {
        let gap = (self.trailing_health - self.health).max(0.0);
        self.health = (self.health + gap * self.config.regain_fraction).min(self.max_health);
    }

    /// A sword kill of a regular enemy (not a boss): `mana_per_kill` mana, up to the cap.
    pub fn gain_kill_mana(&mut self) {
        self.mana = (self.mana + self.config.mana_per_kill).min(self.max_mana);
    }

    /// Everything back to the starting values. The button must be let go before the next heal.
    pub fn respawn(&mut self) {
        *self = Self::new(self.config, self.max_health);
        self.needs_release = true;
    }

    /// Advances the trailing bar and the heal by one tick. A zero, negative or non-finite step
    /// lets no time pass (the button is still read).
    pub fn update(&mut self, delta_seconds: f32, input: HealInput) {
        let dt = if delta_seconds.is_finite() && delta_seconds > 0.0 {
            delta_seconds
        } else {
            0.0
        };
        self.update_trail(dt);
        self.update_heal(dt, input);
        self.trailing_health = self.trailing_health.max(self.health);
    }

    fn update_trail(&mut self, dt: f32) {
        let mut left = dt;
        let waited = left.min(self.trail_hold);
        self.trail_hold -= waited;
        left -= waited;
        if self.trail_hold <= 0.0 && left > 0.0 {
            self.trailing_health =
                (self.trailing_health - self.config.trail_drain * left).max(self.health);
        }
    }

    fn update_heal(&mut self, dt: f32, input: HealInput) {
        if !input.held {
            self.needs_release = false;
        }
        let mut left = dt;
        if self.heal_phase == HealPhase::Idle && input.held && input.grounded && !self.needs_release
        {
            self.heal_phase = HealPhase::Starting;
            self.phase_left = self.config.heal_start_delay;
        }
        if self.heal_phase == HealPhase::Starting {
            if input.held && input.grounded {
                let waited = left.min(self.phase_left);
                self.phase_left -= waited;
                left -= waited;
                if self.phase_left <= 0.0 {
                    self.heal_phase = HealPhase::Channeling;
                }
            } else {
                // Letting go (or leaving the ground) before it began costs nothing.
                self.heal_phase = HealPhase::Idle;
            }
        }
        if self.heal_phase == HealPhase::Channeling {
            if !input.grounded {
                self.finish_heal();
            } else if !input.held {
                self.heal_phase = HealPhase::Stopping;
                self.phase_left = self.config.heal_stop_delay;
            } else {
                self.convert_mana(left);
                left = 0.0;
            }
        }
        if self.heal_phase == HealPhase::Stopping {
            if input.grounded {
                let waited = left.min(self.phase_left);
                self.phase_left -= waited;
                if self.phase_left <= 0.0 {
                    self.finish_heal();
                }
            } else {
                self.finish_heal();
            }
        }
    }

    /// Mana into pending health, limited by the mana there is and by the room left under
    /// `max_health` once what is already pending is counted.
    fn convert_mana(&mut self, dt: f32) {
        let per_mana = self.config.health_per_mana;
        let room = ((self.max_health - self.health - self.pending_heal) / per_mana).max(0.0);
        let mana = (self.config.heal_mana_per_second * dt)
            .min(self.mana)
            .min(room);
        self.mana -= mana;
        self.pending_heal += mana * per_mana;
    }

    /// Adds the pending health (never past `max_health`) and ends the heal.
    fn finish_heal(&mut self) {
        self.health = (self.health + self.pending_heal).min(self.max_health);
        self.pending_heal = 0.0;
        self.heal_phase = HealPhase::Idle;
        self.phase_left = 0.0;
    }
}

#[cfg(test)]
mod tests;
