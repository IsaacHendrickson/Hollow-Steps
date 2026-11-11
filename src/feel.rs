//! Combat feel: hit-stop and screen shake, driven by `GameEvent`s.
//!
//! Pure state and rules. `GameState` owns a `Feel`, feeds it each tick's events and skips
//! enemy and boss updates while `hit_stop_active`; the front end offsets the camera by
//! `shake_offset`. Nothing here is random: the shake is a sine of time since it began, so
//! the same events always give the same offsets.

use std::path::{Path, PathBuf};

use macroquad::math::Vec2;
use serde::Deserialize;

use crate::events::GameEvent;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "feel.ron";
/// Longest hit-stop a config may ask for, in seconds.
pub const MAX_HIT_STOP_SECS: f32 = 0.5;
/// Largest shake amplitude a config may ask for, in pixels.
pub const MAX_SHAKE_AMPLITUDE: f32 = 24.0;
/// Longest shake a config may ask for, in seconds.
pub const MAX_SHAKE_SECS: f32 = 2.0;
/// Fastest shake oscillation a config may ask for. Higher would alias against the 120 Hz step.
pub const MAX_SHAKE_FREQUENCY_HZ: f32 = 60.0;

/// The y axis oscillates at this multiple of the x frequency, with a phase offset, so the
/// shake traces a loop instead of a diagonal line.
const Y_FREQUENCY_RATIO: f32 = 1.7;
const Y_PHASE: f32 = 1.3;

/// One shake: a peak offset in pixels that fades linearly to zero over `secs`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShakeBurst {
    pub amplitude: f32,
    pub secs: f32,
}

/// How long enemies and the boss freeze after each kind of hit.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HitStopConfig {
    /// The player's swing hit an enemy or the boss.
    pub enemy_hit_secs: f32,
    /// The player lost health.
    pub player_damaged_secs: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShakeConfig {
    pub enemy_hit: ShakeBurst,
    pub player_damaged: ShakeBurst,
    /// Oscillations per second.
    pub frequency_hz: f32,
}

/// Hit-stop and shake tunables, from the embedded `data/feel.ron`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FeelConfig {
    pub hit_stop: HitStopConfig,
    pub shake: ShakeConfig,
}

impl Default for FeelConfig {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/feel.ron")).expect("valid feel defaults")
    }
}

impl FeelConfig {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let config: Self = ron::from_str(text).map_err(|error| format!("feel: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `feel.ron` in `assets` if there is one (it replaces the defaults whole), otherwise
    /// the embedded `data/feel.ron`. A file that exists but cannot be read or is invalid is
    /// an error, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        let path: PathBuf = assets.join(FILE_NAME).components().collect();
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                Self::from_ron(&text).map_err(|error| format!("{}: {error}", path.display()))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("{}: {error}", path.display())),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let in_range = |name: &str, value: f32, min: f32, max: f32| {
            if value.is_finite() && (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(format!("feel {name} must be between {min} and {max}"))
            }
        };
        in_range(
            "hit_stop.enemy_hit_secs",
            self.hit_stop.enemy_hit_secs,
            0.0,
            MAX_HIT_STOP_SECS,
        )?;
        in_range(
            "hit_stop.player_damaged_secs",
            self.hit_stop.player_damaged_secs,
            0.0,
            MAX_HIT_STOP_SECS,
        )?;
        for (name, burst) in [
            ("shake.enemy_hit", self.shake.enemy_hit),
            ("shake.player_damaged", self.shake.player_damaged),
        ] {
            in_range(
                &format!("{name}.amplitude"),
                burst.amplitude,
                0.0,
                MAX_SHAKE_AMPLITUDE,
            )?;
            in_range(&format!("{name}.secs"), burst.secs, 0.0, MAX_SHAKE_SECS)?;
        }
        in_range(
            "shake.frequency_hz",
            self.shake.frequency_hz,
            1.0,
            MAX_SHAKE_FREQUENCY_HZ,
        )
    }
}

