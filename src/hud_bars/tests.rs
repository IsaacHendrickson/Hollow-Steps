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
        base_max_health: 100.0,
        base_max_mana: 50.0,
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
            base_max_health: 5.0,
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
            base_max_health: 5000.0,
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
            base_max_health: max,
            base_max_mana: max,
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
            base_max_health: 100.0,
            base_max_mana: 50.0,
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

// ---- charms: capacity added or locked --------------------------------------------------

/// The standard values with other maximums, against the bases 100 health and 50 mana.
fn capped(max_health: f32, max_mana: f32) -> BarValues {
    BarValues {
        max_health,
        max_mana,
        ..values(0.0, 0.0, 0.0, 0.0)
    }
}

/// Every rectangle of a layout with the bar it must stay inside.
fn all_rects(layout: &BarLayout) -> Vec<(Rect, Rect)> {
    let health = [
        layout.empty,
        layout.trailing,
        layout.pending,
        layout.health,
        layout.health_base,
        layout.health_extension,
        layout.health_locked,
        layout.health_edge,
    ];
    let mana = [
        layout.mana_empty,
        layout.mana,
        layout.mana_base,
        layout.mana_extension,
        layout.mana_locked,
        layout.mana_edge,
    ];
    let health = health.map(|rect| (rect, layout.empty));
    health
        .into_iter()
        .chain(mana.map(|rect| (rect, layout.mana_empty)))
        .collect()
}

#[test]
fn a_maximum_equal_to_the_base_is_laid_out_as_it_was_before_charms() {
    let a = anchor();
    let part = |bar: Rect, share: f32| Rect::new(bar.x, bar.y, bar.w * share, bar.h);
    for (health, trailing, pending, mana) in [
        (40.0, 70.0, 25.0, 10.0),
        (100.0, 100.0, 0.0, 50.0),
        (0.0, 0.0, 0.0, 0.0),
        (33.3, 77.7, 5.5, 12.3),
    ] {
        let layout = layout(values(health, trailing, pending, mana));
        let share = fraction(health, 100.0);
        let end = fraction(health + pending, 100.0).max(share);
        // The old rule, written out: bar length times a share of the maximum.
        assert_eq!(layout.health, part(a, share));
        assert_eq!(
            layout.trailing,
            part(a, fraction(trailing, 100.0).max(share))
        );
        assert_eq!(layout.mana, part(layout.mana_empty, fraction(mana, 50.0)));
        let pending = Rect::new(a.x + a.w * share, a.y, a.w * (end - share), a.h);
        assert_eq!(layout.pending, pending);
        // The base part is the whole bar: nothing added, nothing locked, no divider.
        assert_eq!(layout.health_base, a);
        assert_eq!(layout.mana_base, layout.mana_empty);
        for rect in [
            layout.health_extension,
            layout.health_locked,
            layout.health_edge,
        ] {
            assert_eq!(rect.w, 0.0);
        }
        for rect in [layout.mana_extension, layout.mana_locked, layout.mana_edge] {
            assert_eq!(rect.w, 0.0);
        }
    }
}

#[test]
fn a_maximum_one_and_a_half_times_the_base_adds_a_third_after_the_base_part() {
    let full = BarValues {
        health: 150.0,
        trailing_health: 150.0,
        ..capped(150.0, 50.0)
    };
    let layout = layout(full);
    close(layout.health_base.w, 200.0 * 2.0 / 3.0);
    close(layout.health_extension.w, 200.0 / 3.0);
    close(layout.health_extension.x, layout.health_base.right());
    close(layout.health_extension.right(), anchor().right());
    close(layout.empty.w, 200.0); // the total length is unchanged
    close(layout.health_locked.w, 0.0);
    // The divider line sits on the boundary and stays on the bar.
    let middle = layout.health_edge.x + layout.health_edge.w / 2.0;
    close(middle, layout.health_extension.x);
    assert!(layout.health_edge.w > 0.0 && layout.health_edge.right() <= anchor().right());
    // Full health fills the whole bar; the base amount stops at the divider.
    close(layout.health.w, 200.0);
    let at_base = self::layout(BarValues {
        health: 100.0,
        trailing_health: 100.0,
        ..capped(150.0, 50.0)
    });
    close(at_base.health.right(), at_base.health_extension.x);
    // The mana bar, with its maximum at its base, is untouched.
    assert_eq!(layout.mana_base, layout.mana_empty);
    close(layout.mana_extension.w, 0.0);
}

