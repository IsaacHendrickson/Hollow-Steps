//! How an enemy moves through its room: on the ground (the default, today's behavior) or
//! flying at a hover height. Pure data and altitude math; `Enemy` applies it each tick.

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use super::flap::Flap;
use crate::player::WorldBounds;

/// The highest hover height an entity definition may ask for, in pixels above the floor.
pub const MAX_HOVER_HEIGHT: f32 = 1000.0;
/// The largest bob an entity definition may ask for, in pixels either side of the hover line.
pub const MAX_BOB_AMPLITUDE: f32 = 200.0;
/// The fastest bob an entity definition may ask for, in full up-and-down cycles per second.
pub const MAX_BOB_SPEED: f32 = 20.0;

/// Whether an enemy walks the floor or flies. `Ground` is the default, so every existing entity
/// file keeps loading and behaving as before.
#[derive(Clone, Debug, Default, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Movement {
    /// Pinned to the floor by gravity; swept against terrain.
    #[default]
    Ground,
    /// Ignores gravity, floor snapping and terrain. Its altitude is set by the definition:
    /// `hover_height` pixels between the floor and the bottom of its box (at the middle of the
    /// bob), plus a sine bob of `bob_amplitude` pixels at `bob_speed` cycles per second. With a
    /// `flap`, the body's wings alternate between two poses, faster the faster it flies.
    Flying {
        hover_height: f32,
        bob_amplitude: f32,
        bob_speed: f32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        flap: Option<Flap>,
    },
}

impl Movement {
    pub fn is_ground(&self) -> bool {
        matches!(self, Self::Ground)
    }

    pub fn is_flying(&self) -> bool {
        matches!(self, Self::Flying { .. })
    }

