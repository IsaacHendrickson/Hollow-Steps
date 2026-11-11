use super::*;
use std::path::PathBuf;

/// One burst with fixed numbers, so every particle is the same unless a test changes it.
fn preset_text(on: &str, count: u32) -> String {
    format!(
        "(on: [{on}], count: {count}, lifetime_secs: (min: 0.5, max: 0.5), \
         speed: (min: 100.0, max: 100.0), direction_deg: (min: 0.0, max: 0.0), \
         spread: (0.0, 0.0), offset: (0.0, 0.0), gravity: 0.0, drag: 0.0, size: (10.0, 2.0), \
         color_start: (200, 100, 0, 255), color_end: (100, 0, 200, 0))"
    )
}

fn fixture_text(max: u32) -> String {
    format!(
        "(max_particles: {max}, hit_spark: {}, dust: {}, dash_trail: {}, death_burst: {})",
        preset_text("EnemyHit, PlayerDamaged", 4),
        preset_text("PlayerLanded", 3),
        preset_text("PlayerDashed", 2),
        preset_text("EnemyDied", 5),
    )
}

fn fixture(max: u32) -> ParticleConfig {
    ParticleConfig::from_ron(&fixture_text(max)).expect("fixture parses")
}

const AT: Vec2 = Vec2::new(300.0, 200.0);

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-particles-{}-{}",
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

// ---- shipped defaults -----------------------------------------------------------------

#[test]
fn shipped_defaults_validate_and_every_preset_is_wired_to_an_event() {
    let config = ParticleConfig::default();
    config.validate().unwrap();
    for (name, preset) in config.presets() {
        assert!(!preset.on.is_empty(), "{name} is triggered by no event");
    }
}

// ---- the generator --------------------------------------------------------------------

#[test]
fn the_generator_repeats_for_a_seed_and_differs_between_seeds() {
    let run = |seed| {
        let mut rng = Rng::new(seed);
        (0..8).map(|_| rng.next_u64()).collect::<Vec<_>>()
    };
    assert_eq!(run(7), run(7));
    assert_ne!(run(7), run(8));
    // A pinned value: if this changes, every recorded effect changes with it.
    assert_eq!(Rng::new(0).next_u64(), 0xE220_A839_7B1D_CDAF);
}

#[test]
fn unit_stays_below_one_and_between_stays_inside_its_span() {
    let mut rng = Rng::new(FIXED_SEED);
    let span = Span {
        min: -3.0,
        max: 5.0,
    };
    for _ in 0..10_000 {
        let unit = rng.unit();
        assert!((0.0..1.0).contains(&unit), "{unit}");
        let value = rng.between(span);
        assert!((-3.0..5.0).contains(&value), "{value}");
    }
    let flat = Span { min: 2.0, max: 2.0 };
    assert_eq!(rng.between(flat), 2.0);
}

// ---- spawning -------------------------------------------------------------------------

#[test]
fn an_event_spawns_each_burst_that_lists_it() {
    let mut system = ParticleSystem::new(fixture(64));
    assert_eq!(system.handle(GameEvent::EnemyHit, Some(AT)), 4);
    assert_eq!(system.handle(GameEvent::PlayerDamaged, Some(AT)), 4);
    assert_eq!(system.handle(GameEvent::PlayerLanded, Some(AT)), 3);
    assert_eq!(system.handle(GameEvent::PlayerDashed, Some(AT)), 2);
    assert_eq!(system.handle(GameEvent::EnemyDied, Some(AT)), 5);
    assert_eq!(system.len(), 18);

    // Two presets may share an event; both spawn, in the fixed preset order.
    let mut config = fixture(64);
    config.dust.on.push(GameEvent::EnemyHit);
    let mut system = ParticleSystem::new(config);
    assert_eq!(system.handle(GameEvent::EnemyHit, Some(AT)), 4 + 3);
}

#[test]
fn events_nobody_lists_or_without_a_usable_position_spawn_nothing() {
    let mut system = ParticleSystem::new(fixture(64));
    assert_eq!(system.handle(GameEvent::CoinCollected, Some(AT)), 0);
    assert_eq!(system.handle(GameEvent::EnemyHit, None), 0);
    assert_eq!(
        system.handle(GameEvent::EnemyHit, Some(Vec2::new(f32::NAN, 0.0))),
        0
    );
    assert_eq!(
        system.handle(GameEvent::EnemyHit, Some(Vec2::new(0.0, f32::INFINITY))),
        0
    );
    assert!(system.is_empty());

    // A preset with an empty trigger list is off.
    let mut config = fixture(64);
    config.hit_spark.on.clear();
    let mut system = ParticleSystem::new(config);
    assert_eq!(system.handle(GameEvent::EnemyHit, Some(AT)), 0);
}

