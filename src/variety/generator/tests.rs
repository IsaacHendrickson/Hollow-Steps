//! The generator's rules: what loads, what is refused and how the error names the field.
use super::*;
use crate::variety::VarietyRules;

fn rules_with(change: impl FnOnce(&mut GeneratorRules)) -> VarietyRules {
    let mut generator = GeneratorRules::default();
    change(&mut generator);
    VarietyRules {
        generator,
        ..VarietyRules::default()
    }
}

fn parse(section: &str) -> Result<VarietyRules, String> {
    VarietyRules::from_ron(&format!("(flip_percent: 0.0, generator: ({section}))"))
}

// ---- the defaults ----

#[test]
fn the_defaults_are_valid_and_the_generator_is_off() {
    let defaults = GeneratorRules::default();
    defaults.validate().unwrap();
    assert!(!defaults.enabled);
}

#[test]
fn the_embedded_file_spells_out_the_defaults_and_loads() {
    // `data/variety.ron` carries the section; a file with none gets the same numbers.
    let embedded = VarietyRules::default();
    assert_eq!(embedded.generator, GeneratorRules::default());
    embedded.validate().unwrap();
}

#[test]
fn a_file_from_before_the_section_still_loads_with_the_defaults() {
    let old = VarietyRules::from_ron("(flip_percent: 37.5)").unwrap();
    assert_eq!(old.generator, GeneratorRules::default());
    let older = VarietyRules::from_ron("(flip_percent: 10.0, room_flip_percent: 20.0)").unwrap();
    assert!(!older.generator.enabled);
}

#[test]
fn a_partial_section_keeps_the_defaults_for_what_it_leaves_out() {
    let rules = parse("enabled: true, keys: (min: 1, max: 3)")
        .unwrap()
        .generator;
    assert!(rules.enabled);
    assert_eq!(rules.keys, CountRange { min: 1, max: 3 });
    assert_eq!(rules.rooms, GeneratorRules::default().rooms);
    assert_eq!(rules.attempts, GeneratorRules::default().attempts);
}

#[test]
fn a_full_section_loads_and_both_ends_of_every_range_are_allowed() {
    let lowest = "enabled: true, rooms: (min: 3, max: 3), spine: (min: 2, max: 2), \
                  keys: (min: 0, max: 0), mirror_percent: 0.0, attempts: 1";
    let highest = "enabled: true, rooms: (min: 40, max: 40), spine: (min: 40, max: 40), \
                   keys: (min: 8, max: 8), mirror_percent: 100.0, attempts: 10000";
    for section in [lowest, highest] {
        let rules = parse(section).unwrap_or_else(|error| panic!("{section}: {error}"));
        assert!(rules.generator.enabled);
    }
    let rules = parse(highest).unwrap().generator;
    assert_eq!(rules.rooms.bounds(), (40, 40));
    assert_eq!(rules.keys.bounds(), (8, 8));
    assert_eq!(rules.attempt_limit(), 10_000);
}

// ---- bad values ----

#[test]
fn each_bad_number_is_refused_and_the_error_names_the_field() {
    let cases = [
        ("rooms: (min: 2, max: 10)", "generator.rooms.min"),
        ("rooms: (min: -1, max: 10)", "generator.rooms.min"),
        ("rooms: (min: 3, max: 41)", "generator.rooms.max"),
        ("rooms: (min: 10, max: 5)", "generator.rooms.min"),
        (
            "rooms: (min: 9, max: 9), spine: (min: 2, max: 10)",
            "generator.spine.max",
        ),
        ("spine: (min: 1, max: 5)", "generator.spine.min"),
        ("spine: (min: 0, max: 5)", "generator.spine.min"),
        ("spine: (min: 6, max: 4)", "generator.spine.min"),
        ("spine: (min: 4, max: 41)", "generator.spine.max"),
        ("keys: (min: -1, max: 2)", "generator.keys.min"),
        ("keys: (min: 0, max: 9)", "generator.keys.max"),
        ("keys: (min: 3, max: 2)", "generator.keys.min"),
        ("mirror_percent: -0.5", "generator.mirror_percent"),
        ("mirror_percent: 100.5", "generator.mirror_percent"),
        ("attempts: 0", "generator.attempts"),
        ("attempts: -3", "generator.attempts"),
        ("attempts: 10001", "generator.attempts"),
    ];
    for (section, field) in cases {
        let error = parse(section).unwrap_err();
        assert!(error.contains(field), "{section}: {error}");
        assert!(
            error.starts_with("variety:") || error.contains(field),
            "{error}"
        );
    }
}

#[test]
fn a_mirror_percent_that_is_not_a_number_is_refused() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let error = rules_with(|rules| rules.mirror_percent = bad)
            .validate()
            .unwrap_err();
        assert!(error.contains("generator.mirror_percent"), "{bad}: {error}");
    }
}

#[test]
fn a_section_that_is_unknown_or_mistyped_is_refused() {
    for section in [
        "speed: 3",
        "rooms: (min: 3, max: 9, step: 1)",
        "rooms: (min: 3)",
        "rooms: (3, 9)",
        "enabled: \"yes\"",
        "attempts: 2.5",
        "keys: (min: \"none\", max: 2)",
    ] {
        let error = parse(section).unwrap_err();
        assert!(error.starts_with("variety:"), "{section}: {error}");
    }
}

#[test]
fn a_bad_section_is_refused_when_the_game_loads_the_file() {
    // The same path as start-up: `VarietyRules::load` reads `assets/variety.ron` and validates it.
    let dir = std::env::temp_dir().join(format!(
        "hollow-steps-generator-rules-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join(crate::variety::flip::FILE_NAME),
        "(flip_percent: 0.0, generator: (attempts: 0))",
    )
    .unwrap();
    let error = VarietyRules::load(&dir).unwrap_err();
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(error.contains("generator.attempts"), "{error}");
    assert!(error.contains(crate::variety::flip::FILE_NAME), "{error}");
}

#[test]
fn flip_rules_are_still_checked_beside_the_section() {
    let error = VarietyRules::from_ron("(flip_percent: 120.0, generator: ())").unwrap_err();
    assert!(error.contains("flip_percent"), "{error}");
}

// ---- whether a run is generated ----

#[test]
fn never_plays_the_fixed_dungeon_and_always_asks_the_generator_whatever_the_rules_say() {
    for enabled in [false, true] {
        for ends in [false, true] {
            let rules = GeneratorRules {
                enabled,
                ..GeneratorRules::default()
            };
            assert!(!decide_generation(GenerateChoice::Never, &rules, ends));
            assert!(decide_generation(GenerateChoice::Always, &rules, ends));
        }
    }
}

#[test]
fn auto_generates_only_when_the_rules_are_on_and_the_pool_has_a_start_and_a_boss() {
    let rules = |enabled| GeneratorRules {
        enabled,
        ..GeneratorRules::default()
    };
    assert!(decide_generation(GenerateChoice::Auto, &rules(true), true));
    assert!(!decide_generation(
        GenerateChoice::Auto,
        &rules(true),
        false
    ));
    assert!(!decide_generation(
        GenerateChoice::Auto,
        &rules(false),
        true
    ));
    assert!(!decide_generation(
        GenerateChoice::Auto,
        &rules(false),
        false
    ));
}

#[test]
fn the_default_choice_is_auto_and_the_default_rules_leave_it_on_the_fixed_dungeon() {
    assert_eq!(GenerateChoice::default(), GenerateChoice::Auto);
    let rules = GeneratorRules::default();
    assert!(!decide_generation(GenerateChoice::default(), &rules, true));
}