#[test]
fn a_maximum_half_of_the_base_locks_the_last_half_and_a_full_bar_fills_the_first() {
    let full = BarValues {
        health: 50.0,
        trailing_health: 50.0,
        ..capped(50.0, 50.0)
    };
    let layout = layout(full);
    close(layout.health_locked.w, 100.0);
    close(layout.health_locked.right(), anchor().right());
    close(layout.health_base.w, 200.0); // the base scale is unchanged
    close(layout.health.w, 100.0);
    close(layout.trailing.w, 100.0);
    assert!(layout.health.right() <= layout.health_locked.x + 1e-3);
    close(layout.health_extension.w, 0.0);
    close(layout.health_edge.w, 0.0);
    // Nothing fills the locked end, however much there is of it.
    let over = self::layout(BarValues {
        health: 100.0,
        trailing_health: 100.0,
        pending_heal: 80.0,
        ..capped(50.0, 50.0)
    });
    for rect in [over.health, over.trailing, over.pending] {
        assert!(rect.right() <= over.health_locked.x + 1e-3, "{rect:?}");
    }
}

#[test]
fn a_maximum_mana_of_zero_locks_the_whole_mana_bar() {
    let layout = layout(BarValues {
        mana: 20.0,
        ..capped(100.0, 0.0)
    });
    close(layout.mana_locked.x, layout.mana_empty.x);
    close(layout.mana_locked.w, layout.mana_empty.w);
    close(layout.mana.w, 0.0);
    close(layout.health_locked.w, 0.0);
}

#[test]
fn both_bars_follow_the_same_rule_each_against_its_own_base() {
    let more = layout(BarValues {
        mana: 75.0,
        ..capped(100.0, 75.0)
    });
    close(more.mana_base.w, 200.0 * 2.0 / 3.0);
    close(more.mana_extension.w, 200.0 / 3.0);
    close(more.mana.w, 200.0);
    assert!(more.mana_edge.w > 0.0);
    // Less mana but more health at once.
    let mixed = layout(BarValues {
        mana: 25.0,
        health: 150.0,
        ..capped(150.0, 25.0)
    });
    close(mixed.mana_locked.w, 100.0);
    close(mixed.mana.w, 100.0);
    close(mixed.health_extension.w, 200.0 / 3.0);
    close(mixed.health_locked.w, 0.0);
}

#[test]
fn trailing_and_pending_use_the_same_scale_as_health() {
    // Locked end: base 100, max 50. 20 health is 40 px, 40 trailing is 80 px, 10 pending is 20.
    let locked = layout(BarValues {
        health: 20.0,
        trailing_health: 40.0,
        pending_heal: 10.0,
        ..capped(50.0, 50.0)
    });
    close(locked.health.w, 40.0);
    close(locked.trailing.w, 80.0);
    close(locked.pending.x, anchor().x + 40.0);
    close(locked.pending.w, 20.0);
    // Added capacity: base 100, max 200, so a unit is 1 px.
    let added = layout(BarValues {
        health: 50.0,
        trailing_health: 100.0,
        pending_heal: 25.0,
        ..capped(200.0, 50.0)
    });
    close(added.health.w, 50.0);
    close(added.trailing.w, 100.0);
    close(added.pending.x, anchor().x + 50.0);
    close(added.pending.w, 25.0);
}

#[test]
fn nothing_is_ever_longer_than_the_bar() {
    for max in [0.0, 1.0, 50.0, 100.0, 150.0, 1e6] {
        for base in [0.0, 1.0, 100.0, 1e6] {
            for value in [0.0, 30.0, 100.0, 1e9] {
                let layout = layout(BarValues {
                    health: value,
                    max_health: max,
                    trailing_health: value * 2.0,
                    pending_heal: value,
                    mana: value,
                    max_mana: max,
                    base_max_health: base,
                    base_max_mana: base,
                });
                for (rect, bar) in all_rects(&layout) {
                    let case = format!("max {max} base {base} value {value}: {rect:?}");
                    assert!(rect.w >= 0.0 && rect.w <= bar.w + 1e-3, "{case}");
                    assert!(rect.x >= bar.x - 1e-3, "{case}");
                    assert!(rect.right() <= bar.right() + 1e-3, "{case}");
                }
            }
        }
    }
}

