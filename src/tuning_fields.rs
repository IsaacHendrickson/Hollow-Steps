//! The editable numbers of `player.ron`: one `FieldSpec` (label, unit, range) per number, shared by
//! validation and the F4 tuning window. Re-exported from `tuning`.

use crate::tuning::TuningConfig;
/// One editable number: where it lives, how it is shown, and the range it may take.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSpec {
    /// Dotted name as written in the file, e.g. `player.move_speed`.
    pub key: &'static str,
    pub label: &'static str,
    pub group: &'static str,
    /// Shown after the number, e.g. `px/s`; empty for plain numbers.
    pub unit: &'static str,
    pub min: f32,
    pub max: f32,
    /// How much one dragged point changes the value.
    pub step: f32,
    pub decimals: usize,
    /// Whole numbers only (health).
    pub integer: bool,
}

impl FieldSpec {
    /// `Ok` with the value (rounded for whole-number fields) when it is a number inside the range.
    pub fn check(&self, value: f32) -> Result<f32, String> {
        if !value.is_finite() {
            return Err(format!("{} must be a number", self.key));
        }
        if value < self.min || value > self.max {
            return Err(format!(
                "{} must be between {} and {}{}",
                self.key,
                self.min,
                self.max,
                if self.unit.is_empty() {
                    String::new()
                } else {
                    format!(" {}", self.unit)
                }
            ));
        }
        // The range applies to what was entered; whole-number fields are rounded afterwards.
        Ok(if self.integer { value.round() } else { value })
    }
}

/// Every number in `player.ron`, in the order the tuning window shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    MoveSpeed,
    JumpSpeed,
    Gravity,
    MaxFallSpeed,
    HeldJumpGravityMultiplier,
    CoyoteSeconds,
    JumpBufferSeconds,
    Width,
    Height,
    DashSpeed,
    DashDuration,
    DashCooldown,
    EnemyDefaultHealth,
    EnemyKnockbackSpeed,
    EnemyKnockbackSeconds,
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

const SPECS: [FieldSpec; 15] = [
    spec(
        "player.move_speed",
        "Move speed",
        "Movement",
        "px/s",
        1.0,
        1500.0,
        1.0,
        0,
    ),
    spec(
        "player.jump_speed",
        "Jump speed",
        "Jump",
        "px/s",
        1.0,
        2000.0,
        1.0,
        0,
    ),
    spec(
        "player.gravity",
        "Gravity",
        "Jump",
        "px/s²",
        1.0,
        8000.0,
        5.0,
        0,
    ),
    spec(
        "player.max_fall_speed",
        "Max fall speed",
        "Jump",
        "px/s",
        1.0,
        4000.0,
        5.0,
        0,
    ),
    spec(
        "player.held_jump_gravity_multiplier",
        "Gravity while jump held",
        "Jump",
        "x",
        0.0,
        1.0,
        0.01,
        2,
    ),
    spec(
        "player.coyote_seconds",
        "Coyote time",
        "Jump",
        "s",
        0.0,
        0.5,
        0.005,
        3,
    ),
    spec(
        "player.jump_buffer_seconds",
        "Jump buffer",
        "Jump",
        "s",
        0.0,
        0.5,
        0.005,
        3,
    ),
    spec("player.width", "Width", "Size", "px", 8.0, 128.0, 0.5, 1),
    spec("player.height", "Height", "Size", "px", 8.0, 192.0, 0.5, 1),
    spec(
        "player.dash_speed",
        "Dash speed",
        "Dash",
        "px/s",
        1.0,
        3000.0,
        5.0,
        0,
    ),
    spec(
        "player.dash_duration",
        "Dash duration",
        "Dash",
        "s",
        0.01,
        1.0,
        0.005,
        3,
    ),
    spec(
        "player.dash_cooldown",
        "Dash cooldown",
        "Dash",
        "s",
        0.0,
        3.0,
        0.005,
        3,
    ),
    FieldSpec {
        integer: true,
        ..spec(
            "enemy.default_health",
            "Default enemy health",
            "Enemies",
            "hits",
            1.0,
            99.0,
            0.1,
            0,
        )
    },
    spec(
        "enemy.knockback_speed",
        "Knockback speed",
        "Enemies",
        "px/s",
        0.0,
        2000.0,
        1.0,
        0,
    ),
    spec(
        "enemy.knockback_seconds",
        "Knockback time",
        "Enemies",
        "s",
        0.0,
        1.0,
        0.005,
        3,
    ),
];

impl Field {
    pub const ALL: [Field; 15] = [
        Field::MoveSpeed,
        Field::JumpSpeed,
        Field::Gravity,
        Field::MaxFallSpeed,
        Field::HeldJumpGravityMultiplier,
        Field::CoyoteSeconds,
        Field::JumpBufferSeconds,
        Field::Width,
        Field::Height,
        Field::DashSpeed,
        Field::DashDuration,
        Field::DashCooldown,
        Field::EnemyDefaultHealth,
        Field::EnemyKnockbackSpeed,
        Field::EnemyKnockbackSeconds,
    ];

    pub fn spec(self) -> &'static FieldSpec {
        &SPECS[self as usize]
    }

    pub fn get(self, config: &TuningConfig) -> f32 {
        let (player, enemy) = (&config.player, &config.enemy);
        match self {
            Field::MoveSpeed => player.move_speed,
            Field::JumpSpeed => player.jump_speed,
            Field::Gravity => player.gravity,
            Field::MaxFallSpeed => player.max_fall_speed,
            Field::HeldJumpGravityMultiplier => player.held_jump_gravity_multiplier,
            Field::CoyoteSeconds => player.coyote_seconds,
            Field::JumpBufferSeconds => player.jump_buffer_seconds,
            Field::Width => player.size.x,
            Field::Height => player.size.y,
            Field::DashSpeed => player.dash_speed,
            Field::DashDuration => player.dash_duration,
            Field::DashCooldown => player.dash_cooldown,
            Field::EnemyDefaultHealth => f32::from(enemy.default_health),
            Field::EnemyKnockbackSpeed => enemy.knockback_speed,
            Field::EnemyKnockbackSeconds => enemy.knockback_seconds,
        }
    }

    /// Writes the value without checking it; callers go through `TuningConfig::with_field`.
    pub(crate) fn set(self, config: &mut TuningConfig, value: f32) {
        let (player, enemy) = (&mut config.player, &mut config.enemy);
        match self {
            Field::MoveSpeed => player.move_speed = value,
            Field::JumpSpeed => player.jump_speed = value,
            Field::Gravity => player.gravity = value,
            Field::MaxFallSpeed => player.max_fall_speed = value,
            Field::HeldJumpGravityMultiplier => player.held_jump_gravity_multiplier = value,
            Field::CoyoteSeconds => player.coyote_seconds = value,
            Field::JumpBufferSeconds => player.jump_buffer_seconds = value,
            Field::Width => player.size.x = value,
            Field::Height => player.size.y = value,
            Field::DashSpeed => player.dash_speed = value,
            Field::DashDuration => player.dash_duration = value,
            Field::DashCooldown => player.dash_cooldown = value,
            Field::EnemyDefaultHealth => enemy.default_health = value as u8,
            Field::EnemyKnockbackSpeed => enemy.knockback_speed = value,
            Field::EnemyKnockbackSeconds => enemy.knockback_seconds = value,
        }
    }
}