/// Running hit-stop and shake.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feel {
    config: FeelConfig,
    hit_stop_remaining: f32,
    shake: ShakeBurst,
    shake_remaining: f32,
    shake_elapsed: f32,
}

impl Default for Feel {
    fn default() -> Self {
        Self::new(FeelConfig::default())
    }
}

impl Feel {
    pub fn new(config: FeelConfig) -> Self {
        Self {
            config,
            hit_stop_remaining: 0.0,
            shake: ShakeBurst {
                amplitude: 0.0,
                secs: 0.0,
            },
            shake_remaining: 0.0,
            shake_elapsed: 0.0,
        }
    }

    pub fn config(&self) -> &FeelConfig {
        &self.config
    }

    /// Swaps in new tunables. A hit-stop or shake already running keeps going.
    pub fn set_config(&mut self, config: FeelConfig) {
        self.config = config;
    }

    /// Reacts to events raised since the last call. A hit never shortens a hit-stop already
    /// running and never adds to it; a weaker shake never replaces a stronger one.
    pub fn apply(&mut self, events: &[GameEvent]) {
        for event in events {
            let (stop_secs, burst) = match event {
                GameEvent::EnemyHit => (
                    self.config.hit_stop.enemy_hit_secs,
                    self.config.shake.enemy_hit,
                ),
                GameEvent::PlayerDamaged => (
                    self.config.hit_stop.player_damaged_secs,
                    self.config.shake.player_damaged,
                ),
                _ => continue,
            };
            self.hit_stop_remaining = self.hit_stop_remaining.max(stop_secs);
            self.start_shake(burst);
        }
    }

    fn start_shake(&mut self, burst: ShakeBurst) {
        if burst.amplitude > 0.0 && burst.secs > 0.0 && burst.amplitude >= self.shake_amplitude() {
            self.shake = burst;
            self.shake_remaining = burst.secs;
            self.shake_elapsed = 0.0;
        }
    }

    /// Ages hit-stop and shake by one simulation step. Zero, negative and non-finite steps
    /// change nothing.
    pub fn advance(&mut self, delta_seconds: f32) {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        self.hit_stop_remaining = (self.hit_stop_remaining - delta_seconds).max(0.0);
        if self.shake_remaining > 0.0 {
            self.shake_remaining = (self.shake_remaining - delta_seconds).max(0.0);
            self.shake_elapsed += delta_seconds;
        }
    }

    /// True while enemies and the boss should stand still.
    pub fn hit_stop_active(&self) -> bool {
        self.hit_stop_remaining > 0.0
    }

    pub fn hit_stop_remaining(&self) -> f32 {
        self.hit_stop_remaining
    }

    /// The shake's current peak in pixels: the burst's amplitude, fading linearly to zero.
    pub fn shake_amplitude(&self) -> f32 {
        if self.shake_remaining > 0.0 && self.shake.secs > 0.0 {
            self.shake.amplitude * (self.shake_remaining / self.shake.secs).min(1.0)
        } else {
            0.0
        }
    }

