//! Player-feel and enemy-knockback tunables.
//!
//! The shipped values are the embedded `data/player.ron`. A `player.ron` in the assets
//! folder replaces them whole, so a missing or invalid override is reported instead of
//! being merged with the defaults. Every field has one range, shared by validation and the
//! F4 tuning window, so what the window allows is exactly what a file may contain.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::player::PlayerConfig;

/// Name of the override file inside the assets folder.
pub const FILE_NAME: &str = "player.ron";

const DEFAULTS: &str = include_str!("../data/player.ron");

/// Enemy values that are not part of an enemy definition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnemyTuning {
    /// Health of enemies spawned by code (boss summons, enemy spawns) rather than defined in a file.
    pub default_health: u8,
    /// Horizontal speed of the push an enemy gets when the player hits it.
    pub knockback_speed: f32,
    /// How long that push lasts.
    pub knockback_seconds: f32,
}

impl Default for EnemyTuning {
    fn default() -> Self {
        embedded().enemy
    }
}

/// Everything in `player.ron`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuningConfig {
    pub player: PlayerConfig,
    pub enemy: EnemyTuning,
}

impl Default for TuningConfig {
    fn default() -> Self {
        *embedded()
    }
}

/// The shipped defaults, parsed once. They are checked by the unit tests, so a bad edit to
/// `data/player.ron` fails the build's tests rather than a player's game.
fn embedded() -> &'static TuningConfig {
    static EMBEDDED: OnceLock<TuningConfig> = OnceLock::new();
    EMBEDDED.get_or_init(|| {
        TuningConfig::parse(DEFAULTS, "data/player.ron").expect("valid player defaults")
    })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TuningFile {
    player: PlayerFile,
    enemy: EnemyFile,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PlayerFile {
    move_speed: f32,
    jump_speed: f32,
    gravity: f32,
    max_fall_speed: f32,
    held_jump_gravity_multiplier: f32,
    coyote_seconds: f32,
    jump_buffer_seconds: f32,
    width: f32,
    height: f32,
    dash_speed: f32,
    dash_duration: f32,
    dash_cooldown: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnemyFile {
    default_health: u8,
    knockback_speed: f32,
    knockback_seconds: f32,
}

impl From<TuningFile> for TuningConfig {
    fn from(file: TuningFile) -> Self {
        let (player, enemy) = (file.player, file.enemy);
        Self {
            player: PlayerConfig {
                move_speed: player.move_speed,
                jump_speed: player.jump_speed,
                gravity: player.gravity,
                max_fall_speed: player.max_fall_speed,
                held_jump_gravity_multiplier: player.held_jump_gravity_multiplier,
                coyote_seconds: player.coyote_seconds,
                jump_buffer_seconds: player.jump_buffer_seconds,
                size: Vec2::new(player.width, player.height),
                dash_speed: player.dash_speed,
                dash_duration: player.dash_duration,
                dash_cooldown: player.dash_cooldown,
            },
            enemy: EnemyTuning {
                default_health: enemy.default_health,
                knockback_speed: enemy.knockback_speed,
                knockback_seconds: enemy.knockback_seconds,
            },
        }
    }
}

impl From<&TuningConfig> for TuningFile {
    fn from(config: &TuningConfig) -> Self {
        let (player, enemy) = (&config.player, &config.enemy);
        Self {
            player: PlayerFile {
                move_speed: player.move_speed,
                jump_speed: player.jump_speed,
                gravity: player.gravity,
                max_fall_speed: player.max_fall_speed,
                held_jump_gravity_multiplier: player.held_jump_gravity_multiplier,
                coyote_seconds: player.coyote_seconds,
                jump_buffer_seconds: player.jump_buffer_seconds,
                width: player.size.x,
                height: player.size.y,
                dash_speed: player.dash_speed,
                dash_duration: player.dash_duration,
                dash_cooldown: player.dash_cooldown,
            },
            enemy: EnemyFile {
                default_health: enemy.default_health,
                knockback_speed: enemy.knockback_speed,
                knockback_seconds: enemy.knockback_seconds,
            },
        }
    }
}

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
    fn set(self, config: &mut TuningConfig, value: f32) {
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

impl TuningConfig {
    /// Parses and validates one file's text. `source` names the file in errors.
    pub fn parse(text: &str, source: &str) -> Result<Self, String> {
        let file: TuningFile = ron::from_str(text).map_err(|error| format!("{source}: {error}"))?;
        let config = Self::from(file);
        config
            .validate()
            .map_err(|error| format!("{source}: {error}"))?;
        Ok(config)
    }

    /// The `player.ron` in `assets` if there is one, otherwise the embedded defaults. A
    /// file that exists but cannot be read or is invalid is an error, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        let path = assets.join(FILE_NAME);
        match std::fs::read_to_string(&path) {
            Ok(text) => Self::parse(&text, &path_text(&path)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path_text(&path))),
        }
    }

    /// Checks every field against its range.
    pub fn validate(&self) -> Result<(), String> {
        for field in Field::ALL {
            field.spec().check(field.get(self))?;
        }
        Ok(())
    }

    /// A copy with one field changed, or the reason the value is not allowed.
    pub fn with_field(&self, field: Field, value: f32) -> Result<Self, String> {
        let value = field.spec().check(value)?;
        let mut changed = *self;
        field.set(&mut changed, value);
        Ok(changed)
    }

    /// The file text, in the pretty format the editors use.
    pub fn to_ron(&self) -> Result<String, String> {
        self.validate()?;
        crate::definition_editor::serialize(&TuningFile::from(self))
    }
}

/// Writes `config` to `assets/player.ron`. The text is staged in a temporary file and moved
/// into place, as `definition_editor::write_files` does, so a failed write leaves the
/// previous file untouched. Invalid values are refused before anything is written.
pub fn save(assets: &Path, config: &TuningConfig) -> Result<PathBuf, String> {
    use std::io::Write;
    let text = config.to_ron()?;
    let path = assets.join(FILE_NAME);
    let temp = path.with_extension("ron.tmp");
    let error = |path: &Path, error: std::io::Error| format!("{}: {error}", path_text(path));
    let result = (|| {
        match std::fs::remove_file(&temp) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(error(&temp, e)),
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)
            .map_err(|e| error(&temp, e))?;
        file.write_all(text.as_bytes())
            .map_err(|e| error(&path, e))?;
        file.sync_all().map_err(|e| error(&path, e))?;
        std::fs::rename(&temp, &path).map_err(|e| error(&path, e))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result.map(|()| path)
}

fn path_text(path: &Path) -> String {
    let path: PathBuf = path.components().collect();
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::{ENEMY_MAX_HEALTH, KNOCKBACK_SECONDS, KNOCKBACK_SPEED};
    use crate::player::{DASH_COOLDOWN, DASH_DURATION, DASH_SPEED};

    #[test]
    fn shipped_defaults_parse_validate_and_keep_the_old_constants() {
        let config = TuningConfig::default();
        config.validate().unwrap();
        assert_eq!(config.player.dash_speed, DASH_SPEED);
        assert_eq!(config.player.dash_duration, DASH_DURATION);
        assert_eq!(config.player.dash_cooldown, DASH_COOLDOWN);
        assert_eq!(config.enemy.default_health, ENEMY_MAX_HEALTH);
        assert_eq!(config.enemy.knockback_speed, KNOCKBACK_SPEED);
        assert_eq!(config.enemy.knockback_seconds, KNOCKBACK_SECONDS);
        assert_eq!(config.player.move_speed, 220.0);
        assert_eq!(config.player.jump_speed, 480.0);
        assert_eq!(config.player.gravity, 1_400.0);
        assert_eq!(config.player.max_fall_speed, 900.0);
        assert_eq!(config.player.held_jump_gravity_multiplier, 0.5);
        assert_eq!(config.player.coyote_seconds, 0.1);
        assert_eq!(config.player.jump_buffer_seconds, 0.12);
        assert_eq!(config.player.size, Vec2::new(32.0, 48.0));
    }

    #[test]
    fn every_field_has_a_spec_in_order_and_the_shipped_value_inside_its_range() {
        let config = TuningConfig::default();
        for (index, field) in Field::ALL.into_iter().enumerate() {
            assert_eq!(field as usize, index);
            let spec = field.spec();
            assert!(spec.min < spec.max, "{}", spec.key);
            assert!(spec.step > 0.0, "{}", spec.key);
            assert_eq!(spec.check(field.get(&config)), Ok(field.get(&config)));
        }
    }

    #[test]
    fn each_field_rejects_values_outside_its_range_and_non_numbers() {
        let config = TuningConfig::default();
        for field in Field::ALL {
            let spec = field.spec();
            for bad in [
                spec.min - spec.step.max(0.5),
                spec.max + spec.step.max(0.5),
                f32::NAN,
                f32::INFINITY,
                f32::NEG_INFINITY,
            ] {
                let error = config.with_field(field, bad).unwrap_err();
                assert!(error.contains(spec.key), "{error}");
            }
            // The limits themselves are allowed.
            assert!(config.with_field(field, spec.min).is_ok(), "{}", spec.key);
            assert!(config.with_field(field, spec.max).is_ok(), "{}", spec.key);
        }
    }

    #[test]
    fn whole_number_fields_round_and_everything_else_is_kept_exactly() {
        let config = TuningConfig::default();
        let health = config.with_field(Field::EnemyDefaultHealth, 4.4).unwrap();
        assert_eq!(health.enemy.default_health, 4);
        let health = config.with_field(Field::EnemyDefaultHealth, 4.6).unwrap();
        assert_eq!(health.enemy.default_health, 5);
        let speed = config.with_field(Field::MoveSpeed, 301.25).unwrap();
        assert_eq!(speed.player.move_speed, 301.25);
        assert_eq!(speed.enemy, config.enemy);
    }

    #[test]
    fn a_changed_field_changes_only_that_field() {
        let base = TuningConfig::default();
        for field in Field::ALL {
            let spec = field.spec();
            let target = if field.get(&base) == spec.max {
                spec.min
            } else {
                spec.max
            };
            let changed = base.with_field(field, target).unwrap();
            for other in Field::ALL {
                if other == field {
                    assert_eq!(other.get(&changed), target);
                } else {
                    assert_eq!(other.get(&changed), other.get(&base), "{}", spec.key);
                }
            }
        }
    }

    #[test]
    fn parsing_is_strict() {
        // Not matching the newline: a Windows checkout has CRLF line endings in the data file.
        let missing = DEFAULTS.replace("gravity: 1400.0,", "");
        let error = TuningConfig::parse(&missing, "player.ron").unwrap_err();
        assert!(error.starts_with("player.ron: "), "{error}");
        assert!(error.contains("gravity"), "{error}");

        let unknown = DEFAULTS.replace("gravity: 1400.0,", "gravity: 1400.0, wind: 3.0,");
        let error = TuningConfig::parse(&unknown, "player.ron").unwrap_err();
        assert!(error.contains("wind"), "{error}");

        let out_of_range = DEFAULTS.replace("move_speed: 220.0", "move_speed: 0.0");
        let error = TuningConfig::parse(&out_of_range, "player.ron").unwrap_err();
        assert!(error.contains("player.move_speed"), "{error}");

        let overflowing = DEFAULTS.replace("default_health: 3", "default_health: 300");
        assert!(TuningConfig::parse(&overflowing, "player.ron").is_err());
        assert!(TuningConfig::parse("", "player.ron").is_err());
        assert!(TuningConfig::parse("(player: (), enemy: ())", "player.ron").is_err());
    }

    #[test]
    fn a_non_finite_value_in_a_file_is_refused() {
        let nan = DEFAULTS.replace("gravity: 1400.0", "gravity: NaN");
        assert!(TuningConfig::parse(&nan, "player.ron").is_err());
    }

    #[test]
    fn serializing_and_parsing_round_trips_every_field() {
        let mut config = TuningConfig::default();
        for field in Field::ALL {
            let spec = field.spec();
            let value = spec.min + (spec.max - spec.min) / 4.0;
            config = config.with_field(field, value).unwrap();
        }
        let text = config.to_ron().unwrap();
        assert!(!text.contains('\r'));
        assert_eq!(TuningConfig::parse(&text, "player.ron").unwrap(), config);
    }

    #[test]
    fn invalid_configs_cannot_be_serialized() {
        let mut config = TuningConfig::default();
        config.player.gravity = f32::NAN;
        assert!(config.to_ron().is_err());
        config.player.gravity = -1.0;
        assert!(config.to_ron().is_err());
    }
}
