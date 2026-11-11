use super::*;
use crate::vitals::VitalsConfig;
use std::path::PathBuf;

const VIEW: Vec2 = vec2(VIEW_WIDTH, VIEW_HEIGHT);

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} is not close to {b}");
}

fn values(health: f32, trailing: f32, pending: f32, mana: f32) -> BarValues {
    BarValues {
        health,
        max_health: 100.0,
        trailing_health: trailing,
        pending_heal: pending,
        mana,
        max_mana: 50.0,
    }
}

/// A 200 pixel health bar at (10, 20), 10 pixels high.
fn anchor() -> Rect {
    Rect::new(10.0, 20.0, 200.0, 10.0)
}

fn layout(values: BarValues) -> BarLayout {
    layout_bars(&HudConfig::default(), values, anchor())
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-hud-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

// ---- layout ---------------------------------------------------------------------------

#[test]
fn widths_are_proportional_to_health_trailing_health_and_mana() {
    let layout = layout(values(40.0, 70.0, 0.0, 10.0));
    close(layout.empty.w, 200.0);
    close(layout.health.w, 80.0); // 40 of 100
    close(layout.trailing.w, 140.0); // 70 of 100
    close(layout.mana_empty.w, 200.0); // as long as the health bar
    close(layout.mana.w, 40.0); // 10 of 50
    for rect in [layout.health, layout.trailing, layout.pending, layout.mana] {
        close(rect.h, if rect == layout.mana { 12.0 } else { 10.0 });
    }
}

#[test]
fn the_bars_keep_their_length_whatever_the_maximums_are() {
    let small = layout_bars(
        &HudConfig::default(),
        BarValues {
            max_health: 5.0,
            health: 5.0,
            trailing_health: 5.0,
            ..values(0.0, 0.0, 0.0, 0.0)
        },
        anchor(),
    );
    let large = layout_bars(
        &HudConfig::default(),
        BarValues {
            max_health: 5000.0,
            health: 5000.0,
            trailing_health: 5000.0,
            ..values(0.0, 0.0, 0.0, 0.0)
        },
        anchor(),
    );
    assert_eq!(small.health, large.health);
    close(small.health.w, 200.0);
}

#[test]
fn the_trailing_bar_is_never_narrower_than_health() {
    // Trailing health below health, which the vitals never produce but the layout must survive.
    let layout = layout(values(60.0, 20.0, 0.0, 0.0));
    close(layout.trailing.w, layout.health.w);
    // Equal values: the trailing bar is exactly as wide.
    let layout = self::layout(values(60.0, 60.0, 0.0, 0.0));
    close(layout.trailing.w, layout.health.w);
    // Above health it is wider.
    assert!(self::layout(values(60.0, 90.0, 0.0, 0.0)).trailing.w > 120.0);
}

#[test]
fn the_pending_segment_starts_where_health_ends() {
    let layout = layout(values(40.0, 40.0, 25.0, 0.0));
    close(layout.pending.x, layout.health.right());
    close(layout.pending.w, 50.0); // 25 of 100
    close(layout.pending.right(), anchor().x + 130.0); // health + pending = 65 of 100
    close(layout.pending.y, anchor().y);
    close(layout.pending.h, anchor().h);
}

#[test]
fn the_pending_segment_is_cut_off_at_the_end_of_the_bar() {
    let layout = layout(values(80.0, 80.0, 500.0, 0.0));
    close(layout.pending.x, layout.health.right());
    close(layout.pending.right(), anchor().right());
    // Health already full: nothing is left to show.
    let full = self::layout(values(100.0, 100.0, 30.0, 0.0));
    close(full.pending.w, 0.0);
    close(full.pending.x, anchor().right());
}

#[test]
fn nothing_pending_means_a_zero_width_segment() {
    for pending in [0.0, -5.0, f32::NAN] {
        let layout = layout(values(40.0, 40.0, pending, 0.0));
        close(layout.pending.w, 0.0);
        close(layout.pending.x, layout.health.right());
    }
}

#[test]
fn the_mana_bar_is_directly_under_the_health_bar() {
    let config = HudConfig::default();
    let layout = layout(values(100.0, 100.0, 0.0, 25.0));
    close(layout.mana_empty.x, layout.empty.x);
    close(layout.mana_empty.y, layout.empty.bottom() + config.gap);
    close(layout.mana_empty.h, config.mana_height);
    assert!(layout.mana_empty.y >= layout.empty.bottom());
    // Mana fills from the same left edge and top as its own empty bar.
    close(layout.mana.x, layout.mana_empty.x);
    close(layout.mana.y, layout.mana_empty.y);
    close(layout.mana.h, layout.mana_empty.h);
}

#[test]
fn bounds_cover_both_bars() {
    let layout = layout(values(100.0, 100.0, 0.0, 25.0));
    let bounds = layout.bounds();
    close(bounds.x, layout.empty.x);
    close(bounds.y, layout.empty.y);
    close(bounds.w, layout.empty.w);
    close(bounds.bottom(), layout.mana_empty.bottom());
}

#[test]
fn a_zero_or_bad_maximum_is_safe_and_shows_empty_bars() {
    for max in [0.0, -10.0, f32::NAN] {
        let layout = layout(BarValues {
            health: 50.0,
            max_health: max,
            trailing_health: 80.0,
            pending_heal: 10.0,
            mana: 20.0,
            max_mana: max,
        });
        for rect in [layout.health, layout.trailing, layout.pending, layout.mana] {
            assert_eq!(rect.w, 0.0, "max {max}");
            assert!(rect.x.is_finite() && rect.y.is_finite() && rect.h.is_finite());
        }
        // The empty bars are still there.
        close(layout.empty.w, 200.0);
        close(layout.mana_empty.w, 200.0);
    }
}

#[test]
fn values_beyond_the_maximum_or_below_zero_are_clamped() {
    let layout = layout(values(250.0, 300.0, 0.0, 999.0));
    close(layout.health.w, 200.0);
    close(layout.trailing.w, 200.0);
    close(layout.mana.w, 200.0);
    let layout = self::layout(values(-30.0, -10.0, 0.0, -1.0));
    close(layout.health.w, 0.0);
    close(layout.trailing.w, 0.0);
    close(layout.mana.w, 0.0);
}

#[test]
fn nan_and_infinite_values_never_make_a_rectangle_with_a_nan() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let layout = layout(BarValues {
            health: bad,
            max_health: 100.0,
            trailing_health: bad,
            pending_heal: bad,
            mana: bad,
            max_mana: 50.0,
        });
        for rect in [
            layout.empty,
            layout.trailing,
            layout.pending,
            layout.health,
            layout.mana_empty,
            layout.mana,
        ] {
            assert!(
                rect.x.is_finite()
                    && rect.y.is_finite()
                    && rect.w.is_finite()
                    && rect.h.is_finite(),
                "{bad} gave {rect:?}"
            );
            assert!(rect.w >= 0.0);
        }
    }
}

