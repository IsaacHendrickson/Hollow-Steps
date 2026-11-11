//! The drawn box of an enemy or boss, kept apart from its hitbox.
//!
//! A definition's `size` is the hitbox: collision, contact damage and attacks use it. An
//! optional `visual_size` is only the box the body (or the plain colored box) is drawn in.
//! With no `visual_size` the drawn box is the hitbox, so older files look the same. The two
//! boxes share a horizontal center and a bottom edge, so a taller picture grows upward and the
//! feet stay on the floor line the hitbox stands on.

use macroquad::math::Vec2;

/// The smallest side a visual box may have, in pixels.
pub const MIN_VISUAL_SIDE: f32 = 1.0;
/// The largest side a visual box may have, in pixels.
pub const MAX_VISUAL_SIDE: f32 = 2000.0;

/// `None` (draw in the hitbox) is valid; a separate size needs both sides finite and within
/// `MIN_VISUAL_SIDE..=MAX_VISUAL_SIDE`.
pub fn validate(visual: Option<(f32, f32)>) -> Result<(), String> {
    let Some((width, height)) = visual else {
        return Ok(());
    };
    for (label, side) in [("width", width), ("height", height)] {
        if !side.is_finite() || !(MIN_VISUAL_SIDE..=MAX_VISUAL_SIDE).contains(&side) {
            return Err(format!(
                "visual size {label} must be between {MIN_VISUAL_SIDE} and {MAX_VISUAL_SIDE}"
            ));
        }
    }
    Ok(())
}

/// The size of the drawn box: its own when set, otherwise the hitbox.
pub fn size(hitbox: Vec2, visual: Option<(f32, f32)>) -> Vec2 {
    visual.map_or(hitbox, Vec2::from)
}

/// The center of the drawn box for a hitbox centered at `hitbox_center`: the same horizontal
/// center, with the bottom edges level.
pub fn center(hitbox_center: Vec2, hitbox: Vec2, visual: Vec2) -> Vec2 {
    Vec2::new(
        hitbox_center.x,
        hitbox_center.y + (hitbox.y - visual.y) / 2.0,
    )
}

/// The value after the editor's "separate visual size" switch is set to `separate`. Turning it
/// off drops the size. Turning it on keeps a size already there, or starts from the hitbox so
/// nothing visibly jumps.
pub fn with_separate(
    visual: Option<(f32, f32)>,
    hitbox: (f32, f32),
    separate: bool,
) -> Option<(f32, f32)> {
    if !separate {
        return None;
    }
    visual.or(Some((into_range(hitbox.0), into_range(hitbox.1))))
}

fn into_range(side: f32) -> f32 {
    if side.is_finite() {
        side.clamp(MIN_VISUAL_SIDE, MAX_VISUAL_SIDE)
    } else {
        MIN_VISUAL_SIDE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_visual_size_is_valid_and_draws_in_the_hitbox() {
        assert_eq!(validate(None), Ok(()));
        let hitbox = Vec2::new(36.0, 40.0);
        assert_eq!(size(hitbox, None), hitbox);
        assert_eq!(
            center(Vec2::new(100.0, 200.0), hitbox, hitbox),
            Vec2::new(100.0, 200.0)
        );
    }

    #[test]
    fn a_visual_size_replaces_the_hitbox_size_for_drawing_only() {
        let hitbox = Vec2::new(36.0, 40.0);
        assert_eq!(size(hitbox, Some((80.0, 90.0))), Vec2::new(80.0, 90.0));
        assert_eq!(hitbox, Vec2::new(36.0, 40.0));
    }

    #[test]
    fn the_visual_keeps_the_hitbox_bottom_and_horizontal_center() {
        let hitbox = Vec2::new(40.0, 40.0);
        let hitbox_center = Vec2::new(100.0, 180.0);
        let bottom = hitbox_center.y + hitbox.y / 2.0;
        for visual in [
            Vec2::new(100.0, 120.0),
            Vec2::new(10.0, 10.0),
            Vec2::new(40.0, 41.0),
        ] {
            let at = center(hitbox_center, hitbox, visual);
            assert_eq!(at.x, hitbox_center.x);
            assert!((at.y + visual.y / 2.0 - bottom).abs() < 1e-4, "{visual:?}");
        }
        // A taller picture rises above the hitbox; a shorter one sits lower than its center.
        assert!(center(hitbox_center, hitbox, Vec2::new(40.0, 120.0)).y < hitbox_center.y);
        assert!(center(hitbox_center, hitbox, Vec2::new(40.0, 10.0)).y > hitbox_center.y);
    }

    #[test]
    fn sides_outside_the_range_or_not_finite_are_refused() {
        for ok in [
            (MIN_VISUAL_SIDE, MIN_VISUAL_SIDE),
            (48.0, 96.5),
            (MAX_VISUAL_SIDE, MAX_VISUAL_SIDE),
        ] {
            assert_eq!(validate(Some(ok)), Ok(()), "{ok:?}");
        }
        for bad in [
            (0.0, 10.0),
            (10.0, 0.5),
            (-4.0, 10.0),
            (10.0, MAX_VISUAL_SIDE + 0.01),
            (f32::NAN, 10.0),
            (10.0, f32::INFINITY),
        ] {
            let error = validate(Some(bad)).unwrap_err();
            assert!(error.contains("visual size"), "{bad:?}: {error}");
        }
        assert!(validate(Some((0.0, 10.0))).unwrap_err().contains("width"));
        assert!(validate(Some((10.0, 0.0))).unwrap_err().contains("height"));
    }

    #[test]
    fn the_separate_switch_starts_from_the_hitbox_and_keeps_an_existing_size() {
        assert_eq!(with_separate(None, (36.0, 40.0), false), None);
        assert_eq!(
            with_separate(Some((9.0, 9.0)), (36.0, 40.0), false),
            None,
            "switching off drops the size"
        );
        assert_eq!(with_separate(None, (36.0, 40.0), true), Some((36.0, 40.0)));
        assert_eq!(
            with_separate(Some((9.0, 9.0)), (36.0, 40.0), true),
            Some((9.0, 9.0))
        );
    }

    #[test]
    fn a_hitbox_that_cannot_be_a_visual_size_is_pulled_into_range() {
        assert_eq!(
            with_separate(None, (5000.0, 0.2), true),
            Some((MAX_VISUAL_SIDE, MIN_VISUAL_SIDE))
        );
        let started = with_separate(None, (f32::NAN, f32::INFINITY), true).unwrap();
        assert_eq!(validate(Some(started)), Ok(()));
    }
}
