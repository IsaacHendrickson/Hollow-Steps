use super::*;
use crate::particles::Span;

const DT: f32 = 1.0 / 120.0;

/// A 200 pixel health bar at (10, 20), 18 pixels high.
fn bar() -> Rect {
    Rect::new(10.0, 20.0, 200.0, 18.0)
}

/// Fixed numbers, so these tests do not move when the shipped tuning does.
fn shake() -> DamageShake {
    DamageShake {
        seconds: 0.4,
        frequency_hz: 10.0,
        amplitude_per_max_health: 50.0,
        min_amplitude: 2.0,
        max_amplitude: 10.0,
    }
}

fn hud() -> HudConfig {
    HudConfig {
        damage_shake: shake(),
        ..HudConfig::default()
    }
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} is not close to {b}");
}

/// A burst of `count` particles with no spread, so each one starts exactly at the burst point.
fn exact_preset(count: u32) -> crate::particles::Preset {
    let mut preset = ParticleConfig::default().hud_damage;
    preset.count = count;
    preset.spread = (0.0, 0.0);
    preset.offset = (0.0, 0.0);
    preset
}

fn particle_config(preset: crate::particles::Preset, max_particles: u32) -> ParticleConfig {
    ParticleConfig {
        max_particles,
        hud_damage: preset,
        ..ParticleConfig::default()
    }
}

fn damaged(lost: f32, remaining: f32) -> LocatedEvent {
    LocatedEvent {
        event: GameEvent::PlayerDamaged,
        at: Some(vec2(500.0, 400.0)),
        loss: Some(HealthLoss { lost, remaining }),
    }
}

fn feedback(count: u32) -> HudFeedback {
    HudFeedback::new(particle_config(exact_preset(count), 512))
}

// ---- amplitude ------------------------------------------------------------------------

#[test]
fn amplitude_scales_with_the_share_of_max_health_lost() {
    let shake = shake();
    close(BarShake::amplitude_for(&shake, 10.0, 100.0), 5.0); // 0.10 * 50
    close(BarShake::amplitude_for(&shake, 14.0, 100.0), 7.0); // 0.14 * 50
                                                              // The same loss is a bigger share of a smaller maximum.
    close(BarShake::amplitude_for(&shake, 7.0, 50.0), 7.0);
    assert!(
        BarShake::amplitude_for(&shake, 14.0, 100.0) > BarShake::amplitude_for(&shake, 10.0, 100.0)
    );
}

#[test]
fn amplitude_is_clamped_between_the_minimum_and_the_maximum() {
    let shake = shake();
    // A scratch: 1% would be half a pixel, lifted to the minimum.
    close(BarShake::amplitude_for(&shake, 1.0, 100.0), 2.0);
    // Half of all health would be 25 pixels, cut to the maximum.
    close(BarShake::amplitude_for(&shake, 50.0, 100.0), 10.0);
    // More than all of it still stops at the maximum.
    close(BarShake::amplitude_for(&shake, 5000.0, 100.0), 10.0);
}

#[test]
fn no_loss_or_no_maximum_means_no_amplitude() {
    let shake = shake();
    for lost in [0.0, -4.0, f32::NAN, f32::INFINITY] {
        assert_eq!(BarShake::amplitude_for(&shake, lost, 100.0), 0.0, "{lost}");
    }
    for max in [0.0, -100.0, f32::NAN] {
        assert_eq!(BarShake::amplitude_for(&shake, 10.0, max), 0.0, "{max}");
    }
}

// ---- the shake over time --------------------------------------------------------------

#[test]
fn the_shake_decays_to_zero_and_then_stops() {
    let shake = shake();
    let mut bar_shake = BarShake::default();
    bar_shake.start(&shake, 14.0, 100.0); // peak 7 pixels
    assert!(bar_shake.is_active());
    close(bar_shake.current_amplitude(&shake), 7.0);

    let mut previous = f32::MAX;
    let mut elapsed = 0.0;
    while bar_shake.is_active() {
        let amplitude = bar_shake.current_amplitude(&shake);
        assert!(amplitude <= previous, "amplitude grew at {elapsed}s");
        // Neither axis ever swings past the peak.
        let offset = bar_shake.offset(&shake);
        assert!(offset.x.abs() <= amplitude + 1e-4 && offset.y.abs() <= amplitude + 1e-4);
        previous = amplitude;
        bar_shake.update(&shake, DT);
        elapsed += DT;
        assert!(elapsed < 1.0, "the shake never ended");
    }
    // It ends once its time is up, not before and not long after.
    assert!(elapsed >= shake.seconds - DT && elapsed <= shake.seconds + DT);
    assert_eq!(bar_shake.offset(&shake), Vec2::ZERO);
    assert_eq!(bar_shake.current_amplitude(&shake), 0.0);
}