#[test]
fn particles_start_inside_the_spread_box_around_the_offset_point() {
    let mut config = fixture(64);
    config.hit_spark.spread = (10.0, 4.0);
    config.hit_spark.offset = (0.0, -18.0);
    let mut system = ParticleSystem::new(config);
    system.handle(GameEvent::EnemyHit, Some(AT));
    assert_eq!(system.len(), 4);
    for particle in system.particles() {
        let from_center = particle.position - (AT + Vec2::new(0.0, -18.0));
        assert!(from_center.x.abs() <= 10.0 && from_center.y.abs() <= 4.0);
    }
    // With a box this big, the four particles do not all start on the same spot.
    let first = system.particles()[0].position;
    assert!(system.particles().iter().any(|p| p.position != first));
}

// ---- lifetimes ------------------------------------------------------------------------

#[test]
fn lifetimes_are_drawn_from_the_span() {
    let mut config = fixture(64);
    config.death_burst.lifetime_secs = Span { min: 0.2, max: 0.6 };
    let mut system = ParticleSystem::new(config);
    system.handle(GameEvent::EnemyDied, Some(AT));
    for particle in system.particles() {
        assert!((0.2..=0.6).contains(&particle.lifetime()));
        assert_eq!(particle.age(), 0.0);
    }
}

#[test]
fn a_particle_is_removed_on_the_step_that_reaches_its_lifetime() {
    let mut system = ParticleSystem::new(fixture(64)); // every lifetime is 0.5 s
    system.handle(GameEvent::PlayerDashed, Some(AT));
    assert_eq!(system.len(), 2);
    for _ in 0..49 {
        system.update(0.01);
    }
    assert_eq!(system.len(), 2, "alive just before 0.5 s");
    system.update(0.011);
    assert!(system.is_empty(), "gone once 0.5 s has passed");
}

#[test]
fn a_huge_step_removes_everything_without_leaving_bad_numbers() {
    let mut system = ParticleSystem::new(fixture(64));
    system.handle(GameEvent::EnemyDied, Some(AT));
    system.update(1.0e30);
    assert!(system.is_empty());
}

#[test]
fn zero_negative_and_non_finite_steps_change_nothing() {
    let mut system = ParticleSystem::new(fixture(64));
    system.handle(GameEvent::EnemyDied, Some(AT));
    let before = system.particles().to_vec();
    for step in [0.0, -0.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        system.update(step);
        assert_eq!(system.particles(), before);
    }
}

#[test]
fn size_and_color_blend_from_birth_to_death() {
    let mut system = ParticleSystem::new(fixture(64)); // size 10 -> 2, lifetime 0.5 s
    system.handle(GameEvent::PlayerDashed, Some(AT));
    let born = system.particles()[0];
    assert_eq!(born.progress(), 0.0);
    assert_eq!(born.size(), 10.0);
    assert_eq!(born.color(), [200, 100, 0, 255]);

    system.update(0.25);
    let half = system.particles()[0];
    assert!((half.progress() - 0.5).abs() < 1e-5);
    assert!((half.size() - 6.0).abs() < 1e-4);
    assert_eq!(half.color(), [150, 50, 100, 128]);
}

// ---- motion ---------------------------------------------------------------------------

fn travelled(config: ParticleConfig, event: GameEvent, seconds: f32) -> Vec2 {
    let mut system = ParticleSystem::new(config);
    system.handle(event, Some(AT));
    let start = system.particles()[0].position;
    let steps = (seconds / 0.01).round() as usize;
    for _ in 0..steps {
        system.update(0.01);
    }
    system.particles()[0].position - start
}

#[test]
fn direction_zero_goes_right_and_minus_ninety_goes_up() {
    // 100 px/s for 0.2 s, no gravity or drag.
    let right = travelled(fixture(64), GameEvent::PlayerDashed, 0.2);
    assert!(
        (right.x - 20.0).abs() < 0.1 && right.y.abs() < 0.1,
        "{right}"
    );

    let mut config = fixture(64);
    config.dash_trail.direction_deg = Span {
        min: -90.0,
        max: -90.0,
    };
    let up = travelled(config, GameEvent::PlayerDashed, 0.2);
    assert!(up.x.abs() < 0.1 && (up.y + 20.0).abs() < 0.1, "{up}");
}

