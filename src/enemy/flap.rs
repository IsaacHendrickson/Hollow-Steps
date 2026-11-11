//! Wing flapping for fliers: the two poses the wings alternate between, and how fast they beat.
//! The beat rate follows the enemy's horizontal speed; the clock is pure and deterministic.

use serde::{Deserialize, Serialize};

/// The most wing beats per second a definition may ask for.
pub const MAX_FLAP_RATE: f32 = 30.0;
/// The most extra beats per second a definition may add for each pixel/second of speed.
pub const MAX_FLAP_RATE_PER_SPEED: f32 = 0.5;

/// Which poses a flier's wings alternate between and how quickly. Both poses are looked up in
/// the pose catalog; the body must give a pose's `hand_l` / `hand_r` points somewhere to go.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Flap {
    /// The pose of the upstroke, shown first in every beat.
    pub up_pose: String,
    /// The pose of the downstroke.
    pub down_pose: String,
    /// Beats (an upstroke and a downstroke) per second while hovering on the spot.
    pub hover_rate: f32,
    /// Extra beats per second for each pixel per second of horizontal speed.
    pub rate_per_speed: f32,
    /// The fastest the wings may beat, in beats per second.
    pub max_rate: f32,
}

impl Flap {
    /// What the editor starts from when an author turns the wing flap on.
    pub fn default_flap() -> Self {
        Self {
            up_pose: "fly_up".into(),
            down_pose: "fly_down".into(),
            hover_rate: 3.0,
            rate_per_speed: 0.02,
            max_rate: 7.0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for (field, name) in [("up_pose", &self.up_pose), ("down_pose", &self.down_pose)] {
            crate::pose::validate_name(name).map_err(|e| format!("movement: flap {field}: {e}"))?;
        }
        if !self.hover_rate.is_finite() || self.hover_rate <= 0.0 || self.hover_rate > MAX_FLAP_RATE
        {
            return Err(format!(
                "movement: flap hover_rate must be above 0 and at most {MAX_FLAP_RATE}"
            ));
        }
        if !self.rate_per_speed.is_finite()
            || !(0.0..=MAX_FLAP_RATE_PER_SPEED).contains(&self.rate_per_speed)
        {
            return Err(format!(
                "movement: flap rate_per_speed must be between 0 and {MAX_FLAP_RATE_PER_SPEED}"
            ));
        }
        if !self.max_rate.is_finite()
            || self.max_rate < self.hover_rate
            || self.max_rate > MAX_FLAP_RATE
        {
            return Err(format!(
                "movement: flap max_rate must be between hover_rate and {MAX_FLAP_RATE}"
            ));
        }
        Ok(())
    }

    /// Beats per second at `horizontal_speed` pixels per second (either direction).
    pub fn rate(&self, horizontal_speed: f32) -> f32 {
        let speed = if horizontal_speed.is_finite() {
            horizontal_speed.abs()
        } else {
            0.0
        };
        (self.hover_rate + self.rate_per_speed * speed).min(self.max_rate)
    }
}

/// Where a flier's wings are in their beat. Each beat starts with the upstroke: the first half
/// of the beat shows the up pose, the second half the down pose.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FlapClock {
    beat: f64,
}

impl FlapClock {
    /// Advances by `dt` seconds at the rate for `horizontal_speed`. A zero, negative or
    /// non-finite `dt` changes nothing.
    pub fn advance(&mut self, flap: &Flap, horizontal_speed: f32, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.beat = (self.beat + f64::from(flap.rate(horizontal_speed)) * f64::from(dt)).fract();
    }