#[test]
fn the_amplitude_fades_linearly() {
    let shake = shake();
    let mut bar_shake = BarShake::default();
    bar_shake.start(&shake, 14.0, 100.0);
    bar_shake.update(&shake, shake.seconds / 2.0);
    close(bar_shake.current_amplitude(&shake), 3.5);
}

#[test]
fn the_offset_comes_from_time_and_is_the_same_every_run() {
    let shake = shake();
    let run = || {
        let mut bar_shake = BarShake::default();
        bar_shake.start(&shake, 14.0, 100.0);
        (0..20)
            .map(|_| {
                bar_shake.update(&shake, DT);
                bar_shake.offset(&shake)
            })
            .collect::<Vec<_>>()
    };
    let first = run();
    assert_eq!(first, run());
    // It really moves: not the same offset every step.
    assert!(first.windows(2).any(|pair| pair[0] != pair[1]));
}

#[test]
fn a_still_bar_has_no_offset() {
    assert_eq!(BarShake::default().offset(&shake()), Vec2::ZERO);
    assert!(!BarShake::default().is_active());
}

#[test]
fn a_zero_negative_or_bad_step_does_not_age_the_shake() {
    let shake = shake();
    let mut bar_shake = BarShake::default();
    bar_shake.start(&shake, 14.0, 100.0);
    let before = bar_shake;
    for step in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        bar_shake.update(&shake, step);
        assert_eq!(bar_shake, before, "step {step}");
    }
}

#[test]
fn a_huge_step_ends_the_shake_cleanly() {
    let shake = shake();
    let mut bar_shake = BarShake::default();
    bar_shake.start(&shake, 14.0, 100.0);
    bar_shake.update(&shake, 1000.0);
    assert!(!bar_shake.is_active());
    assert_eq!(bar_shake.offset(&shake), Vec2::ZERO);
}

#[test]
fn a_second_hit_restarts_the_shake_but_never_calms_a_stronger_one() {
    let shake = shake();
    let mut bar_shake = BarShake::default();
    bar_shake.start(&shake, 50.0, 100.0); // peak 10
    bar_shake.update(&shake, shake.seconds / 4.0); // 7.5 left
    bar_shake.start(&shake, 1.0, 100.0); // a scratch, peak 2
    close(bar_shake.current_amplitude(&shake), 7.5);
    // The restart gives it the full time again.
    bar_shake.update(&shake, shake.seconds / 2.0);
    close(bar_shake.current_amplitude(&shake), 3.75);
    // A harder hit than what is left takes over.
    bar_shake.start(&shake, 50.0, 100.0);
    close(bar_shake.current_amplitude(&shake), 10.0);
}

#[test]
fn a_hit_with_no_amplitude_changes_nothing() {
    let shake = shake();
    let mut bar_shake = BarShake::default();
    bar_shake.start(&shake, 0.0, 100.0);
    assert_eq!(bar_shake, BarShake::default());
    bar_shake.start(&shake, 14.0, 100.0);
    bar_shake.update(&shake, 0.1);
    let before = bar_shake;
    bar_shake.start(&shake, f32::NAN, 100.0);
    assert_eq!(bar_shake, before);
}

// ---- the lost segment -----------------------------------------------------------------

#[test]
fn the_lost_segment_runs_from_the_health_left_to_the_health_before() {
    // 30 lost, 50 left: from 50% to 80% of the bar.
    let segment = lost_segment(
        bar(),
        100.0,
        HealthLoss {
            lost: 30.0,
            remaining: 50.0,
        },
    )
    .unwrap();
    close(segment.x, bar().x + 100.0);
    close(segment.w, 60.0);
    close(segment.right(), bar().x + 160.0);
    close(segment.y, bar().y);
    close(segment.h, bar().h);
}

#[test]
fn a_hit_that_took_the_last_health_spans_from_the_left_end() {
    let segment = lost_segment(
        bar(),
        100.0,
        HealthLoss {
            lost: 40.0,
            remaining: 0.0,
        },
    )
    .unwrap();
    close(segment.x, bar().x);
    close(segment.w, 80.0);
}

#[test]
fn the_lost_segment_stays_on_the_bar() {
    let segment = lost_segment(
        bar(),
        100.0,
        HealthLoss {
            lost: 500.0,
            remaining: 90.0,
        },
    )
    .unwrap();
    close(segment.right(), bar().right());
    assert!(segment.x >= bar().x && segment.w >= 0.0);
}

#[test]
fn there_is_no_segment_when_nothing_was_lost_or_there_is_no_maximum() {
    let loss = |lost| HealthLoss {
        lost,
        remaining: 10.0,
    };
    for lost in [0.0, -3.0, f32::NAN, f32::INFINITY] {
        assert_eq!(lost_segment(bar(), 100.0, loss(lost)), None, "{lost}");
    }
    for max in [0.0, -5.0, f32::NAN] {
        assert_eq!(lost_segment(bar(), max, loss(10.0)), None, "{max}");
    }
}

