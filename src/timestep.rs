//! Fixed simulation step: converts variable frame times into whole simulation ticks.
//!
//! Pure rules only; the binary feeds in `get_frame_time()` and runs one
//! `GameState::update` per returned tick.

use crate::player::PlayerInput;

/// Simulation step length in seconds (120 Hz).
pub const FIXED_STEP_SECONDS: f32 = 1.0 / 120.0;
/// Most ticks run for one rendered frame. Eight ticks cover down to 15 fps; slower
/// frames drop the excess time instead of spiraling.
pub const MAX_STEPS_PER_FRAME: u32 = 8;

/// Accumulates frame time and reports how many fixed ticks to run.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedStep {
    step: f64,
    max_steps: u32,
    accumulator: f64,
}

impl Default for FixedStep {
    fn default() -> Self {
        Self::new(FIXED_STEP_SECONDS, MAX_STEPS_PER_FRAME)
    }
}

impl FixedStep {
    /// `step` must be finite and positive and `max_steps` at least one; invalid values
    /// fall back to the defaults.
    pub fn new(step: f32, max_steps: u32) -> Self {
        let step = if step.is_finite() && step > 0.0 {
            step
        } else {
            FIXED_STEP_SECONDS
        };
        Self {
            step: f64::from(step),
            max_steps: max_steps.max(1),
            accumulator: 0.0,
        }
    }

    /// Length of one tick, to pass to `GameState::update`.
    pub fn step_seconds(&self) -> f32 {
        self.step as f32
    }

    /// Adds a frame's time and returns the number of ticks to run now. Non-finite or
    /// negative frame times count as zero. When the cap is hit, the whole ticks that
    /// could not run are discarded and only the fraction of a tick is kept.
    pub fn advance(&mut self, frame_seconds: f32) -> u32 {
        if frame_seconds.is_finite() && frame_seconds > 0.0 {
            self.accumulator += f64::from(frame_seconds);
        }
        // Tolerance keeps exact multiples (e.g. 2 * step) from losing a tick to rounding.
        let mut steps = 0;
        while self.accumulator + self.step * 1e-6 >= self.step && steps < self.max_steps {
            self.accumulator -= self.step;
            steps += 1;
        }
        self.accumulator = self.accumulator.max(0.0);
        if self.accumulator >= self.step {
            // Cap hit: drop the whole ticks we could not run, keep the fraction.
            self.accumulator %= self.step;
        }
        steps
    }

    /// Fraction of a tick left in the accumulator, in `[0, 1)`, for later render
    /// interpolation.
    pub fn alpha(&self) -> f32 {
        ((self.accumulator / self.step) as f32).clamp(0.0, 0.999_999)
    }

    /// Drops any stored time, e.g. after the editor or a pause so no burst follows.
    pub fn reset(&mut self) {
        self.accumulator = 0.0;
    }
}

/// Holds edge-triggered input until a tick consumes it, so a frame that runs zero ticks
/// (frame faster than the step) does not lose a key press.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InputLatch {
    held: PlayerInput,
    jump: bool,
    attack: bool,
    pickup: bool,
    dash: bool,
}

impl InputLatch {
    /// Records one frame's input: held values replace the stored ones, edge flags are
    /// OR-ed into the pending set.
    pub fn record(&mut self, input: PlayerInput) {
        self.held = PlayerInput {
            move_axis: input.move_axis,
            jump_held: input.jump_held,
            ..PlayerInput::default()
        };
        self.jump |= input.jump_pressed;
        self.attack |= input.attack_pressed;
        self.pickup |= input.pickup_pressed;
        self.dash |= input.dash_pressed;
    }

    /// Input for the next tick: held values always, pending edges exactly once (the
    /// first call after they were recorded).
    pub fn take_tick(&mut self) -> PlayerInput {
        let input = PlayerInput {
            jump_pressed: self.jump,
            attack_pressed: self.attack,
            pickup_pressed: self.pickup,
            dash_pressed: self.dash,
            ..self.held
        };
        self.jump = false;
        self.attack = false;
        self.pickup = false;
        self.dash = false;
        input
    }