    pub fn wings_up(&self) -> bool {
        self.beat < 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flap() -> Flap {
        Flap {
            up_pose: "fly_up".into(),
            down_pose: "fly_down".into(),
            hover_rate: 2.0,
            rate_per_speed: 0.01,
            max_rate: 6.0,
        }
    }

    #[test]
    fn the_rate_grows_with_horizontal_speed_either_way_and_is_capped() {
        let flap = flap();
        assert_eq!(flap.rate(0.0), 2.0);
        assert!((flap.rate(100.0) - 3.0).abs() < 1e-6);
        assert!(
            (flap.rate(-100.0) - 3.0).abs() < 1e-6,
            "direction does not matter"
        );
        assert_eq!(flap.rate(1.0e6), 6.0, "capped");
        assert_eq!(flap.rate(f32::NAN), 2.0, "a bad speed counts as hovering");
        assert_eq!(flap.rate(f32::INFINITY), 2.0);
    }

    #[test]
    fn each_beat_starts_with_the_upstroke_then_the_downstroke() {
        let flap = flap();
        let mut clock = FlapClock::default();
        assert!(clock.wings_up());
        // At 2 beats per second a beat is 0.5 s: up for 0.25 s, then down for 0.25 s.
        clock.advance(&flap, 0.0, 0.2);
        assert!(clock.wings_up());
        clock.advance(&flap, 0.0, 0.1);
        assert!(!clock.wings_up(), "0.3 s in: downstroke");
        clock.advance(&flap, 0.0, 0.15);
        assert!(!clock.wings_up(), "0.45 s in: still the downstroke");
        clock.advance(&flap, 0.0, 0.1);
        assert!(clock.wings_up(), "0.55 s in: the next beat's upstroke");
    }

    #[test]
    fn moving_faster_beats_the_wings_faster() {
        let flap = flap();
        let count_up_strokes = |speed: f32| {
            let mut clock = FlapClock::default();
            let mut was_up = clock.wings_up();
            let mut strokes = 0;
            // 10.2 s: no speed below ends exactly on a beat boundary.
            for _ in 0..1224 {
                clock.advance(&flap, speed, 1.0 / 120.0);
                if clock.wings_up() && !was_up {
                    strokes += 1;
                }
                was_up = clock.wings_up();
            }
            strokes
        };
        let hovering = count_up_strokes(0.0);
        let cruising = count_up_strokes(200.0);
        let racing = count_up_strokes(5000.0);
        assert_eq!(hovering, 20, "2 beats/s for 10.2 s: 20 full beats");
        assert_eq!(cruising, 40, "(2 + 0.01 * 200) = 4 beats/s: 40 beats");
        assert_eq!(racing, 61, "capped at 6 beats/s: 61 beats");
    }

    #[test]
    fn a_bad_delta_does_not_move_the_wings() {
        let flap = flap();
        let mut clock = FlapClock::default();
        clock.advance(&flap, 50.0, 0.3);
        let before = clock;
        for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            clock.advance(&flap, 50.0, bad);
            assert_eq!(clock, before, "delta {bad}");
        }
    }

    #[test]
    fn a_huge_delta_wraps_instead_of_growing_without_bound() {
        let flap = flap();
        let mut clock = FlapClock::default();
        clock.advance(&flap, 0.0, 1.0e6);
        assert!(clock.beat >= 0.0 && clock.beat < 1.0);
    }

    #[test]
    fn validation_names_the_bad_field() {
        assert!(flap().validate().is_ok());
        assert!(Flap::default_flap().validate().is_ok());
        let cases: [(Flap, &str); 9] = [
            (
                Flap {
                    up_pose: String::new(),
                    ..flap()
                },
                "up_pose",
            ),
            (
                Flap {
                    up_pose: "Up Pose".into(),
                    ..flap()
                },
                "up_pose",
            ),
            (
                Flap {
                    down_pose: "bad name!".into(),
                    ..flap()
                },
                "down_pose",
            ),
            (
                Flap {
                    hover_rate: 0.0,
                    ..flap()
                },
                "hover_rate",
            ),
            (
                Flap {
                    hover_rate: f32::NAN,
                    ..flap()
                },
                "hover_rate",
            ),
            (
                Flap {
                    hover_rate: MAX_FLAP_RATE + 1.0,
                    ..flap()
                },
                "hover_rate",
            ),
            (
                Flap {
                    rate_per_speed: -0.1,
                    ..flap()
                },
                "rate_per_speed",
            ),
            (
                Flap {
                    max_rate: 1.0,
                    ..flap()
                },
                "max_rate",
            ),
            (
                Flap {
                    max_rate: MAX_FLAP_RATE + 1.0,
                    ..flap()
                },
                "max_rate",
            ),
        ];
        for (flap, field) in cases {
            let error = flap.validate().unwrap_err();
            assert!(error.contains(field), "{flap:?}: {error}");
        }
    }
}
