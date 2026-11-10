use std::sync::Arc;

use hollow_steps::body::{
    BodyInstance, BodyPoint, BodyTemplate, BodyTemplates, BodyVisual, Follow,
};
use macroquad::math::Vec2;

fn template() -> BodyTemplate {
    ron::from_str(include_str!("fixtures/bodies/motion.ron")).unwrap()
}

fn instance(template: BodyTemplate) -> BodyInstance {
    let mut visual: BodyVisual = ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap();
    visual.template = "motion".into();
    BodyInstance::new(
        visual,
        &BodyTemplates::from([("motion".into(), Arc::new(template))]),
    )
    .unwrap()
}

fn still_body() -> BodyInstance {
    let mut template = template();
    template.idle.clear();
    instance(template)
}

fn near(actual: Vec2, expected: Vec2) {
    assert!(
        actual.distance(expected) < 0.00001,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn default_spring_converges_and_does_not_overshoot_at_the_capped_step() {
    let mut body = still_body();
    let start = body.points["root"];
    let target = start + Vec2::new(0.5, -0.25);
    body.set_target("root", target).unwrap();
    for _ in 0..120 {
        body.step(1.0 / 30.0);
        assert!(body.points["root"].x <= target.x);
        assert!(body.points["root"].y >= target.y);
    }
    near(body.points["root"], target);
    near(body.velocities["root"], Vec2::ZERO);
}

#[test]
fn legacy_templates_inherit_defaults_that_settle_without_overshoot_at_common_rates() {
    let template: BodyTemplate = ron::from_str(include_str!("fixtures/bodies/small.ron")).unwrap();
    assert_eq!(
        template.follow,
        Follow {
            stiffness: 120.0,
            damping: 22.0
        }
    );
    assert!(template.idle.is_empty());
    assert!(template.follow_overrides.is_empty());
    for rate in [30, 60, 120, 240] {
        let mut body = instance(template.clone());
        let target = body.points["root"] + Vec2::new(0.5, -0.25);
        body.set_target("root", target).unwrap();
        for _ in 0..5 * rate {
            let previous = body.points["root"];
            body.step(1.0 / rate as f32);
            assert!(body.points["root"].x >= previous.x && body.points["root"].x <= target.x);
            assert!(body.points["root"].y <= previous.y && body.points["root"].y >= target.y);
        }
        near(body.points["root"], target);
        near(body.velocities["root"], Vec2::ZERO);
    }
}

#[test]
fn semi_implicit_euler_uses_the_new_velocity_and_clamps_only_the_spring_delta() {
    let mut body = still_body();
    let start = body.points["root"];
    body.set_target("root", start + Vec2::X).unwrap();
    let mut capped = body.clone();
    body.step(9.0);
    capped.step(1.0 / 30.0);
    assert_eq!(body.points, capped.points);
    assert_eq!(body.velocities, capped.velocities);
    near(body.velocities["root"], Vec2::new(4.0, 0.0));
    near(body.points["root"], start + Vec2::new(4.0 / 30.0, 0.0));

    let mut idle = instance(template());
    idle.step(9.0);
    near(
        idle.targets["root"],
        Vec2::from(idle.template.points["root"].rest),
    );
}

#[test]
fn per_point_follow_override_can_overshoot_without_affecting_other_points() {
    let mut body = still_body();
    let target = body.points["tip"] + Vec2::X;
    body.set_target("tip", target).unwrap();
    let root = body.points["root"];
    let mut overshot = false;
    for _ in 0..240 {
        body.step(1.0 / 60.0);
        overshot |= body.points["tip"].x > target.x;
        assert_eq!(body.points["root"], root);
    }
    assert!(overshot);
    near(body.points["tip"], target);
}

#[test]
fn identical_step_sequences_are_deterministic_and_invalid_deltas_are_noops() {
    let mut a = instance(template());
    let mut b = a.clone();
    a.set_target("tip", Vec2::new(0.8, -0.2)).unwrap();
    b.set_target("tip", Vec2::new(0.8, -0.2)).unwrap();
    for dt in [0.0, 0.01, 0.02, 1.0, 1.0 / 60.0]
        .into_iter()
        .cycle()
        .take(250)
    {
        a.step(dt);
        b.step(dt);
        assert_eq!(a.points, b.points);
        assert_eq!(a.velocities, b.velocities);
        assert_eq!(a.targets, b.targets);
    }
    let before = format!("{a:?}");
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        a.step(dt);
        assert_eq!(format!("{a:?}"), before);
    }
}

#[test]
fn idle_sine_uses_known_times_phases_and_box_fraction_amplitudes() {
    let template = template();
    let idle = &template.idle[0];
    for (time, factor) in [(0.0, 1.0), (1.0, 0.0), (2.0, -1.0), (3.0, 0.0), (4.0, 1.0)] {
        near(idle.offset_at(time), Vec2::from(idle.amplitude) * factor);
        near(
            template.idle[1].offset_at(time),
            Vec2::new(-0.03 * factor, 0.0),
        );
    }
    let mut body = instance(template);
    body.step(2.0);
    near(body.targets["root"], Vec2::new(0.09, -0.22));
    near(body.targets["tip"], Vec2::new(0.43, 0.3));
}

#[test]
fn spawn_and_snap_use_current_targets_and_reset_velocity() {
    let mut body = instance(template());
    assert_eq!(body.points, body.targets);
    near(body.points["root"], Vec2::new(0.11, -0.18));
    near(body.points["tip"], Vec2::new(0.37, 0.3));
    body.set_target("root", Vec2::new(0.5, 0.6)).unwrap();
    body.step(0.01);
    assert_ne!(body.velocities["root"], Vec2::ZERO);
    body.snap_to_targets();
    assert_eq!(body.points, body.targets);
    assert!(body.velocities.values().all(|v| *v == Vec2::ZERO));
}

#[test]
fn clear_target_restores_rest_plus_current_idle_without_snapping() {
    let mut body = instance(template());
    body.set_target("root", Vec2::new(0.5, 0.6)).unwrap();
    body.step(2.0);
    near(body.targets["root"], Vec2::new(0.5, 0.6));
    let position = body.points["root"];
    body.clear_target("root").unwrap();
    assert_eq!(body.points["root"], position);
    near(body.targets["root"], Vec2::new(0.09, -0.22));
    body.step(1.0);
    near(body.targets["root"], Vec2::new(0.1, -0.2));
}

#[test]
fn drawing_both_facings_does_not_change_stored_motion_or_next_step() {
    let mut body = instance(template());
    body.step(0.02);
    let mut reference = body.clone();
    for part in &body.template.parts {
        let right = body
            .part_transform(part, Vec2::ZERO, Vec2::splat(100.0), false)
            .unwrap();
        let left = body
            .part_transform(part, Vec2::ZERO, Vec2::splat(100.0), true)
            .unwrap();
        near(left.center, Vec2::new(-right.center.x, right.center.y));
    }
    assert_eq!(format!("{body:?}"), format!("{reference:?}"));
    body.step(0.02);
    reference.step(0.02);
    assert_eq!(body.points, reference.points);
}

#[test]
fn reload_keeps_surviving_positions_velocities_targets_and_idle_phase() {
    let mut old = instance(template());
    old.set_target("root", Vec2::new(0.7, 0.8)).unwrap();
    old.set_target("tip", Vec2::ZERO).unwrap();
    old.step(0.25);
    let mut replacement = template();
    replacement.points.get_mut("root").unwrap().rest = (-0.1, -0.4);
    replacement.points.remove("tip");
    replacement.parts.clear();
    replacement.follow_overrides.clear();
    replacement.idle.retain(|i| i.point == "root");
    replacement
        .points
        .insert("new".into(), BodyPoint { rest: (0.3, 0.5) });
    let mut reloaded = instance(replacement);
    reloaded.preserve_motion_from(&old);
    assert_eq!(reloaded.points["root"], old.points["root"]);
    assert_eq!(reloaded.velocities["root"], old.velocities["root"]);
    assert_eq!(reloaded.targets["root"], old.targets["root"]);
    assert!(!reloaded.points.contains_key("tip"));
    near(reloaded.points["new"], Vec2::new(0.3, 0.5));
    reloaded.clear_target("root").unwrap();
    near(
        reloaded.targets["root"],
        Vec2::new(-0.1, -0.4) + old.template.idle[0].offset_at(0.25),
    );
}

#[test]
fn invalid_follow_idle_and_target_values_have_clear_errors() {
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut bad = template();
        bad.follow.stiffness = value;
        assert!(bad.validate().unwrap_err().contains("follow: stiffness"));
        let mut bad = template();
        bad.idle[0].period_secs = value;
        let error = bad.validate().unwrap_err();
        assert!(
            error.contains("root") && error.contains("period_secs"),
            "{error}"
        );
    }
    for value in [-1.0, f32::NAN, f32::INFINITY] {
        let mut bad = template();
        bad.follow.damping = value;
        assert!(bad.validate().unwrap_err().contains("damping"));
    }
    let mut valid = template();
    valid.follow.damping = 0.0;
    valid.validate().unwrap();
    let mut bad = template();
    bad.follow_overrides
        .insert("missing".into(), Follow::default());
    assert!(bad
        .validate()
        .unwrap_err()
        .contains("follow override: unknown point \"missing\""));
    for (value, field) in [
        (
            Follow {
                stiffness: 0.0,
                damping: 1.0,
            },
            "stiffness",
        ),
        (
            Follow {
                stiffness: 1.0,
                damping: -1.0,
            },
            "damping",
        ),
    ] {
        let mut bad = template();
        bad.follow_overrides.insert("tip".into(), value);
        let error = bad.validate().unwrap_err();
        assert!(error.contains("tip") && error.contains(field), "{error}");
    }
    let mut bad = template();
    bad.idle[0].point = "missing".into();
    assert!(bad
        .validate()
        .unwrap_err()
        .contains("idle: unknown point \"missing\""));
    for phase in [false, true] {
        let mut bad = template();
        if phase {
            bad.idle[0].phase_offset_secs = f32::INFINITY;
        } else {
            bad.idle[0].amplitude.0 = f32::NAN;
        }
        assert!(bad.validate().unwrap_err().contains("finite"));
    }
    let mut body = instance(template());
    let before = format!("{body:?}");
    assert!(body
        .set_target("missing", Vec2::ZERO)
        .unwrap_err()
        .contains("unknown point"));
    assert!(body
        .clear_target("missing")
        .unwrap_err()
        .contains("unknown point"));
    assert!(body
        .set_target("root", Vec2::splat(f32::NAN))
        .unwrap_err()
        .contains("finite"));
    assert_eq!(format!("{body:?}"), before);
}