    /// Forgets everything, e.g. when the editor opens.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total_steps(frame_seconds: f32, frames: usize) -> u32 {
        let mut step = FixedStep::default();
        (0..frames).map(|_| step.advance(frame_seconds)).sum()
    }

    #[test]
    fn step_counts_match_elapsed_time_for_common_frame_rates() {
        for (fps, frames) in [(30.0_f32, 30), (60.0, 60), (144.0, 144), (240.0, 240)] {
            let steps = total_steps(1.0 / fps, frames);
            assert!(
                (119..=121).contains(&steps),
                "{fps} fps gave {steps} steps for one second"
            );
        }
    }

    #[test]
    fn per_frame_counts_are_whole_and_small_frames_may_run_zero() {
        let mut step = FixedStep::default();
        assert_eq!(step.advance(1.0 / 30.0), 4);
        let mut fast = FixedStep::default();
        assert_eq!(fast.advance(1.0 / 144.0), 0);
        assert_eq!(fast.advance(1.0 / 144.0), 1);
    }

    #[test]
    fn step_count_is_capped_and_excess_time_is_dropped() {
        let mut step = FixedStep::default();
        assert_eq!(step.advance(5.0), MAX_STEPS_PER_FRAME);
        assert!(step.alpha() < 1.0);
        // No catch-up burst on the next normal frame.
        assert_eq!(step.advance(1.0 / 60.0), 2);
    }

    #[test]
    fn bad_frame_times_and_settings_are_safe() {
        let mut step = FixedStep::default();
        for bad in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_eq!(step.advance(bad), 0);
        }
        assert_eq!(step.alpha(), 0.0);
        let fallback = FixedStep::new(f32::NAN, 0);
        assert_eq!(fallback.step_seconds(), FIXED_STEP_SECONDS);
        let mut capped_at_one = FixedStep::new(0.01, 0);
        assert_eq!(capped_at_one.advance(1.0), 1);
    }

    #[test]
    fn alpha_is_the_leftover_fraction_and_reset_clears_it() {
        let mut step = FixedStep::default();
        step.advance(FIXED_STEP_SECONDS * 1.5);
        assert!((step.alpha() - 0.5).abs() < 0.01);
        step.reset();
        assert_eq!(step.alpha(), 0.0);
    }

    #[test]
    fn edge_flags_reach_only_the_first_tick_and_held_input_every_tick() {
        let mut latch = InputLatch::default();
        latch.record(PlayerInput {
            move_axis: 1.0,
            jump_pressed: true,
            jump_held: true,
            attack_pressed: true,
            pickup_pressed: true,
            dash_pressed: true,
        });
        let first = latch.take_tick();
        assert!(
            first.jump_pressed
                && first.attack_pressed
                && first.pickup_pressed
                && first.dash_pressed
        );
        for _ in 0..3 {
            let later = latch.take_tick();
            assert!(
                !later.jump_pressed
                    && !later.attack_pressed
                    && !later.pickup_pressed
                    && !later.dash_pressed
            );
            assert_eq!(later.move_axis, 1.0);
            assert!(later.jump_held);
        }
    }

    #[test]
    fn edges_survive_frames_with_no_ticks_and_are_still_delivered_once() {
        let mut latch = InputLatch::default();
        latch.record(PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        });
        // A zero-tick frame passes; the next frame records plain held input.
        latch.record(PlayerInput::default());
        assert!(latch.take_tick().attack_pressed);
        assert!(!latch.take_tick().attack_pressed);
    }

    #[test]
    fn held_values_follow_the_latest_frame_and_clear_forgets_edges() {
        let mut latch = InputLatch::default();
        latch.record(PlayerInput {
            move_axis: 1.0,
            jump_held: true,
            jump_pressed: true,
            ..PlayerInput::default()
        });
        latch.record(PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        });
        latch.clear();
        let input = latch.take_tick();
        assert_eq!(input, PlayerInput::default());
        latch.record(PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        });
        let input = latch.take_tick();
        assert_eq!(input.move_axis, -1.0);
        assert!(!input.jump_held);
    }
}