    /// How far to shift the camera now, in world pixels. Zero when nothing is shaking.
    pub fn shake_offset(&self) -> Vec2 {
        let amplitude = self.shake_amplitude();
        if amplitude <= 0.0 {
            return Vec2::ZERO;
        }
        let phase = std::f32::consts::TAU * self.config.shake.frequency_hz * self.shake_elapsed;
        Vec2::new(
            amplitude * phase.sin(),
            amplitude * (phase * Y_FREQUENCY_RATIO + Y_PHASE).sin(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Binary fractions, so timers count down exactly.
    const DT: f32 = 1.0 / 128.0;

    fn config() -> FeelConfig {
        FeelConfig {
            hit_stop: HitStopConfig {
                enemy_hit_secs: 8.0 * DT,
                player_damaged_secs: 16.0 * DT,
            },
            shake: ShakeConfig {
                enemy_hit: ShakeBurst {
                    amplitude: 2.0,
                    secs: 32.0 * DT,
                },
                player_damaged: ShakeBurst {
                    amplitude: 8.0,
                    secs: 64.0 * DT,
                },
                frequency_hz: 16.0,
            },
        }
    }

    fn frozen_ticks(feel: &mut Feel) -> usize {
        let mut ticks = 0;
        while feel.hit_stop_active() {
            feel.advance(DT);
            ticks += 1;
            assert!(ticks < 10_000, "hit-stop never ended");
        }
        ticks
    }

    #[test]
    fn shipped_defaults_validate() {
        let config = FeelConfig::default();
        assert!(config.validate().is_ok());
        assert!(config.hit_stop.enemy_hit_secs > 0.0);
        assert!(config.shake.player_damaged.amplitude > config.shake.enemy_hit.amplitude);
    }

    #[test]
    fn bad_values_and_unknown_fields_are_rejected() {
        let good = config();
        let mut broken = Vec::new();
        for value in [-0.1, f32::NAN, f32::INFINITY, MAX_HIT_STOP_SECS + 0.01] {
            let mut c = good;
            c.hit_stop.enemy_hit_secs = value;
            broken.push(c);
            let mut c = good;
            c.hit_stop.player_damaged_secs = value;
            broken.push(c);
        }
        for value in [-1.0, f32::NAN, MAX_SHAKE_AMPLITUDE + 1.0] {
            let mut c = good;
            c.shake.enemy_hit.amplitude = value;
            broken.push(c);
            let mut c = good;
            c.shake.player_damaged.amplitude = value;
            broken.push(c);
        }
        for value in [-1.0, f32::INFINITY, MAX_SHAKE_SECS + 0.1] {
            let mut c = good;
            c.shake.enemy_hit.secs = value;
            broken.push(c);
        }
        for value in [0.0, -3.0, f32::NAN, MAX_SHAKE_FREQUENCY_HZ + 1.0] {
            let mut c = good;
            c.shake.frequency_hz = value;
            broken.push(c);
        }
        for config in broken {
            assert!(config.validate().is_err(), "{config:?} should be refused");
        }
        let text = include_str!("../data/feel.ron").replace("frequency_hz", "frequency");
        assert!(FeelConfig::from_ron(&text).is_err());
        assert!(FeelConfig::from_ron("(hit_stop: (enemy_hit_secs: 0.1))").is_err());
    }

    #[test]
    fn nothing_happens_without_events() {
        let mut feel = Feel::new(config());
        feel.apply(&[]);
        feel.advance(DT);
        assert!(!feel.hit_stop_active());
        assert_eq!(feel.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn an_enemy_hit_freezes_for_its_configured_time_then_stops() {
        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::EnemyHit]);
        assert!(feel.hit_stop_active());
        assert_eq!(feel.hit_stop_remaining(), 8.0 * DT);
        assert_eq!(frozen_ticks(&mut feel), 8);
        assert!(!feel.hit_stop_active());
    }

    #[test]
    fn taking_damage_freezes_longer_than_landing_a_hit() {
        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::PlayerDamaged]);
        assert_eq!(frozen_ticks(&mut feel), 16);
    }

    #[test]
    fn repeated_hits_do_not_stack_or_shorten_a_running_hit_stop() {
        let mut feel = Feel::new(config());
        feel.apply(&[
            GameEvent::EnemyHit,
            GameEvent::EnemyHit,
            GameEvent::EnemyHit,
        ]);
        assert_eq!(feel.hit_stop_remaining(), 8.0 * DT, "max, not a sum");
        feel.advance(DT);
        feel.advance(DT);
        feel.apply(&[GameEvent::EnemyHit]);
        assert_eq!(
            feel.hit_stop_remaining(),
            8.0 * DT,
            "a fresh hit restarts it"
        );
        feel.apply(&[GameEvent::PlayerDamaged]);
        assert_eq!(feel.hit_stop_remaining(), 16.0 * DT);
        feel.apply(&[GameEvent::EnemyHit]);
        assert_eq!(
            feel.hit_stop_remaining(),
            16.0 * DT,
            "a shorter stop never cuts a longer one"
        );
    }

    #[test]
    fn other_events_do_nothing() {
        let mut feel = Feel::new(config());
        feel.apply(&[
            GameEvent::PlayerJumped,
            GameEvent::PlayerLanded,
            GameEvent::PlayerDashed,
            GameEvent::PlayerAttacked,
            GameEvent::EnemyDied,
            GameEvent::CoinCollected,
            GameEvent::ItemPickedUp,
            GameEvent::RoomEntered,
            GameEvent::BossDefeated,
        ]);
        assert!(!feel.hit_stop_active());
        assert_eq!(feel.shake_amplitude(), 0.0);
        assert_eq!(feel.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn shake_fades_linearly_and_ends_exactly_at_its_duration() {
        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::PlayerDamaged]);
        assert_eq!(feel.shake_amplitude(), 8.0);
        feel.advance(32.0 * DT);
        assert_eq!(feel.shake_amplitude(), 4.0);
        feel.advance(16.0 * DT);
        assert_eq!(feel.shake_amplitude(), 2.0);
        feel.advance(16.0 * DT);
        assert_eq!(feel.shake_amplitude(), 0.0);
        assert_eq!(feel.shake_offset(), Vec2::ZERO);
        feel.advance(10.0);
        assert_eq!(feel.shake_offset(), Vec2::ZERO);
    }

    #[test]
    fn shake_offset_stays_within_the_fading_amplitude_and_moves() {
        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::PlayerDamaged]);
        let mut moved = false;
        let mut peak_first_half: f32 = 0.0;
        let mut peak_second_half: f32 = 0.0;
        for tick in 0..64 {
            feel.advance(DT);
            let offset = feel.shake_offset();
            let bound = feel.shake_amplitude() + 1e-4;
            assert!(offset.x.abs() <= bound && offset.y.abs() <= bound);
            moved |= offset != Vec2::ZERO;
            let size = offset.length();
            if tick < 32 {
                peak_first_half = peak_first_half.max(size);
            } else {
                peak_second_half = peak_second_half.max(size);
            }
        }
        assert!(moved);
        assert!(peak_second_half < peak_first_half, "the shake decays");
    }

    #[test]
    fn the_same_events_always_give_the_same_shake() {
        let run = || {
            let mut feel = Feel::new(config());
            feel.apply(&[GameEvent::EnemyHit]);
            let mut offsets = Vec::new();
            for tick in 0..80 {
                if tick == 10 {
                    feel.apply(&[GameEvent::PlayerDamaged]);
                }
                feel.advance(DT);
                offsets.push(feel.shake_offset());
            }
            offsets
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn a_weaker_burst_does_not_replace_a_stronger_one_but_a_stronger_one_does() {
        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::PlayerDamaged]);
        feel.advance(4.0 * DT);
        let before = feel.shake_amplitude();
        feel.apply(&[GameEvent::EnemyHit]);
        assert_eq!(
            feel.shake_amplitude(),
            before,
            "weaker hit leaves the shake alone"
        );

        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::EnemyHit]);
        feel.advance(4.0 * DT);
        feel.apply(&[GameEvent::PlayerDamaged]);
        assert_eq!(
            feel.shake_amplitude(),
            8.0,
            "stronger damage restarts at its peak"
        );
    }

    #[test]
    fn zero_negative_and_non_finite_steps_change_nothing() {
        let mut feel = Feel::new(config());
        feel.apply(&[GameEvent::PlayerDamaged]);
        let before = feel;
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            feel.advance(dt);
            assert_eq!(feel, before);
        }
        assert!(feel.shake_offset().is_finite());
    }

    #[test]
    fn zero_length_bursts_and_stops_are_harmless() {
        let mut quiet = config();
        quiet.hit_stop.enemy_hit_secs = 0.0;
        quiet.shake.enemy_hit = ShakeBurst {
            amplitude: 0.0,
            secs: 0.0,
        };
        assert!(quiet.validate().is_ok());
        let mut feel = Feel::new(quiet);
        feel.apply(&[GameEvent::EnemyHit]);
        assert!(!feel.hit_stop_active());
        assert_eq!(feel.shake_offset(), Vec2::ZERO);
    }
}