    /// The flying defaults the editor starts from when an author switches a ground enemy to
    /// flying: hovering a little above the floor with a gentle bob.
    pub fn default_flying() -> Self {
        Self::Flying {
            hover_height: 48.0,
            bob_amplitude: 4.0,
            bob_speed: 0.5,
            flap: None,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let Self::Flying {
            hover_height,
            bob_amplitude,
            bob_speed,
            flap,
        } = self
        else {
            return Ok(());
        };
        let (hover_height, bob_amplitude, bob_speed) = (*hover_height, *bob_amplitude, *bob_speed);
        if !hover_height.is_finite() || !(0.0..=MAX_HOVER_HEIGHT).contains(&hover_height) {
            return Err(format!(
                "movement: hover_height must be between 0 and {MAX_HOVER_HEIGHT}"
            ));
        }
        if !bob_amplitude.is_finite() || !(0.0..=MAX_BOB_AMPLITUDE).contains(&bob_amplitude) {
            return Err(format!(
                "movement: bob_amplitude must be between 0 and {MAX_BOB_AMPLITUDE}"
            ));
        }
        if !bob_speed.is_finite() || bob_speed <= 0.0 || bob_speed > MAX_BOB_SPEED {
            return Err(format!(
                "movement: bob_speed must be above 0 and at most {MAX_BOB_SPEED}"
            ));
        }
        if let Some(flap) = flap {
            flap.validate()?;
        }
        Ok(())
    }

    /// The center y of a flying enemy's box `time_secs` into its flight, or `None` for a ground
    /// enemy. Deterministic (a sine of the elapsed time) and kept inside the room: never below
    /// the floor, never above the ceiling.
    pub fn altitude(&self, size: Vec2, bounds: WorldBounds, time_secs: f64) -> Option<f32> {
        let Self::Flying {
            hover_height,
            bob_amplitude,
            bob_speed,
            ..
        } = *self
        else {
            return None;
        };
        let half = f64::from(size.y) / 2.0;
        let lowest = f64::from(bounds.floor_y) - half;
        let highest = (f64::from(bounds.top) + half).min(lowest);
        let midline = lowest - f64::from(hover_height);
        let phase = std::f64::consts::TAU * f64::from(bob_speed) * time_secs;
        let bob = f64::from(bob_amplitude) * phase.sin();
        Some((midline + bob).clamp(highest, lowest) as f32)
    }

    /// The wing flap of a flier that has one.
    pub fn flap(&self) -> Option<&Flap> {
        match self {
            Self::Flying { flap, .. } => flap.as_ref(),
            Self::Ground => None,
        }
    }

    /// Keeps a flying enemy's box inside the room horizontally.
    pub fn clamp_x(x: f32, size: Vec2, bounds: WorldBounds) -> f32 {
        let half = size.x / 2.0;
        let lowest = bounds.left + half;
        let highest = (bounds.right - half).max(lowest);
        x.clamp(lowest, highest)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> WorldBounds {
        WorldBounds {
            left: 0.0,
            top: 0.0,
            right: 960.0,
            floor_y: 460.0,
        }
    }

    const SIZE: Vec2 = Vec2::new(32.0, 40.0);

    fn flying(hover_height: f32, bob_amplitude: f32, bob_speed: f32) -> Movement {
        Movement::Flying {
            hover_height,
            bob_amplitude,
            bob_speed,
            flap: None,
        }
    }

    #[test]
    fn ground_is_the_default_and_has_no_altitude() {
        assert_eq!(Movement::default(), Movement::Ground);
        assert!(Movement::Ground.is_ground());
        assert_eq!(Movement::Ground.altitude(SIZE, bounds(), 1.0), None);
        assert!(Movement::Ground.validate().is_ok());
    }

    #[test]
    fn altitude_is_the_hover_line_plus_a_sine_bob() {
        let movement = flying(60.0, 8.0, 0.5);
        // Floor 460, box 40 tall: resting center is 440; hovering 60 above that is 380.
        assert_eq!(movement.altitude(SIZE, bounds(), 0.0), Some(380.0));
        // A quarter cycle (0.5 cycles/s -> 0.5 s) is the bob's peak; half a cycle is back.
        let quarter = movement.altitude(SIZE, bounds(), 0.5).unwrap();
        assert!((quarter - 388.0).abs() < 1e-3, "{quarter}");
        let half = movement.altitude(SIZE, bounds(), 1.0).unwrap();
        assert!((half - 380.0).abs() < 1e-3, "{half}");
        let three_quarters = movement.altitude(SIZE, bounds(), 1.5).unwrap();
        assert!((three_quarters - 372.0).abs() < 1e-3, "{three_quarters}");
    }

    #[test]
    fn altitude_repeats_every_cycle_and_never_leaves_the_bob_band() {
        let movement = flying(60.0, 8.0, 2.0);
        for step in 0..400 {
            let t = f64::from(step) * 0.013;
            let y = movement.altitude(SIZE, bounds(), t).unwrap();
            assert!((372.0..=388.0).contains(&y), "t={t}: {y}");
            let later = movement.altitude(SIZE, bounds(), t + 0.5).unwrap();
            assert!((y - later).abs() < 1e-2, "one cycle is 0.5 s at 2 Hz");
        }
    }

    #[test]
    fn altitude_stays_between_the_ceiling_and_the_floor() {
        // Hover far above the ceiling: pinned to the ceiling.
        let high = flying(900.0, 50.0, 1.0);
        for step in 0..100 {
            let y = high
                .altitude(SIZE, bounds(), f64::from(step) * 0.05)
                .unwrap();
            assert!((20.0..=440.0).contains(&y), "{y}");
        }
        // Zero hover with a big bob dips into the floor: pinned to the floor line.
        let low = flying(0.0, 50.0, 1.0);
        for step in 0..100 {
            let y = low
                .altitude(SIZE, bounds(), f64::from(step) * 0.05)
                .unwrap();
            assert!((20.0..=440.0).contains(&y), "{y}");
        }
        // A room shorter than the enemy never produces an inverted clamp (no panic).
        let tiny = WorldBounds {
            top: 450.0,
            ..bounds()
        };
        assert!(flying(10.0, 5.0, 1.0)
            .altitude(SIZE, tiny, 0.3)
            .unwrap()
            .is_finite());
    }

    #[test]
    fn clamp_x_keeps_the_box_inside_the_room() {
        assert_eq!(Movement::clamp_x(-50.0, SIZE, bounds()), 16.0);
        assert_eq!(Movement::clamp_x(2000.0, SIZE, bounds()), 944.0);
        assert_eq!(Movement::clamp_x(300.0, SIZE, bounds()), 300.0);
    }

    #[test]
    fn validation_rejects_bad_values_naming_the_field() {
        assert!(flying(0.0, 0.0, 0.1).validate().is_ok());
        assert!(flying(MAX_HOVER_HEIGHT, MAX_BOB_AMPLITUDE, MAX_BOB_SPEED)
            .validate()
            .is_ok());
        let cases = [
            (flying(-1.0, 4.0, 1.0), "hover_height"),
            (flying(f32::NAN, 4.0, 1.0), "hover_height"),
            (flying(f32::INFINITY, 4.0, 1.0), "hover_height"),
            (flying(MAX_HOVER_HEIGHT + 1.0, 4.0, 1.0), "hover_height"),
            (flying(10.0, -0.5, 1.0), "bob_amplitude"),
            (flying(10.0, f32::NAN, 1.0), "bob_amplitude"),
            (flying(10.0, MAX_BOB_AMPLITUDE + 1.0, 1.0), "bob_amplitude"),
            (flying(10.0, 4.0, 0.0), "bob_speed"),
            (flying(10.0, 4.0, -1.0), "bob_speed"),
            (flying(10.0, 4.0, f32::NAN), "bob_speed"),
            (flying(10.0, 4.0, MAX_BOB_SPEED + 1.0), "bob_speed"),
        ];
        for (movement, field) in cases {
            let error = movement.validate().unwrap_err();
            assert!(error.contains(field), "{movement:?}: {error}");
        }
    }

    #[test]
    fn ron_round_trips_and_rejects_unknown_fields() {
        let movement = flying(56.0, 5.0, 0.7);
        let text = ron::to_string(&movement).unwrap();
        assert_eq!(ron::from_str::<Movement>(&text).unwrap(), movement);
        assert_eq!(
            ron::from_str::<Movement>("Ground").unwrap(),
            Movement::Ground
        );
        assert!(ron::from_str::<Movement>(
            "Flying(hover_height: 1.0, bob_amplitude: 1.0, bob_speed: 1.0, wings: 2)"
        )
        .is_err());
        assert!(ron::from_str::<Movement>("Flying(hover_height: 1.0)").is_err());
    }
}
