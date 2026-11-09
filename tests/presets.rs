mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hollow_steps::behavior::{Behavior, BehaviorContext, BehaviorDefinition};
use hollow_steps::behavior_values::KeyValues;
use hollow_steps::definition_editor::{serialize, DefinitionDraft};
use hollow_steps::player::WorldBounds;
use hollow_steps::preset::{load_presets, BehaviorPreset};
use macroquad::math::Vec2;

struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-presets-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["rooms", "entities", "behaviors", "presets"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn presets() -> Vec<BehaviorPreset> {
    load_presets(&support::fixture_assets().join("presets")).unwrap()
}
fn preset(name: &str) -> BehaviorPreset {
    presets().into_iter().find(|p| p.name == name).unwrap()
}
fn assert_file(error: &str, path: &Path) {
    assert_eq!(
        Path::new(error.split_once(": ").unwrap().0),
        path,
        "{error}"
    );
}

#[test]
fn loads_full_named_described_presets_and_searches_names_or_descriptions() {
    let presets = presets();
    assert_eq!(
        presets.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(),
        ["Charger", "Chaser", "Patrol", "Skittish", "Stationary"]
    );
    for preset in &presets {
        preset.validate().unwrap();
        assert!(!preset.description.trim().is_empty());
        assert_eq!(
            *preset,
            BehaviorPreset::from_ron(&serialize(preset).unwrap()).unwrap()
        );
    }
    let labels = presets
        .iter()
        .map(|p| format!("{}: {}", p.name, p.description))
        .collect::<Vec<_>>();
    assert_eq!(hollow_steps::search::filter_names(&labels, " cHaSeR "), [1]);
    assert_eq!(hollow_steps::search::filter_names(&labels, "recover"), [0]);
    assert!(hollow_steps::search::filter_names(&labels, "missing").is_empty());
}

#[test]
fn validates_metadata_full_definition_and_existing_action_condition_catalog() {
    let valid = serialize(&preset("Chaser")).unwrap();
    let bad_sources = [
        valid.replace("name: \"Chaser\"", "name: \" \""),
        valid.replace("description:", "unknown_field:"),
        valid.replace("initial_state: \"patrol\"", "initial_state: \"missing\""),
        valid.replace("target: \"chase\"", "target: \"missing\""),
        valid.replace("move_toward_player", "fly_at_player"),
        valid.replace("player_distance_less_than", "can_see_player"),
        valid.replace("speed: 130.0", "speed: -1.0"),
        valid.replace("speed: 130.0", "speed: NaN"),
        valid.replace(
            "player_distance_less_than(180.0)",
            "player_distance_less_than(-1.0)",
        ),
        valid.replace(
            "player_distance_less_than(180.0)",
            "player_distance_less_than(inf)",
        ),
        valid.replace("name: \"chase\"", "name: \"patrol\""),
        "(name: \"Incomplete\", description: \"Missing definition\")".into(),
    ];
    for source in bad_sources {
        assert_ne!(source, valid, "test must change the fixture");
        assert!(
            BehaviorPreset::from_ron(&source).is_err(),
            "accepted {source}"
        );
    }
    let mut empty_description = preset("Patrol");
    empty_description.description = " \n ".into();
    assert!(empty_description
        .validate()
        .unwrap_err()
        .contains("description"));
}

#[test]
fn catalog_errors_include_file_preset_and_state_and_reject_duplicate_names() {
    let assets = Assets::new();
    let path = assets.0.join("presets").join("chaser.ron");
    let mut invalid = preset("Chaser");
    invalid.behavior.states[0].transitions[0].target = "missing".into();
    std::fs::write(&path, serialize(&invalid).unwrap()).unwrap();
    let error = DefinitionDraft::load(&assets.0).unwrap_err();
    assert_file(&error, &path);
    assert!(error.contains("Chaser") && error.contains("patrol") && error.contains("missing"));
    std::fs::write(&path, "(broken").unwrap();
    let error = load_presets(&assets.0.join("presets")).unwrap_err();
    assert_file(&error, &path);
    assert!(error.contains("1:"), "parse position: {error}");
    std::fs::write(&path, serialize(&preset("Chaser")).unwrap()).unwrap();
    let mut duplicate = preset("Chaser");
    duplicate.name = " chaser ".into();
    let path = assets.0.join("presets").join("z_duplicate.ron");
    std::fs::write(&path, serialize(&duplicate).unwrap()).unwrap();
    let error = load_presets(&assets.0.join("presets")).unwrap_err();
    assert_file(&error, &path);
    assert!(error.contains("duplicate preset name"));
    assert!(load_presets(&assets.0.join("absent")).unwrap().is_empty());
}

#[test]
fn create_assign_and_save_are_independent_of_preset_and_keep_behavior_file_format() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.entity_path("patrol").unwrap();
    let template = preset("Chaser");
    let template_path = assets.0.join("presets").join("chaser.ron");
    let template_bytes = std::fs::read(&template_path).unwrap();
    draft
        .create_behavior_from_preset(&template, " lonely ", None)
        .unwrap();
    assert_eq!(draft.behavior_users("lonely"), 0);
    assert_eq!(draft.entities[&path].behavior, "patrol");
    draft
        .create_behavior_from_preset(&template, "hunter", Some(&path))
        .unwrap();
    assert_eq!(draft.entities[&path].behavior, "hunter");
    let mut values = KeyValues::from_behavior(&draft.behaviors["hunter"]);
    values.values[0].value = "99".into();
    values
        .apply(draft.behaviors.get_mut("hunter").unwrap())
        .unwrap();
    assert_eq!(draft.behaviors["lonely"], template.behavior);
    assert_eq!(draft.presets, presets());
    let expected = draft.behaviors["hunter"].clone();
    draft.commit(&[]).unwrap();
    let reloaded = DefinitionDraft::load(&assets.0).unwrap();
    assert_eq!(reloaded.entities[&path].behavior, "hunter");
    assert_eq!(reloaded.behaviors["hunter"], expected);
    let source = std::fs::read_to_string(assets.0.join("behaviors").join("hunter.ron")).unwrap();
    assert_eq!(BehaviorDefinition::from_ron(&source).unwrap(), expected);
    assert_eq!(std::fs::read(template_path).unwrap(), template_bytes);
    draft.assign_behavior(&path, "lonely").unwrap();
    assert_eq!(draft.entities[&path].behavior, "lonely");
}