#[test]
fn the_burst_starts_at_the_right_end_of_the_segment_halfway_up_the_bar() {
    let segment = Rect::new(110.0, 20.0, 60.0, 18.0);
    let point = burst_point(segment);
    close(point.x, 170.0);
    close(point.y, 29.0);
}

// ---- particles ------------------------------------------------------------------------

#[test]
fn particles_spawn_in_the_lost_segment_at_its_right_end() {
    let hud = hud();
    let mut feedback = feedback(8);
    // 30 lost, 50 left: the right end of the lost part is 80% along the bar.
    let spawned = feedback.handle(&hud, bar(), 100.0, &[damaged(30.0, 50.0)]);
    assert_eq!(spawned, 8);
    let end = vec2(bar().x + 160.0, bar().y + bar().h / 2.0);
    assert_eq!(feedback.particles().len(), 8);
    for particle in feedback.particles() {
        close(particle.position.x, end.x);
        close(particle.position.y, end.y);
    }
}

#[test]
fn with_spread_every_particle_starts_inside_the_preset_box_around_the_burst_point() {
    let hud = hud();
    let mut preset = exact_preset(32);
    preset.spread = (3.0, 6.0);
    preset.offset = (1.0, -1.0);
    let mut feedback = HudFeedback::new(particle_config(preset, 512));
    feedback.handle(&hud, bar(), 100.0, &[damaged(30.0, 50.0)]);
    let end = vec2(bar().x + 160.0 + 1.0, bar().y + bar().h / 2.0 - 1.0);
    assert_eq!(feedback.particles().len(), 32);
    for particle in feedback.particles() {
        assert!((particle.position.x - end.x).abs() <= 3.0 + 1e-3);
        assert!((particle.position.y - end.y).abs() <= 6.0 + 1e-3);
        // Inside the bar's height, so the sparks come off the bar, not off the screen.
        assert!(particle.position.y >= bar().y && particle.position.y <= bar().bottom());
    }
}

#[test]
fn the_particle_count_is_the_presets_and_the_cap_cuts_a_burst_short() {
    let hud = hud();
    let mut feedback = feedback(12);
    assert_eq!(
        feedback.handle(&hud, bar(), 100.0, &[damaged(10.0, 40.0)]),
        12
    );
    let mut capped = HudFeedback::new(particle_config(exact_preset(12), 5));
    assert_eq!(capped.handle(&hud, bar(), 100.0, &[damaged(10.0, 40.0)]), 5);
    assert_eq!(capped.particles().len(), 5);
}

#[test]
fn each_damage_event_in_a_frame_gets_its_own_burst() {
    let hud = hud();
    let mut feedback = feedback(4);
    let spawned = feedback.handle(
        &hud,
        bar(),
        100.0,
        &[damaged(10.0, 40.0), damaged(10.0, 30.0)],
    );
    assert_eq!(spawned, 8);
}

#[test]
fn the_same_seed_gives_the_same_particles_and_another_seed_does_not() {
    let hud = hud();
    let config = ParticleConfig::default();
    let run = |seed| {
        let mut feedback = HudFeedback::with_seed(config.clone(), seed);
        feedback.handle(&hud, bar(), 100.0, &[damaged(30.0, 50.0)]);
        for _ in 0..10 {
            feedback.update(&hud, DT);
        }
        feedback.particles().to_vec()
    };
    let first = run(7);
    assert!(!first.is_empty());
    assert_eq!(first, run(7));
    assert_ne!(first, run(8));
    // The default emitter is seeded on its own constant, apart from the world's.
    assert_ne!(HUD_SEED, crate::particles::FIXED_SEED);
}

#[test]
fn the_particles_fall_and_fade_out_with_the_steps() {
    let hud = hud();
    let mut feedback = HudFeedback::new(ParticleConfig::default());
    feedback.handle(&hud, bar(), 100.0, &[damaged(30.0, 50.0)]);
    assert!(!feedback.particles().is_empty());
    for _ in 0..(1.0 / DT) as usize {
        feedback.update(&hud, DT);
    }
    assert!(feedback.particles().is_empty());
}

// ---- nothing to show ------------------------------------------------------------------

#[test]
fn zero_damage_does_nothing() {
    let hud = hud();
    let mut feedback = feedback(8);
    let events = [
        damaged(0.0, 50.0),
        damaged(-5.0, 50.0),
        damaged(f32::NAN, 50.0),
        damaged(f32::INFINITY, 50.0),
    ];
    assert_eq!(feedback.handle(&hud, bar(), 100.0, &events), 0);
    assert!(feedback.particles().is_empty());
    assert!(!feedback.shake_active());
    assert_eq!(feedback.bar_offset(&hud), Vec2::ZERO);
}