#[test]
fn a_health_offset_moves_the_health_layers_and_not_the_mana_bar() {
    let still = layout(values(40.0, 60.0, 10.0, 25.0));
    let moved = still.with_health_offset(vec2(3.0, -2.0));
    for (before, after) in [
        (still.empty, moved.empty),
        (still.trailing, moved.trailing),
        (still.pending, moved.pending),
        (still.health, moved.health),
    ] {
        close(after.x, before.x + 3.0);
        close(after.y, before.y - 2.0);
        close(after.w, before.w);
        close(after.h, before.h);
    }
    assert_eq!(moved.mana_empty, still.mana_empty);
    assert_eq!(moved.mana, still.mana);
}

#[test]
fn fraction_handles_edges() {
    close(fraction(50.0, 100.0), 0.5);
    close(fraction(150.0, 100.0), 1.0);
    close(fraction(-1.0, 100.0), 0.0);
    assert_eq!(fraction(1.0, 0.0), 0.0);
    assert_eq!(fraction(f32::NAN, 100.0), 0.0);
    assert_eq!(fraction(f32::INFINITY, f32::INFINITY), 0.0);
}

// ---- from the vitals ------------------------------------------------------------------

#[test]
fn the_bars_follow_real_vitals_damage_and_trail() {
    let mut vitals = VitalsState::new(VitalsConfig::default(), 100.0);
    vitals.damage(30.0);
    let layout = layout_bars(&HudConfig::default(), BarValues::from(&vitals), anchor());
    close(layout.health.w, 140.0); // 70 of 100
    close(layout.trailing.w, 200.0); // still where health was
    assert!(layout.trailing.w > layout.health.w);
    close(layout.mana.w, 0.0); // starts with no mana
}