#[test]
fn invalid_creation_or_assignment_never_partially_mutates_draft() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let entities = draft.entities.clone();
    let behaviors = draft.behaviors.clone();
    for name in ["", ".", "..", "../escape", "bad:name", "patrol"] {
        assert!(draft
            .create_behavior_from_preset(&preset("Patrol"), name, None)
            .is_err());
    }
    let missing = assets.0.join("entities").join("missing.ron");
    assert!(draft
        .create_behavior_from_preset(&preset("Patrol"), "new", Some(&missing))
        .is_err());
    let mut invalid = preset("Patrol");
    invalid.behavior.initial_state = "missing".into();
    assert!(draft
        .create_behavior_from_preset(&invalid, "new", None)
        .is_err());
    let path = draft.entity_path("patrol").unwrap();
    assert!(draft.assign_behavior(&path, "missing").is_err());
    assert!(draft.assign_behavior(&missing, "patrol").is_err());
    assert_eq!(draft.entities, entities);
    assert_eq!(draft.behaviors, behaviors);
}

#[test]
fn shared_warning_counts_entity_definitions_and_copy_only_reassigns_selected_entity() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.entity_path("patrol").unwrap();
    assert_eq!(draft.behavior_users("patrol"), 1);
    assert!(!draft.behavior_needs_warning("patrol", None));
    let other = draft
        .duplicate_entity(&path, "unused second entity")
        .unwrap();
    assert_eq!(draft.behavior_users("patrol"), 2);
    assert!(draft.behavior_needs_warning("patrol", None));
    assert!(draft.behavior_needs_warning("patrol", Some("chase")));
    assert!(!draft.behavior_needs_warning("patrol", Some("patrol")));
    let before = draft.behaviors["patrol"].clone();
    assert!(draft
        .copy_behavior("patrol", "patrol", Some(&path))
        .is_err());
    draft
        .copy_behavior("patrol", "personal", Some(&path))
        .unwrap();
    assert_eq!(draft.behavior_users("personal"), 1);
    assert_eq!(draft.behavior_users("patrol"), 1);
    assert_eq!(draft.entities[&other].behavior, "patrol");
    assert_eq!(draft.entities[&path].behavior, "personal");
    assert_eq!(draft.behaviors["personal"], before);
    assert!(!draft.behavior_needs_warning("personal", None));
    draft.copy_behavior("patrol", "unassigned", None).unwrap();
    assert_eq!(draft.behavior_users("unassigned"), 0);
    draft.commit(&[]).unwrap();
    assert_eq!(draft.entities[&path].behavior, "personal");
}

fn all_parameters() -> BehaviorDefinition {
    BehaviorDefinition::from_ron(r#"(initial_state: "chase", states: [
        (name: "chase", actions: [move_toward_player(speed: 1.0), move_away_from_player(speed: 2.0), patrol(speed: 3.0), wait(seconds: 4.0), telegraph(seconds: 5.0)], transitions: [
            (target: "rest", condition: timer_elapsed(1.0)),
            (target: "rest", condition: player_distance_less_than(2.0)),
            (target: "rest", condition: player_distance_greater_than(3.0)),
            (target: "rest", condition: health_below(4)),
            (target: "rest", condition: action_finished),
        ]),
        (name: "rest", actions: [wait(seconds: 6.0)], transitions: [
            (target: "chase", condition: timer_elapsed(1.0), invert: true),
            (target: "chase", condition: player_distance_less_than(2.0), invert: true),
            (target: "chase", condition: player_distance_greater_than(3.0), invert: true),
            (target: "chase", condition: health_below(4), invert: true),
        ]),
    ])"#).unwrap()
}