#[test]
fn gravity_pulls_down_and_drag_slows() {
    let plain = travelled(fixture(64), GameEvent::PlayerDashed, 0.3);

    let mut config = fixture(64);
    config.dash_trail.gravity = 500.0;
    let falling = travelled(config, GameEvent::PlayerDashed, 0.3);
    assert!(falling.y > plain.y + 10.0, "{falling} vs {plain}");
    assert!((falling.x - plain.x).abs() < 0.1);

    let mut config = fixture(64);
    config.dash_trail.drag = 5.0;
    let dragged = travelled(config, GameEvent::PlayerDashed, 0.3);
    assert!(
        dragged.x > 0.0 && dragged.x < plain.x - 5.0,
        "{dragged} vs {plain}"
    );
}

// ---- determinism ----------------------------------------------------------------------

/// Spawns varied bursts, steps them and returns what is alive.
fn scripted(seed: u64) -> Vec<Particle> {
    let config = ParticleConfig {
        max_particles: 512,
        ..ParticleConfig::default()
    };
    let mut system = ParticleSystem::with_seed(config, seed);
    for (index, event) in [
        GameEvent::EnemyHit,
        GameEvent::PlayerLanded,
        GameEvent::PlayerDashed,
        GameEvent::EnemyDied,
    ]
    .into_iter()
    .enumerate()
    {
        system.handle(event, Some(AT + Vec2::new(index as f32 * 17.0, 0.0)));
        for _ in 0..7 {
            system.update(1.0 / 120.0);
        }
    }
    system.particles().to_vec()
}

#[test]
fn the_same_seed_and_events_give_identical_particles() {
    let first = scripted(FIXED_SEED);
    assert!(!first.is_empty());
    assert_eq!(first, scripted(FIXED_SEED));
    assert_ne!(first, scripted(FIXED_SEED + 1), "another seed varies them");
}

#[test]
fn a_new_system_uses_the_fixed_seed() {
    let mut a = ParticleSystem::new(ParticleConfig::default());
    let mut b = ParticleSystem::with_seed(ParticleConfig::default(), FIXED_SEED);
    a.handle(GameEvent::EnemyDied, Some(AT));
    b.handle(GameEvent::EnemyDied, Some(AT));
    assert_eq!(a.particles(), b.particles());
}

// ---- the cap --------------------------------------------------------------------------

#[test]
fn bursts_stop_at_the_particle_cap_and_room_frees_up_as_particles_expire() {
    let mut system = ParticleSystem::new(fixture(7));
    assert_eq!(system.handle(GameEvent::EnemyDied, Some(AT)), 5);
    assert_eq!(
        system.handle(GameEvent::EnemyDied, Some(AT)),
        2,
        "cut short"
    );
    assert_eq!(system.len(), 7);
    assert_eq!(system.handle(GameEvent::EnemyHit, Some(AT)), 0, "full");
    assert_eq!(system.len(), 7);

    system.update(0.6);
    assert!(system.is_empty());
    assert_eq!(system.handle(GameEvent::EnemyDied, Some(AT)), 5);
}

#[test]
fn a_full_system_stays_deterministic() {
    let run = || {
        let mut system = ParticleSystem::new(fixture(6));
        for _ in 0..4 {
            system.handle(GameEvent::EnemyDied, Some(AT));
        }
        system.handle(GameEvent::EnemyHit, Some(AT));
        system.particles().to_vec()
    };
    assert_eq!(run(), run());
    assert_eq!(run().len(), 6);
}

// ---- clearing and reloading -----------------------------------------------------------

#[test]
fn entering_a_room_clears_the_particles_and_spawns_nothing() {
    let mut system = ParticleSystem::new(fixture(64));
    system.handle(GameEvent::EnemyDied, Some(AT));
    assert!(!system.is_empty());
    assert_eq!(system.handle(GameEvent::RoomEntered, None), 0);
    assert!(system.is_empty());

    // Events after the room change, in the same batch, still spawn.
    assert_eq!(system.handle(GameEvent::EnemyDied, Some(AT)), 5);
}

#[test]
fn clear_leaves_the_generator_where_it_was() {
    let mut cleared = ParticleSystem::new(fixture(64));
    let mut kept = ParticleSystem::new(fixture(64));
    cleared.handle(GameEvent::EnemyDied, Some(AT));
    kept.handle(GameEvent::EnemyDied, Some(AT));
    cleared.clear();
    cleared.handle(GameEvent::EnemyHit, Some(AT));
    kept.handle(GameEvent::EnemyHit, Some(AT));
    assert_eq!(cleared.particles(), &kept.particles()[5..]);
}

