//! A short message the HUD shows for a moment ("Locked: needs Key 3"). Pure state: `show` starts
//! it, `update` counts it down in fixed ticks, `text` is what to draw, if anything.

/// How long a notice stays after the last time it was shown.
pub const NOTICE_SECONDS: f32 = 2.0;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Notice {
    text: String,
    remaining: f32,
}

impl Notice {
    /// Shows `text` for `NOTICE_SECONDS`. Showing it again, while it is still up or not,
    /// starts the time over.
    pub fn show(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.remaining = NOTICE_SECONDS;
    }

    /// Counts the notice down by `delta_seconds`. A negative or non-finite step changes nothing.
    pub fn update(&mut self, delta_seconds: f32) {
        if delta_seconds.is_finite() && delta_seconds > 0.0 {
            self.remaining = (self.remaining - delta_seconds).max(0.0);
        }
    }

    /// What to draw: the text while there is time left, otherwise nothing.
    pub fn text(&self) -> Option<&str> {
        (self.remaining > 0.0).then_some(self.text.as_str())
    }

    pub fn clear(&mut self) {
        self.remaining = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_shows_until_a_notice_is_shown() {
        assert_eq!(Notice::default().text(), None);
    }

    #[test]
    fn a_notice_shows_for_about_two_seconds() {
        let mut notice = Notice::default();
        notice.show("Locked: needs Key 3");
        assert_eq!(notice.text(), Some("Locked: needs Key 3"));

        notice.update(NOTICE_SECONDS - 0.01);
        assert_eq!(notice.text(), Some("Locked: needs Key 3"));
        notice.update(0.02);
        assert_eq!(notice.text(), None);
    }

    #[test]
    fn showing_again_starts_the_time_over_and_can_change_the_text() {
        let mut notice = Notice::default();
        notice.show("one");
        notice.update(1.5);
        notice.show("two");
        notice.update(1.5);
        assert_eq!(notice.text(), Some("two"));
    }

    #[test]
    fn a_zero_negative_or_non_finite_step_changes_nothing() {
        let mut notice = Notice::default();
        notice.show("x");
        for step in [0.0, -5.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            notice.update(step);
            assert_eq!(notice.text(), Some("x"), "step {step}");
        }
    }

    #[test]
    fn a_very_large_step_ends_it_and_clear_hides_it() {
        let mut notice = Notice::default();
        notice.show("x");
        notice.update(1.0e9);
        assert_eq!(notice.text(), None);

        notice.show("y");
        notice.clear();
        assert_eq!(notice.text(), None);
    }
}
