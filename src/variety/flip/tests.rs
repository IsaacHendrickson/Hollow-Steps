//! The flip rules file and the choice, with no game: what loads, what is refused, and the pure
//! decision of whether a seed flips. The draws of three seeds are pinned against a separate
//! implementation of the stream mixing, so a change to it fails here, not in a player's run.
use super::*;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-variety-rules-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, text: &str) {
        std::fs::write(self.0.join(FILE_NAME), text).unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn rules(percent: f32) -> VarietyRules {
    VarietyRules {
        flip_percent: percent,
        ..VarietyRules::default()
    }
}

// ---- the file ----

#[test]
fn the_embedded_default_never_flips() {
    assert_eq!(VarietyRules::default().flip_percent, 0.0);
    assert_eq!(
        VarietyRules::default(),
        VarietyRules::from_ron(DEFAULTS).unwrap()
    );
}

#[test]
fn a_good_file_loads_and_both_ends_of_the_range_are_allowed() {
    for text in [
        "(flip_percent: 37.5)",
        "(flip_percent: 0.0)",
        "(flip_percent: 100.0)",
    ] {
        assert!(VarietyRules::from_ron(text).is_ok(), "{text}");
    }
    assert_eq!(
        VarietyRules::from_ron("(flip_percent: 37.5)").unwrap(),
        rules(37.5)
    );
}

#[test]
fn each_bad_flip_percent_is_refused_and_the_error_names_the_field() {
    for bad in [
        -0.1,
        100.1,
        1_000.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ] {
        let error = rules(bad).validate().unwrap_err();
        assert!(error.contains("flip_percent"), "{bad}: {error}");
    }
    for text in ["(flip_percent: -1.0)", "(flip_percent: 101.0)"] {
        let error = VarietyRules::from_ron(text).unwrap_err();
        assert!(error.contains("flip_percent"), "{text}: {error}");
    }
}

#[test]
fn a_file_with_a_missing_unknown_or_mistyped_field_is_refused() {
    for text in [
        "()",
        "(flip_percent: 50.0, flip_chance: 1.0)",
        "(flip_percent: \"half\")",
        "(flip_percent: true)",
        "not ron at all",
    ] {
        let error = VarietyRules::from_ron(text).unwrap_err();
        assert!(error.starts_with("variety:"), "{text}: {error}");
    }
}

#[test]
fn without_a_file_the_embedded_default_is_used() {
    let dir = TempDir::new();
    assert_eq!(VarietyRules::load(&dir.0).unwrap(), VarietyRules::default());
}

#[test]
fn a_file_in_assets_replaces_the_default_whole() {
    let dir = TempDir::new();
    dir.write("// a comment\n(flip_percent: 62.5)");
    assert_eq!(VarietyRules::load(&dir.0).unwrap(), rules(62.5));
}

#[test]
fn a_bad_file_in_assets_is_an_error_naming_it_never_a_fallback() {
    let dir = TempDir::new();
    dir.write("(flip_percent: 250.0)");
    let error = VarietyRules::load(&dir.0).unwrap_err();
    assert!(
        error.contains(FILE_NAME) && error.contains("flip_percent"),
        "{error}"
    );
    dir.write("(flip_percent: 1.0, extra: 2)");
    assert!(VarietyRules::load(&dir.0).unwrap_err().contains(FILE_NAME));
}

#[test]
fn a_file_that_cannot_be_read_is_an_error_naming_it() {
    let dir = TempDir::new();
    std::fs::create_dir(dir.0.join(FILE_NAME)).unwrap();
    assert!(VarietyRules::load(&dir.0).unwrap_err().contains(FILE_NAME));
}

// ---- the choice ----

#[test]
fn the_choice_starts_on_auto_and_has_three_labelled_options() {
    assert_eq!(FlipChoice::default(), FlipChoice::Auto);
    let labels: Vec<_> = FlipChoice::ALL
        .iter()
        .map(|choice| choice.label())
        .collect();
    assert_eq!(labels, ["Auto", "Never", "Always"]);
}

#[test]
fn never_and_always_override_the_rules() {
    for percent in [0.0, 50.0, 100.0] {
        for seed in 0..50 {
            assert!(!decide_flip(FlipChoice::Never, &rules(percent), seed));
            assert!(decide_flip(FlipChoice::Always, &rules(percent), seed));
        }
    }
}

#[test]
fn zero_percent_never_flips_and_one_hundred_always_does() {
    for seed in (0..5_000u64).chain([u64::MAX, u64::MAX - 1, 1 << 63]) {
        assert!(!decide_flip(FlipChoice::Auto, &rules(0.0), seed), "{seed}");
        assert!(decide_flip(FlipChoice::Auto, &rules(100.0), seed), "{seed}");
    }
}

/// The first number of `stream(seed, "flip", 0)` as the 24 bits `unit()` is made of. Worked out
/// with a separate implementation of the mixing (FNV-1a of the label, then SplitMix steps).
#[test]
fn the_draws_of_three_seeds_are_pinned() {
    for (seed, bits) in [
        (0u64, 2_367_871u64),
        (1, 9_143_875),
        (2, 829_144),
        (6, 15_422_930),
    ] {
        assert_eq!(
            stream(seed, "flip", 0).next_u64() >> 40,
            bits,
            "seed {seed}"
        );
    }
}

#[test]
fn auto_is_pinned_for_a_seed_on_both_sides_of_its_draw() {
    // Seed 0 draws 14.11 percent, seed 1 draws 54.50 percent.
    assert!(!decide_flip(FlipChoice::Auto, &rules(14.0), 0));
    assert!(decide_flip(FlipChoice::Auto, &rules(14.2), 0));
    assert!(decide_flip(FlipChoice::Auto, &rules(50.0), 0));
    assert!(!decide_flip(FlipChoice::Auto, &rules(50.0), 1));
    assert!(!decide_flip(FlipChoice::Auto, &rules(54.4), 1));
    assert!(decide_flip(FlipChoice::Auto, &rules(54.6), 1));
}

#[test]
fn the_same_seed_always_gives_the_same_answer() {
    for seed in 0..200 {
        let first = decide_flip(FlipChoice::Auto, &rules(50.0), seed);
        assert_eq!(first, decide_flip(FlipChoice::Auto, &rules(50.0), seed));
    }
}

#[test]
fn a_percent_flips_about_that_share_of_seeds() {
    for percent in [10.0f32, 50.0, 90.0] {
        let flips = (0..4_000u64)
            .filter(|seed| decide_flip(FlipChoice::Auto, &rules(percent), seed * 7_919 + 13))
            .count() as f32;
        let share = flips / 4_000.0 * 100.0;
        assert!((share - percent).abs() < 4.0, "{percent}: {share}");
    }
}

#[test]
fn a_higher_percent_never_flips_fewer_seeds() {
    for seed in 0..300u64 {
        let mut was = false;
        for percent in [0.0, 10.0, 25.0, 50.0, 75.0, 100.0] {
            let now = decide_flip(FlipChoice::Auto, &rules(percent), seed);
            assert!(now || !was, "seed {seed} at {percent}");
            was = now;
        }
    }
}

#[test]
fn the_draw_does_not_share_a_stream_with_the_enemy_draws() {
    // A different label gives a different stream, so the flip never moves an enemy pick.
    for seed in 0..50u64 {
        assert_ne!(
            stream(seed, "flip", 0).next_u64(),
            stream(seed, "enemies", 0).next_u64()
        );
    }
}