// ---- the settings ---------------------------------------------------------------------

#[test]
fn the_shipped_defaults_validate() {
    HudConfig::default().validate().unwrap();
}

#[test]
fn the_health_bar_is_anchored_top_left_from_the_margin() {
    let config = HudConfig::default();
    let bar = config.health_anchor(VIEW);
    close(bar.x, config.margin.0);
    close(bar.y, config.margin.1);
    close(bar.w, config.bar_length);
    close(bar.h, config.health_height);
    // A different screen size does not move a top-left bar.
    assert_eq!(config.health_anchor(vec2(1280.0, 720.0)), bar);
}

fn error_for(edit: impl FnOnce(&mut HudConfig)) -> String {
    let mut config = HudConfig::default();
    edit(&mut config);
    config.validate().expect_err("should be invalid")
}

#[test]
fn bad_settings_are_errors_naming_the_field() {
    assert!(error_for(|c| c.bar_length = MIN_BAR_LENGTH - 1.0).contains("bar_length"));
    assert!(error_for(|c| c.bar_length = f32::NAN).contains("bar_length"));
    assert!(error_for(|c| c.health_height = 0.0).contains("health_height"));
    assert!(error_for(|c| c.mana_height = MAX_BAR_HEIGHT + 1.0).contains("mana_height"));
    assert!(error_for(|c| c.gap = -1.0).contains("gap"));
    assert!(error_for(|c| c.margin.0 = -1.0).contains("margin.x"));
    assert!(error_for(|c| c.margin.1 = f32::INFINITY).contains("margin.y"));
}

#[test]
fn the_bars_must_fit_on_the_screen() {
    assert!(error_for(|c| {
        c.margin.0 = VIEW_WIDTH - 10.0;
    })
    .contains("fit"));
    assert!(error_for(|c| {
        c.margin.1 = VIEW_HEIGHT - 10.0;
    })
    .contains("fit"));
}

#[test]
fn the_trailing_and_pending_colors_must_differ_from_the_health_color() {
    assert!(error_for(|c| c.colors.trailing = c.colors.health).contains("colors.trailing"));
    assert!(error_for(|c| c.colors.pending = c.colors.health).contains("colors.pending"));
}

#[test]
fn ron_with_an_unknown_field_or_a_missing_one_is_an_error() {
    let text = include_str!("../../data/hud.ron");
    let with_extra = text.replacen("gap:", "colour_blind: true,\n    gap:", 1);
    assert!(HudConfig::from_ron(&with_extra).is_err());
    let without_gap = text.replacen("gap: 6.0,", "", 1);
    assert!(HudConfig::from_ron(&without_gap)
        .unwrap_err()
        .contains("gap"));
}

#[test]
fn an_override_file_replaces_the_defaults_and_a_missing_one_means_defaults() {
    let dir = TempDir::new();
    assert_eq!(HudConfig::load(&dir.0).unwrap(), HudConfig::default());
    let text = include_str!("../../data/hud.ron").replacen("280.0", "300.0", 1);
    std::fs::write(dir.0.join(FILE_NAME), text).unwrap();
    close(HudConfig::load(&dir.0).unwrap().bar_length, 300.0);
}

#[test]
fn a_bad_override_file_is_an_error_naming_it_never_a_fallback() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(margin: (0.0, 0.0))").unwrap();
    let error = HudConfig::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
}