#[test]
fn a_new_config_applies_to_later_bursts_and_live_particles_keep_theirs() {
    let mut system = ParticleSystem::new(fixture(64));
    system.handle(GameEvent::PlayerDashed, Some(AT));
    let mut config = fixture(64);
    config.dash_trail.count = 9;
    system.set_config(config);
    assert_eq!(system.len(), 2);
    assert_eq!(system.handle(GameEvent::PlayerDashed, Some(AT)), 9);
    assert_eq!(system.config().dash_trail.count, 9);
}

// ---- validation and loading -----------------------------------------------------------

fn error_for(change: impl FnOnce(&mut ParticleConfig)) -> String {
    let mut config = fixture(64);
    change(&mut config);
    config.validate().unwrap_err()
}

#[test]
fn bad_values_are_rejected_and_the_message_names_the_field() {
    assert!(error_for(|c| c.max_particles = 0).contains("max_particles"));
    assert!(error_for(|c| c.max_particles = MAX_PARTICLE_CAP + 1).contains("max_particles"));
    assert!(error_for(|c| c.dust.count = 0).contains("dust.count"));
    assert!(error_for(|c| c.dust.count = MAX_BURST_COUNT + 1).contains("dust.count"));
    assert!(error_for(|c| c.hit_spark.lifetime_secs.min = 0.0).contains("hit_spark.lifetime_secs"));
    assert!(error_for(|c| c.hit_spark.lifetime_secs.max = 9.0).contains("hit_spark.lifetime_secs"));
    assert!(error_for(|c| {
        c.dash_trail.speed = Span {
            min: 50.0,
            max: 10.0,
        }
    })
    .contains("min must not be above max"));
    assert!(error_for(|c| c.dash_trail.speed.min = -1.0).contains("dash_trail.speed"));
    assert!(error_for(|c| c.death_burst.gravity = f32::NAN).contains("death_burst.gravity"));
    assert!(error_for(|c| c.death_burst.drag = -0.5).contains("death_burst.drag"));
    assert!(error_for(|c| c.death_burst.size.1 = MAX_SIZE + 1.0).contains("size.end"));
    assert!(error_for(|c| c.dust.spread.0 = -1.0).contains("spread.x"));
    assert!(error_for(|c| c.dust.offset.1 = f32::INFINITY).contains("offset.y"));
    assert!(error_for(|c| c.dust.direction_deg.max = 400.0).contains("direction_deg"));
    assert!(error_for(|c| c.dust.on = vec![GameEvent::PlayerLanded; 2]).contains("twice"));
}

#[test]
fn bad_files_are_rejected() {
    // Unknown field, unknown event name, wrong color length, missing preset.
    let with = |from: &str, to: &str| fixture_text(64).replacen(from, to, 1);
    assert!(ParticleConfig::from_ron(&with("max_particles", "max_particle")).is_err());
    assert!(ParticleConfig::from_ron(&with("EnemyDied", "EnemyDyed")).is_err());
    assert!(ParticleConfig::from_ron(&with("(200, 100, 0, 255)", "(200, 100, 0)")).is_err());
    let missing = format!(
        "(max_particles: 8, dust: {})",
        preset_text("PlayerLanded", 1)
    );
    assert!(ParticleConfig::from_ron(&missing).is_err());
    // A valid file with one out-of-range number fails validation, not parsing.
    let error = ParticleConfig::from_ron(&with("count: 3", "count: 0")).unwrap_err();
    assert!(error.contains("count"), "{error}");
}

#[test]
fn load_uses_the_defaults_without_a_file_and_replaces_them_whole_with_one() {
    let dir = TempDir::new();
    assert_eq!(
        ParticleConfig::load(&dir.0).unwrap(),
        ParticleConfig::default()
    );

    std::fs::write(dir.0.join(FILE_NAME), fixture_text(11)).unwrap();
    let loaded = ParticleConfig::load(&dir.0).unwrap();
    assert_eq!(loaded.max_particles, 11);
    assert_eq!(loaded, fixture(11));
}

#[test]
fn load_names_the_file_when_it_is_invalid_instead_of_falling_back() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(max_particles: 0)").unwrap();
    let error = ParticleConfig::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
}