#[test]
fn key_values_cover_every_numeric_parameter_and_round_trip_edits_through_disk() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    draft.add_behavior("all", all_parameters()).unwrap();
    let behavior = draft.behaviors.get_mut("all").unwrap();
    let before = behavior.clone();
    let mut values = KeyValues::from_behavior(behavior);
    assert_eq!(values.values.len(), 14);
    assert!(values.values[..9]
        .iter()
        .all(|v| v.label.starts_with("chase:")));
    assert!(values.values[9..]
        .iter()
        .all(|v| v.label.starts_with("rest:")));
    assert!(values.values[6].label.contains("closer than"));
    assert!(values.values[11].label.contains("at least this far"));
    for (index, entry) in values.values.iter_mut().enumerate() {
        entry.value = (20 + index).to_string();
    }
    values.apply(behavior).unwrap();
    assert_eq!(KeyValues::from_behavior(behavior).values, values.values);
    assert_eq!(behavior.initial_state, before.initial_state);
    for (old, new) in before.states.iter().zip(&behavior.states) {
        assert_eq!(old.name, new.name);
        for (old, new) in old.transitions.iter().zip(&new.transitions) {
            assert_eq!((&old.target, old.invert), (&new.target, new.invert));
        }
    }
    let expected = behavior.clone();
    draft.commit(&[]).unwrap();
    let reloaded = DefinitionDraft::load(&assets.0).unwrap();
    assert_eq!(reloaded.behaviors["all"], expected);
    assert_eq!(
        KeyValues::from_behavior(&reloaded.behaviors["all"]).values,
        values.values
    );
}

#[test]
fn invalid_key_values_are_transactional_and_zero_and_integer_boundaries_are_valid() {
    let mut behavior = all_parameters();
    let original = behavior.clone();
    for (index, invalid) in [
        (0, "-1"),
        (1, "NaN"),
        (2, "inf"),
        (3, ""),
        (4, "abc"),
        (5, "-1"),
        (6, "NaN"),
        (7, "inf"),
        (8, "1.5"),
        (8, "256"),
        (8, "-1"),
    ] {
        let mut values = KeyValues::from_behavior(&behavior);
        values.values[0].value = "123".into();
        values.values[index].value = invalid.into();
        let error = values.apply(&mut behavior).unwrap_err();
        assert!(error.contains(&values.values[index].label));
        assert_eq!(behavior, original);
    }
    let mut values = KeyValues::from_behavior(&behavior);
    for entry in &mut values.values {
        entry.value = "0".into();
    }
    values.values[8].value = "255".into();
    values.apply(&mut behavior).unwrap();
    behavior.validate().unwrap();
    assert!(KeyValues::from_behavior(&preset("Stationary").behavior)
        .values
        .is_empty());
}

fn context(player_x: f32) -> BehaviorContext {
    BehaviorContext {
        player_position: Vec2::new(player_x, 442.0),
        health: 3,
        size: Vec2::new(36.0, 36.0),
        bounds: WorldBounds::new(0.0, 960.0, 460.0),
        patrol_left: 100.0,
        patrol_right: 500.0,
    }
}

#[test]
fn fixture_chaser_has_separate_start_and_give_up_distances() {
    let mut behavior = Behavior::new(Arc::new(preset("Chaser").behavior)).unwrap();
    let mut position = Vec2::new(100.0, 442.0);
    // Patrol moves right, away from this player, so the post-action check also stays outside.
    behavior.update(&mut position, context(-80.0), 0.1);
    assert_eq!(behavior.state_name(), "patrol"); // equality doesn't start chasing
    position.x = 100.0;
    behavior.update(&mut position, context(279.0), 0.1);
    assert_eq!(behavior.state_name(), "chase");
    position.x = 100.0;
    behavior.update(&mut position, context(400.0), 0.1);
    assert_eq!(behavior.state_name(), "chase"); // equality doesn't give up
    position.x = 100.0;
    behavior.update(&mut position, context(401.0), 0.1);
    assert_eq!(behavior.state_name(), "patrol");
}

#[test]
fn fixture_charger_telegraphs_before_fast_movement_and_other_templates_move_as_described() {
    let mut charger = Behavior::new(Arc::new(preset("Charger").behavior)).unwrap();
    let mut position = Vec2::new(100.0, 442.0);
    charger.update(&mut position, context(300.0), 0.3);
    assert!(charger.is_telegraphing());
    assert_eq!(position.x, 100.0);
    charger.update(&mut position, context(300.0), 0.3);
    assert_eq!(position.x, 100.0);
    charger.update(&mut position, context(300.0), 0.1);
    assert_eq!(position.x, 142.0);
    for (name, player, expected) in [
        ("Patrol", 900.0, 108.0),
        ("Skittish", 150.0, 84.0),
        ("Stationary", 150.0, 100.0),
    ] {
        let mut behavior = Behavior::new(Arc::new(preset(name).behavior)).unwrap();
        let mut position = Vec2::new(100.0, 442.0);
        behavior.update(&mut position, context(player), 0.0);
        assert_eq!(position.x, 100.0);
        behavior.update(&mut position, context(player), 0.1);
        assert_eq!(position.x, expected, "{name}");
    }
}