#[test]
fn no_maximum_health_does_nothing() {
    let hud = hud();
    let mut feedback = feedback(8);
    for max in [0.0, -1.0, f32::NAN] {
        assert_eq!(feedback.handle(&hud, bar(), max, &[damaged(10.0, 5.0)]), 0);
    }
    assert!(!feedback.shake_active());
    assert!(feedback.particles().is_empty());
}

#[test]
fn a_damage_event_without_a_loss_and_every_other_event_do_nothing() {
    let hud = hud();
    let mut feedback = feedback(8);
    let no_loss = LocatedEvent {
        event: GameEvent::PlayerDamaged,
        at: Some(Vec2::ZERO),
        loss: None,
    };
    let not_damage = |event| LocatedEvent {
        event,
        at: Some(Vec2::ZERO),
        loss: Some(HealthLoss {
            lost: 10.0,
            remaining: 10.0,
        }),
    };
    let events = [
        no_loss,
        not_damage(GameEvent::EnemyHit),
        not_damage(GameEvent::EnemyDied),
        not_damage(GameEvent::RoomEntered),
    ];
    assert_eq!(feedback.handle(&hud, bar(), 100.0, &events), 0);
    assert!(feedback.particles().is_empty());
    assert!(!feedback.shake_active());
}

// ---- the shake of the bar through the feedback ----------------------------------------

#[test]
fn a_hit_shakes_the_bar_by_the_share_of_health_it_took_and_clear_stops_it() {
    let hud = hud();
    let mut feedback = feedback(4);
    feedback.handle(&hud, bar(), 100.0, &[damaged(50.0, 40.0)]); // half of all health
    assert!(feedback.shake_active());
    feedback.update(&hud, DT);
    let offset = feedback.bar_offset(&hud);
    assert!(offset != Vec2::ZERO);
    assert!(offset.x.abs() <= 10.0 && offset.y.abs() <= 10.0 * Y_SHARE + 1e-3);
    feedback.clear();
    assert!(!feedback.shake_active());
    assert!(feedback.particles().is_empty());
    assert_eq!(feedback.bar_offset(&hud), Vec2::ZERO);
}

#[test]
fn a_fatal_hit_still_places_its_burst_on_the_lost_segment() {
    // Died at the end of the tick: the vitals are refilled already, but the event remembers
    // that the hit left 0 and took 25.
    let hud = hud();
    let mut feedback = feedback(4);
    feedback.handle(&hud, bar(), 50.0, &[damaged(25.0, 0.0)]);
    let end = vec2(bar().x + 100.0, bar().y + bar().h / 2.0);
    assert_eq!(feedback.particles().len(), 4);
    for particle in feedback.particles() {
        close(particle.position.x, end.x);
        close(particle.position.y, end.y);
    }
}

// ---- a separate emitter ---------------------------------------------------------------

#[test]
fn the_world_emitter_never_spawns_the_hud_burst_and_the_hud_emitter_never_spawns_the_world_ones() {
    let config = ParticleConfig::default();
    let mut world = ParticleSystem::new(config.clone());
    let spawned = world.handle(GameEvent::PlayerDamaged, Some(vec2(500.0, 400.0)));
    assert_eq!(spawned, config.hit_spark.count as usize);
    assert!(config
        .presets()
        .iter()
        .all(|(name, _)| *name != "hud_damage"));

    let hud = hud();
    let mut feedback = HudFeedback::new(config.clone());
    let spawned = feedback.handle(&hud, bar(), 100.0, &[damaged(30.0, 50.0)]);
    assert_eq!(spawned, config.hud_damage.count as usize);
    // The world position of the event is not used: the HUD burst is on the bar.
    for particle in feedback.particles() {
        assert!(particle.position.x < bar().right() + 200.0);
        assert!(particle.position.y < bar().bottom() + 200.0);
    }
}

#[test]
fn a_position_that_is_not_finite_spawns_nothing() {
    let mut system = ParticleSystem::new(ParticleConfig::default());
    assert_eq!(system.spawn_hud_damage(vec2(f32::NAN, 1.0)), 0);
    assert_eq!(system.spawn_hud_damage(vec2(1.0, f32::INFINITY)), 0);
    assert!(system.is_empty());
    assert_eq!(system.spawn_hud_damage(vec2(5.0, 5.0)), 12);
}

#[test]
fn the_shipped_hud_burst_starts_inside_the_bar_height() {
    let config = ParticleConfig::default();
    let preset = &config.hud_damage;
    assert!(preset.on.is_empty());
    let hud = HudConfig::default();
    assert!(preset.spread.1 * 2.0 <= hud.health_height + 1e-3);
    // And the shipped ranges are real ones.
    let Span { min, max } = preset.lifetime_secs;
    assert!(min > 0.0 && max >= min);
}