#[test]
fn zero_nan_and_negative_maximums_and_bases_are_safe() {
    let odd = [0.0, -5.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 100.0];
    for max in odd {
        for base in odd {
            let layout = layout(BarValues {
                health: 40.0,
                max_health: max,
                trailing_health: 60.0,
                pending_heal: 10.0,
                mana: 20.0,
                max_mana: max,
                base_max_health: base,
                base_max_mana: base,
            });
            for (rect, bar) in all_rects(&layout) {
                let case = format!("max {max} base {base}: {rect:?}");
                let finite = rect.x.is_finite() && rect.y.is_finite();
                assert!(finite && rect.w.is_finite() && rect.h.is_finite(), "{case}");
                assert!(
                    rect.w >= 0.0 && rect.right() <= bar.right() + 1e-3,
                    "{case}"
                );
            }
        }
    }
}

#[test]
fn the_fillable_part_is_cut_only_when_the_maximum_is_below_the_base() {
    let a = anchor();
    assert_eq!(fillable_bar(a, 100.0, 100.0), a);
    assert_eq!(fillable_bar(a, 150.0, 100.0), a);
    assert_eq!(fillable_bar(a, f32::NAN, f32::NAN), a);
    close(fillable_bar(a, 50.0, 100.0).w, 100.0);
    close(fillable_bar(a, 0.0, 100.0).w, 0.0);
    assert_eq!(fillable_bar(a, 50.0, 100.0).x, a.x);
}

#[test]
fn a_shake_moves_all_the_health_parts_and_none_of_the_mana_parts() {
    let still = layout(BarValues {
        health: 150.0,
        mana: 25.0,
        ..capped(150.0, 25.0)
    });
    let moved = still.with_health_offset(vec2(3.0, -2.0));
    for (before, after) in [
        (still.health_base, moved.health_base),
        (still.health_extension, moved.health_extension),
        (still.health_locked, moved.health_locked),
        (still.health_edge, moved.health_edge),
    ] {
        close(after.x, before.x + 3.0);
        close(after.y, before.y - 2.0);
    }
    assert_eq!(moved.mana_base, still.mana_base);
    assert_eq!(moved.mana_extension, still.mana_extension);
    assert_eq!(moved.mana_locked, still.mana_locked);
    assert_eq!(moved.mana_edge, still.mana_edge);
}

#[test]
fn bar_values_from_vitals_have_no_base_difference_until_one_is_given() {
    let vitals = VitalsState::new(VitalsConfig::default(), 100.0);
    let plain = BarValues::from(&vitals);
    assert_eq!(plain.base_max_health, plain.max_health);
    assert_eq!(plain.base_max_mana, plain.max_mana);
    let based = plain.with_base(80.0, 60.0);
    assert_eq!((based.base_max_health, based.base_max_mana), (80.0, 60.0));
    assert_eq!(based.max_health, plain.max_health);
}

// ---- the new colors -------------------------------------------------------------------

#[test]
fn a_hud_file_without_the_new_colors_still_loads_with_the_shipped_ones() {
    let text = include_str!("../../data/hud.ron");
    let old = text
        .lines()
        .filter(|line| {
            let line = line.trim_start();
            !line.starts_with("extension_edge:") && !line.starts_with("locked:")
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!old.contains("locked:") && !old.contains("extension_edge:"));
    // The defaults for the missing colors are the colors the shipped file lists.
    assert_eq!(HudConfig::from_ron(&old).unwrap(), HudConfig::default());
}

#[test]
fn the_new_colors_must_stand_out_from_the_empty_bars() {
    let locked = "colors.locked";
    assert!(error_for(|c| c.colors.locked = c.colors.empty).contains(locked));
    assert!(error_for(|c| c.colors.locked = c.colors.mana_empty).contains(locked));
    let edge = "colors.extension_edge";
    assert!(error_for(|c| c.colors.extension_edge = c.colors.empty).contains(edge));
    assert!(error_for(|c| c.colors.extension_edge = c.colors.mana_empty).contains(edge));
}
