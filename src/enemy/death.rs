//! How long a defeated enemy lingers before it is removed.

/// Counts the seconds since an enemy's health ran out. Only a defeated enemy advances it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DeathTimer {
    secs: f32,
    elapsed: f32,
}

impl DeathTimer {
    /// A timer for a death that lasts `secs`. A negative or non-finite length counts as zero,
    /// which is over the moment it begins.
    pub fn new(secs: f32) -> Self {
        Self {
            secs: if secs.is_finite() { secs.max(0.0) } else { 0.0 },
            elapsed: 0.0,
        }
    }

    /// Adds `delta_seconds`; zero, negative and non-finite steps change nothing.
    pub fn advance(&mut self, delta_seconds: f32) {
        if delta_seconds.is_finite() && delta_seconds > 0.0 {
            self.elapsed = (self.elapsed + delta_seconds).min(self.secs);
        }
    }

    /// True once the whole death has played out.
    pub fn finished(&self) -> bool {
        self.elapsed >= self.secs
    }

    /// How far through the death we are: `0` at the start, `1` when finished.
    pub fn progress(&self) -> f32 {
        if self.secs > 0.0 {
            (self.elapsed / self.secs).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// Opacity for drawing: `1` at the start, fading linearly to `0` when finished.
    pub fn opacity(&self) -> f32 {
        1.0 - self.progress()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_fades_linearly_and_finishes_exactly_at_its_length() {
        let mut timer = DeathTimer::new(0.5);
        assert!(!timer.finished());
        assert_eq!(timer.opacity(), 1.0);
        timer.advance(0.25);
        assert_eq!(timer.progress(), 0.5);
        assert_eq!(timer.opacity(), 0.5);
        assert!(!timer.finished());
        timer.advance(0.25);
        assert!(timer.finished());
        assert_eq!(timer.opacity(), 0.0);
    }

    #[test]
    fn a_long_step_cannot_overshoot() {
        let mut timer = DeathTimer::new(0.5);
        timer.advance(100.0);
        assert!(timer.finished());
        assert_eq!(timer.progress(), 1.0);
        assert_eq!(timer.opacity(), 0.0);
    }

    #[test]
    fn a_zero_length_death_is_finished_at_once() {
        let timer = DeathTimer::new(0.0);
        assert!(timer.finished());
        assert_eq!(timer.progress(), 1.0);
        assert_eq!(timer.opacity(), 0.0);
        for secs in [-1.0, f32::NAN, f32::INFINITY] {
            assert!(DeathTimer::new(secs).finished(), "{secs}");
        }
    }

    #[test]
    fn zero_negative_and_non_finite_steps_change_nothing() {
        let mut timer = DeathTimer::new(1.0);
        timer.advance(0.5);
        let before = timer;
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            timer.advance(dt);
            assert_eq!(timer, before);
        }
    }
}
