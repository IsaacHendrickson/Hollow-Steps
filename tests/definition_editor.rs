mod support;

use hollow_steps::behavior::{Action, BehaviorDefinition, Condition, Transition};
use hollow_steps::definition_editor::{
    add_state, edit_state_lists, remove_state, rename_state, serialize, DefinitionDraft,
};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEnemies, RoomId};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-definition-editor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["entities", "behaviors", "rooms", "bosses"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        for filename in ["first.ron", "second.ron"] {
            let file = path.join("rooms").join(filename);
            let mut document: RoomEnemies =
                ron::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            document.enemies.push(hollow_steps::room::EnemyPlacement {
                definition: "patrol".into(),
                patrol_left: 300.0,
                patrol_right: 400.0,
            });
            std::fs::write(file, serialize(&document).unwrap()).unwrap();
        }
        Self(path)
    }
    fn snapshot(&self) -> Vec<(PathBuf, Vec<u8>)> {
        let mut files = Vec::new();
        for dir in ["entities", "behaviors", "rooms", "bosses"] {
            for entry in std::fs::read_dir(self.0.join(dir)).unwrap() {
                let path = entry.unwrap().path();
                files.push((path.clone(), std::fs::read(path).unwrap()));
            }
        }
        files.sort();
        files
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn behavior() -> BehaviorDefinition {
    BehaviorDefinition::from_ron(r#"(initial_state: "idle", states: [
        (name: "idle", actions: [wait(seconds: 1.0)], transitions: [(target: "idle", condition: action_finished)]),
    ])"#).unwrap()
}

fn assert_error_file(error: &str, expected: &Path) {
    let (reported, _) = error
        .split_once(": ")
        .expect("file errors must start with a path and colon");
    assert_eq!(Path::new(reported), expected, "{error}");
    assert_eq!(Path::new(reported).file_name(), expected.file_name());
}

#[test]
fn state_operations_preserve_links_and_reject_invalid_operations_without_mutation() {
    let mut b = behavior();
    let original = b.clone();
    assert!(remove_state(&mut b, 0).is_err());
    assert!(rename_state(&mut b, 99, "missing".into()).is_err());
    assert!(add_state(&mut b, " ".into()).is_err());
    assert_eq!(b, original);
    add_state(&mut b, "attack".into()).unwrap();
    let before = b.clone();
    assert!(rename_state(&mut b, 0, "attack".into()).is_err());
    assert!(add_state(&mut b, "idle".into()).is_err());
    assert_eq!(b, before);
    b.states[1].transitions.push(Transition {
        target: "idle".into(),
        condition: Condition::HealthBelow(2),
        invert: true,
    });
    rename_state(&mut b, 0, "rest".into()).unwrap();
    assert_eq!(b.initial_state, "rest");
    assert!(b.states.iter().all(|s| s.transitions[0].target == "rest"));
    remove_state(&mut b, 0).unwrap();
    assert_eq!(b.initial_state, "attack");
    assert!(b.states[0].transitions.is_empty());
    b.validate().unwrap();
}

#[test]
fn action_and_transition_lists_edit_atomically_and_round_trip_all_variants() {
    let mut b = behavior();
    let state = &mut b.states[0];
    let before = state.clone();
    assert!(edit_state_lists(state, "[]", "broken").is_err());
    assert_eq!(*state, before);
    let actions = "[move_toward_player(speed: 20.0), move_away_from_player(speed: 30.0), patrol(speed: 40.0), wait(seconds: 0.0), telegraph(seconds: 0.5)]";
    let transitions = r#"[(target: "idle", condition: timer_elapsed(0.0), invert: true),
        (target: "idle", condition: player_distance_less_than(20.0)),
        (target: "idle", condition: player_distance_greater_than(40.0)),
        (target: "idle", condition: health_below(2)),
        (target: "idle", condition: action_finished)]"#;
    edit_state_lists(state, actions, transitions).unwrap();
    assert_eq!(state.actions.len(), 5);
    assert_eq!(state.transitions.len(), 5);
    assert!(state.transitions[0].invert);
    assert_eq!(
        b,
        BehaviorDefinition::from_ron(&serialize(&b).unwrap()).unwrap()
    );
    edit_state_lists(&mut b.states[0], "[]", "[]").unwrap();
    b.validate().unwrap();
}

#[test]
fn save_reload_updates_entity_fields_behavior_and_all_renamed_references() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let path = editor.content.entity_path("patrol").unwrap();
    let entity = editor.content.entities.get_mut(&path).unwrap();
    entity.name = "renamed patrol".into();
    entity.health = 12;
    entity.size = (40.0, 42.0);
    entity.contact_damage = 3;
    entity.behavior = "chase".into();
    let b = editor.content.behaviors.get_mut("chase").unwrap();
    add_state(b, "custom".into()).unwrap();
    let index = b.states.len() - 1;
    edit_state_lists(
        &mut b.states[index],
        "[telegraph(seconds: 0.75), patrol(speed: 10.0)]",
        r#"[(target: "custom", condition: health_below(2), invert: true)]"#,
    )
    .unwrap();
    b.initial_state = "custom".into();
    let expected_behavior = b.clone();
    game.save_editor().unwrap();
    let mut reloaded = GameState::try_new(&assets.0).unwrap();
    reloaded.toggle_editor().unwrap();
    let editor = reloaded.editor().unwrap();
    let entity = &editor.content.entities[&path];
    assert_eq!(entity.name, "renamed patrol");
    assert_eq!(
        (
            entity.health,
            entity.size,
            entity.contact_damage,
            entity.behavior.as_str()
        ),
        (12, (40.0, 42.0), 3, "chase")
    );
    assert_eq!(editor.content.behaviors["chase"], expected_behavior);
    for entry in std::fs::read_dir(assets.0.join("rooms")).unwrap() {
        let document: RoomEnemies =
            ron::from_str(&std::fs::read_to_string(entry.unwrap().path()).unwrap()).unwrap();
        assert!(document.enemies.iter().all(|e| e.definition != "patrol"));
    }
    assert!(editor
        .draft
        .enemies
        .iter()
        .any(|e| e.definition == "renamed patrol"));
    reloaded.edit_room(RoomId::Second).unwrap();
    reloaded.save_editor().unwrap();
}

#[test]
fn invalid_entity_behavior_or_room_geometry_never_writes_any_file() {
    let assets = Assets::new();
    let before = assets.snapshot();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let path = game
        .editor()
        .unwrap()
        .content
        .entity_path("patrol")
        .unwrap();
    let original = game.editor().unwrap().content.entities[&path].clone();
    for invalid in ["health", "size", "behavior", "name", "room fit"] {
        let editor = game.editor_mut().unwrap();
        let mut entity = original.clone();
        match invalid {
            "health" => entity.health = 0,
            "size" => entity.size.0 = f32::NAN,
            "behavior" => entity.behavior = "missing".into(),
            "name" => entity.name = "chase".into(),
            _ => entity.size.1 = 10000.0,
        }
        editor.content.entities.insert(path.clone(), entity);
        assert!(game.save_editor().is_err(), "{invalid}");
        assert_eq!(assets.snapshot(), before);
    }
    let editor = game.editor_mut().unwrap();
    editor.content.entities.insert(path, original);
    let b = editor.content.behaviors.get_mut("patrol").unwrap();
    b.states[0].actions = vec![Action::Wait { seconds: -1.0 }];
    assert!(game.save_editor().is_err());
    assert_eq!(assets.snapshot(), before);
    let editor = game.editor_mut().unwrap();
    let b = editor.content.behaviors.get_mut("patrol").unwrap();
    b.states[0].actions.clear();
    b.states[0].transitions.push(Transition {
        target: "missing".into(),
        condition: Condition::TimerElapsed(0.0),
        invert: false,
    });
    assert!(game.save_editor().is_err());
    assert_eq!(assets.snapshot(), before);
}

#[test]
fn stale_temps_are_removed_and_staging_failures_clean_up() {
    use hollow_steps::definition_editor::write_files;
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    for entity in draft.entities.values_mut() {
        entity.health += 1;
    }
    let plan = draft.plan(&[]).unwrap();
    let before = assets.snapshot();
    let temp = plan[0].0.with_extension("ron.tmp");
    std::fs::write(&temp, "stale").unwrap();
    let blocked = plan[1].0.with_extension("ron.tmp");
    std::fs::create_dir(&blocked).unwrap();
    assert!(write_files(&plan).is_err());
    assert!(!temp.exists());
    std::fs::remove_dir(blocked).unwrap();
    assert_eq!(assets.snapshot(), before);
    std::fs::write(&temp, "stale").unwrap();
    write_files(&plan).unwrap();
    assert!(!temp.exists());
    GameState::try_new(&assets.0).unwrap();
}

#[test]
fn entity_fields_parse_all_properties_and_reject_bad_values() {
    use hollow_steps::definition_editor::entity_from_fields;
    let fields = ["new name", "255", "42.5", "40", "0", "chase"].map(str::to_owned);
    let entity = entity_from_fields(&fields).unwrap();
    assert_eq!(
        (
            entity.name.as_str(),
            entity.health,
            entity.size,
            entity.contact_damage,
            entity.behavior.as_str()
        ),
        ("new name", 255, (42.5, 40.0), 0, "chase")
    );
    for (index, value) in [
        (0, " "),
        (1, "256"),
        (1, "0"),
        (2, "NaN"),
        (3, "-1"),
        (4, "oops"),
        (5, ""),
    ] {
        let mut invalid = fields.clone();
        invalid[index] = value.into();
        assert!(entity_from_fields(&invalid).is_err());
    }
    let mut b = behavior();
    add_state(&mut b, "second".into()).unwrap();
    hollow_steps::definition_editor::set_initial_state(&mut b, 1).unwrap();
    assert_eq!(b.initial_state, "second");
    let before = b.clone();
    assert!(hollow_steps::definition_editor::set_initial_state(&mut b, 9).is_err());
    assert_eq!(b, before);
}

#[test]
fn structured_catalogs_round_trip_through_the_loader() {
    use hollow_steps::definition_editor::{ActionInput, StateInputs, TransitionInput};
    let mut b = behavior();
    let mut inputs = StateInputs::default();
    for kind in 0..Action::TYPES.len() {
        let input = ActionInput::new(kind).unwrap();
        assert_eq!(input.build().unwrap().kind(), kind);
        inputs.add_action(kind).unwrap();
    }
    for kind in 0..Condition::TYPES.len() {
        let input = TransitionInput::new("idle".into(), kind).unwrap();
        assert_eq!(
            input.build(&["idle".into()]).unwrap().condition.kind(),
            kind
        );
        inputs.add_transition("idle".into(), kind).unwrap();
    }
    inputs.apply(&mut b.states[0], &["idle".into()]).unwrap();
    let loaded = BehaviorDefinition::from_ron(&serialize(&b).unwrap()).unwrap();
    assert_eq!(loaded, b);
    assert!(ActionInput::new(Action::TYPES.len()).is_err());
    assert!(TransitionInput::new("idle".into(), Condition::TYPES.len()).is_err());
}

#[test]
fn structured_edits_add_remove_reorder_values_targets_and_inversion() {
    use hollow_steps::definition_editor::{move_row, remove_row, StateInputs};
    let mut b = behavior();
    add_state(&mut b, "attack".into()).unwrap();
    let names = vec!["idle".into(), "attack".into()];
    let mut inputs = StateInputs::from_state(&b.states[0]);
    let patrol = Action::TYPES.iter().position(|n| *n == "patrol").unwrap();
    inputs.add_action(patrol).unwrap();
    inputs.actions[1].values[0] = "42.5".into();
    move_row(&mut inputs.actions, 1, true).unwrap();
    assert!(move_row(&mut inputs.actions, 0, true).is_err());
    assert!(move_row(&mut inputs.actions, 1, false).is_err());
    assert!(move_row(&mut inputs.actions, usize::MAX, false).is_err());
    remove_row(&mut inputs.actions, 1).unwrap();
    assert!(remove_row(&mut inputs.actions, 1).is_err());
    inputs.add_transition("attack".into(), 0).unwrap();
    inputs.transitions[1].value = "0".into();
    inputs.transitions[1].invert = true;
    move_row(&mut inputs.transitions, 0, false).unwrap();
    inputs.apply(&mut b.states[0], &names).unwrap();
    assert_eq!(b.states[0].actions, vec![Action::Patrol { speed: 42.5 }]);
    assert_eq!(b.states[0].transitions[0].target, "attack");
    assert!(b.states[0].transitions[0].invert);
    assert_eq!(
        b.states[0].transitions[0].condition,
        Condition::TimerElapsed(0.0)
    );
    remove_row(&mut inputs.transitions, 1).unwrap();
    let finished = Condition::TYPES
        .iter()
        .position(|n| *n == "action_finished")
        .unwrap();
    inputs.transitions[0].change_kind(finished).unwrap();
    assert!(inputs.transitions[0].value.is_empty());
    inputs.apply(&mut b.states[0], &names).unwrap();
    assert_eq!(
        b.states[0].transitions[0].condition,
        Condition::ActionFinished
    );
    remove_row(&mut inputs.actions, 0).unwrap();
    remove_row(&mut inputs.transitions, 0).unwrap();
    inputs.apply(&mut b.states[0], &names).unwrap();
    assert!(b.states[0].actions.is_empty() && b.states[0].transitions.is_empty());
}

#[test]
fn invalid_structured_numbers_preserve_model_and_report_field_errors() {
    use hollow_steps::definition_editor::{ActionInput, StateInputs, TransitionInput};
    let mut b = behavior();
    let before = b.states[0].clone();
    let names = vec!["idle".into()];
    for kind in 0..Action::TYPES.len() {
        for text in ["", "no", "NaN", "inf", "-1", "1e99"] {
            let mut input = ActionInput::new(kind).unwrap();
            input.values[0] = text.into();
            assert!(input.build().is_err(), "{kind}: {text}");
            let inputs = StateInputs {
                actions: vec![input],
                transitions: vec![],
            };
            assert!(inputs.apply(&mut b.states[0], &names).is_err());
            assert_eq!(b.states[0], before);
        }
    }
    for kind in 0..Condition::TYPES.len() {
        let template = Condition::defaults(kind).unwrap();
        if template.parameter().is_none() {
            continue;
        }
        for text in ["", "NaN", "inf", "-1", "1e99"] {
            let mut row = TransitionInput::new("idle".into(), kind).unwrap();
            row.value = text.into();
            assert!(row.build(&names).is_err());
        }
        let mut row = TransitionInput::new("missing".into(), kind).unwrap();
        assert!(row.build(&names).is_err());
        row.target = "idle".into();
        row.value = "0".into();
        assert!(row.build(&names).is_ok());
    }
    let health = Condition::TYPES
        .iter()
        .position(|n| *n == "health_below")
        .unwrap();
    let mut row = TransitionInput::new("idle".into(), health).unwrap();
    for value in ["256", "1.5"] {
        row.value = value.into();
        assert!(row.build(&names).is_err());
    }
    row.value = "255".into();
    assert!(row.build(&names).is_ok());
}

#[test]
fn structured_save_then_reload_preserves_action_and_transition_order() {
    use hollow_steps::definition_editor::{move_row, StateInputs};
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let b = editor.content.behaviors.get_mut("patrol").unwrap();
    let names = b.states.iter().map(|s| s.name.clone()).collect::<Vec<_>>();
    let mut inputs = StateInputs::default();
    for kind in 0..Action::TYPES.len() {
        inputs.add_action(kind).unwrap();
    }
    move_row(&mut inputs.actions, 0, false).unwrap();
    inputs.actions[0].values[0] = "123.25".into();
    for kind in 0..Condition::TYPES.len() {
        inputs.add_transition(names[0].clone(), kind).unwrap();
    }
    inputs.transitions[0].invert = true;
    move_row(&mut inputs.transitions, 0, false).unwrap();
    inputs.apply(&mut b.states[0], &names).unwrap();
    let expected = b.clone();
    game.save_editor().unwrap();
    let loaded = DefinitionDraft::load(&assets.0).unwrap();
    assert_eq!(loaded.behaviors["patrol"], expected);
    GameState::try_new(&assets.0).unwrap();
}

#[test]
fn transition_validation_is_transactional_and_entity_errors_identify_each_field() {
    use hollow_steps::definition_editor::{validate_entity_field, StateInputs};
    let mut b = behavior();
    let before = b.states[0].clone();
    let mut inputs = StateInputs::from_state(&before);
    inputs.actions[0].values[0] = "99".into();
    inputs.transitions[0].target = "missing".into();
    assert!(inputs.apply(&mut b.states[0], &["idle".into()]).is_err());
    assert_eq!(b.states[0], before);
    for (index, invalid) in [
        (0, ""),
        (1, "0"),
        (1, "256"),
        (2, "NaN"),
        (3, "0"),
        (4, "-1"),
        (5, " "),
    ] {
        assert!(validate_entity_field(index, invalid).is_err());
    }
    for (index, valid) in [
        (0, "name"),
        (1, "255"),
        (2, "0.01"),
        (3, "100"),
        (4, "0"),
        (5, "patrol"),
    ] {
        assert!(validate_entity_field(index, valid).is_ok());
    }
}

#[test]
fn full_draft_validation_rejects_patrol_ranges_against_new_size_and_walls() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let before = assets.snapshot();
    let path = draft.entity_path("patrol").unwrap();
    draft.entities.get_mut(&path).unwrap().size.0 = 900.0;
    assert!(draft.plan(&[]).is_err());
    draft.entities.get_mut(&path).unwrap().size.0 = 32.0;
    let room = assets.0.join("rooms").join("second.ron");
    let mut document: RoomEnemies =
        ron::from_str(&std::fs::read_to_string(&room).unwrap()).unwrap();
    document.enemies[0].patrol_left = 0.0;
    let error = draft.plan(&[(room.clone(), document)]).unwrap_err();
    assert_error_file(&error, &room);
    assert!(error.contains("invalid patrol interval"));
    assert_eq!(assets.snapshot(), before);
}

#[test]
fn external_changes_refuse_save_and_identify_each_file_even_after_planning() {
    use hollow_steps::definition_editor::write_files;
    for (directory, filename) in [
        ("entities", "patrol.ron"),
        ("behaviors", "patrol.ron"),
        ("rooms", "first.ron"),
    ] {
        let assets = Assets::new();
        let draft = DefinitionDraft::load(&assets.0).unwrap();
        let plan = draft.plan(&[]).unwrap();
        let path = assets.0.join(directory).join(filename);
        let source = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{source}\n// external edit")).unwrap();
        let before = assets.snapshot();
        for error in [
            draft.plan(&[]).unwrap_err(),
            write_files(&plan).unwrap_err(),
        ] {
            assert_error_file(&error, &path);
            assert!(error.contains("changed on disk; reload before saving"));
        }
        assert_eq!(assets.snapshot(), before);
    }
}

#[test]
fn unchanged_parsed_content_keeps_bytes_and_mtime_and_changed_files_use_lf() {
    let assets = Assets::new();
    let path = assets.0.join("entities").join("patrol.ron");
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace('\n', "\r\n");
    std::fs::write(&path, source).unwrap();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let before = assets.snapshot();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    assert!(draft.plan(&[]).unwrap().is_empty());
    draft.commit(&[]).unwrap();
    assert_eq!(assets.snapshot(), before);
    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
    draft.entities.get_mut(&path).unwrap().health += 1;
    let plan = draft.plan(&[]).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].0, path);
    assert!(!plan[0].1.contains('\r'));
}

#[test]
fn plan_is_immutable_and_discovers_renames_in_every_room() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.entity_path("patrol").unwrap();
    draft.entities.get_mut(&path).unwrap().name = "new patrol".into();
    let room = assets.0.join("rooms").join("first.ron");
    let document: RoomEnemies = ron::from_str(&std::fs::read_to_string(&room).unwrap()).unwrap();
    let inputs = [(room, document)];
    let before = serialize(&inputs[0].1).unwrap();
    let plan = draft.plan(&inputs).unwrap();
    assert_eq!(serialize(&inputs[0].1).unwrap(), before);
    assert!(plan
        .iter()
        .any(|(p, s)| p.ends_with("second.ron") && s.contains("new patrol")));
    assert!(inputs[0].1.enemies.iter().any(|e| e.definition == "patrol"));
}

#[test]
fn commit_supports_second_rename_in_one_session() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.entity_path("patrol").unwrap();
    for name in ["first rename", "second rename"] {
        draft.entities.get_mut(&path).unwrap().name = name.into();
        draft.commit(&[]).unwrap();
        assert_eq!(draft.entity_path(name), Some(path.clone()));
        assert!(draft.plan(&[]).unwrap().is_empty());
        let document: RoomEnemies = ron::from_str(
            &std::fs::read_to_string(assets.0.join("rooms").join("second.ron")).unwrap(),
        )
        .unwrap();
        assert!(document.enemies.iter().any(|e| e.definition == name));
        GameState::try_new(&assets.0).unwrap();
    }
}

#[test]
fn input_trimming_blank_lines_and_health_message_are_consistent() {
    use hollow_steps::definition_editor::{
        entity_from_fields, validate_entity_field, ActionInput, TransitionInput,
    };
    let fields = [" name ", " 1 ", " 32 ", " 40 ", " 0 ", " patrol "].map(str::to_owned);
    let entity = entity_from_fields(&fields).unwrap();
    assert_eq!(
        (entity.name.as_str(), entity.behavior.as_str()),
        ("name", "patrol")
    );
    assert!(validate_entity_field(1, "0")
        .unwrap_err()
        .contains("1..255"));
    let mut fields = fields;
    fields[1] = "0".into();
    assert!(entity_from_fields(&fields).unwrap_err().contains("1..255"));
    let mut b = behavior();
    add_state(&mut b, " extra ".into()).unwrap();
    rename_state(&mut b, 1, " renamed ".into()).unwrap();
    assert_eq!(b.states[1].name, "renamed");
    let mut action = ActionInput::new(0).unwrap();
    action.values[0] = " 10 ".into();
    action.build().unwrap();
    let mut transition = TransitionInput::new(" idle ".into(), 0).unwrap();
    transition.value = " 1 ".into();
    assert_eq!(transition.build(&["idle".into()]).unwrap().target, "idle");
}

#[test]
fn blank_enemies_have_independent_editable_behaviors_and_survive_save() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let existing = draft.behaviors.clone();
    // Generated names must also skip behaviors created by the author.
    draft.add_behavior("blank_0", behavior()).unwrap();
    let first = draft.add_blank_entity(" first blank ").unwrap();
    let second = draft.add_blank_entity("second blank").unwrap();
    let first_behavior = draft.entities[&first].behavior.clone();
    let second_behavior = draft.entities[&second].behavior.clone();
    assert_ne!(first_behavior, "blank_0");
    assert_ne!(first_behavior, second_behavior);
    assert_eq!(draft.behavior_users(&first_behavior), 1);
    assert_eq!(draft.behavior_users(&second_behavior), 1);
    assert_eq!(
        draft.behaviors[&first_behavior],
        BehaviorDefinition::blank()
    );
    let original_blank = draft.behaviors[&second_behavior].clone();
    edit_state_lists(
        &mut draft.behaviors.get_mut(&first_behavior).unwrap().states[0],
        "[patrol(speed: 12.0)]",
        "[]",
    )
    .unwrap();
    assert_eq!(draft.behaviors[&second_behavior], original_blank);
    for (name, original) in existing {
        assert_eq!(draft.behaviors[&name], original);
    }
    draft.commit(&[]).unwrap();
    let reloaded = DefinitionDraft::load(&assets.0).unwrap();
    assert_eq!(reloaded.entities[&first].name, "first blank");
    assert_eq!(
        reloaded.behaviors[&first_behavior],
        draft.behaviors[&first_behavior]
    );
    assert_eq!(reloaded.behaviors[&second_behavior], original_blank);
    let bounds = hollow_steps::player::WorldBounds::new(0.0, 960.0, 460.0);
    let mut enemy = reloaded
        .definitions()
        .unwrap()
        .spawn("second blank", 300.0, 400.0, bounds)
        .unwrap();
    let position = enemy.position();
    for player_x in [position.x, position.x + 1.0, 900.0] {
        enemy.update(
            macroquad::math::Vec2::new(player_x, position.y),
            10.0,
            bounds,
        );
        assert_eq!(enemy.position(), position);
    }
}

#[test]
fn blank_creation_rejects_invalid_requests_without_partial_definitions() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let entities = draft.entities.clone();
    let behaviors = draft.behaviors.clone();
    for name in ["", "  ", "patrol", " patrol "] {
        assert!(draft.add_blank_entity(name).is_err());
        assert_eq!(draft.entities, entities);
        assert_eq!(draft.behaviors, behaviors);
    }
    let missing = assets.0.join("entities").join("missing.ron");
    assert!(draft.create_blank_behavior("new", Some(&missing)).is_err());
    assert_eq!(draft.behaviors, behaviors);
    // A completely empty catalog can still create the first enemy.
    draft.entities.clear();
    draft.behaviors.clear();
    let path = draft.add_blank_entity("only enemy").unwrap();
    assert_eq!(draft.entities.len(), 1);
    assert_eq!(draft.behaviors.len(), 1);
    let assigned = draft.entities[&path].behavior.clone();
    assert_eq!(draft.behaviors[&assigned], BehaviorDefinition::blank());
    draft
        .create_blank_behavior("replacement", Some(&path))
        .unwrap();
    assert_eq!(draft.entities[&path].behavior, "replacement");
    let before = draft.behaviors.clone();
    assert!(draft
        .create_blank_behavior("replacement", Some(&path))
        .is_err());
    assert!(draft.create_blank_behavior("../bad", Some(&path)).is_err());
    assert_eq!(draft.behaviors, before);
    assert_eq!(draft.entities[&path].behavior, "replacement");
}

#[test]
fn entity_and_behavior_operations_check_references_and_delete_files_on_save() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let original = draft.entity_path("patrol").unwrap();
    assert!(draft.delete_entity(&original).is_err());
    assert!(draft.delete_behavior("patrol").is_err());
    let duplicate = draft.duplicate_entity(&original, " copy ").unwrap();
    assert_eq!(draft.entity_path("copy"), Some(duplicate.clone()));
    assert!(draft.duplicate_entity(&original, "copy").is_err());
    draft.add_behavior(" extra ", behavior()).unwrap();
    assert!(draft.add_behavior("extra", behavior()).is_err());
    draft.entities.get_mut(&duplicate).unwrap().behavior = "extra".into();
    assert!(draft.delete_behavior("extra").is_err());
    draft.rename_behavior("extra", " renamed ").unwrap();
    assert_eq!(draft.entities[&duplicate].behavior, "renamed");
    draft.commit(&[]).unwrap();
    assert!(duplicate.exists());
    assert!(!assets.0.join("behaviors").join("extra.ron").exists());
    assert!(assets.0.join("behaviors").join("renamed.ron").exists());
    draft.delete_entity(&duplicate).unwrap();
    assert_eq!(draft.entity_path("copy"), None);
    draft.delete_behavior("renamed").unwrap();
    draft.commit(&[]).unwrap();
    assert!(!duplicate.exists());
    assert!(!assets.0.join("behaviors").join("renamed.ron").exists());
    assert!(draft.delete_entity(&duplicate).is_err());
    assert!(draft.delete_behavior("missing").is_err());
    GameState::try_new(&assets.0).unwrap();
}

#[test]
fn deleting_entity_checks_unsaved_room_references() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let original = editor.content.entity_path("patrol").unwrap();
    let duplicate = editor.content.duplicate_entity(&original, "copy").unwrap();
    editor
        .draft
        .enemies
        .push(hollow_steps::room::EnemyPlacement {
            definition: "copy".into(),
            patrol_left: 300.0,
            patrol_right: 400.0,
        });
    let error = editor.delete_entity(&duplicate).unwrap_err();
    assert_error_file(&error, &editor.path);
    assert!(error.contains("referenced"));
    assert!(editor.content.entities.contains_key(&duplicate));
    editor.draft.enemies.pop();
    editor.delete_entity(&duplicate).unwrap();
    assert!(!editor.content.entities.contains_key(&duplicate));
}

#[test]
fn new_entity_edits_keep_preview_and_unsaved_references_valid() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let original = editor.content.entity_path("patrol").unwrap();
    let mut entity = editor.content.entities[&original].clone();
    entity.name = "new entity".into();
    let path = editor.content.add_entity(entity.clone()).unwrap();
    assert_eq!(editor.content.entity_path("new entity"), Some(path.clone()));
    editor.definitions = editor.content.definitions().unwrap();
    editor
        .draft
        .enemies
        .push(hollow_steps::room::EnemyPlacement {
            definition: entity.name.clone(),
            patrol_left: 300.0,
            patrol_right: 400.0,
        });
    entity.name = "renamed entity".into();
    editor.update_entity(&path, entity.clone()).unwrap();
    assert_eq!(editor.draft.enemies.last().unwrap().definition, entity.name);
    assert_eq!(editor.content.entity_path(&entity.name), Some(path.clone()));
    assert!(editor.definitions.size(&entity.name).is_ok());
    assert!(editor
        .delete_entity(&path)
        .unwrap_err()
        .contains("referenced"));

    let mut invalid = entity.clone();
    invalid.name = "patrol".into();
    assert!(editor.update_entity(&path, invalid).is_err());
    assert_eq!(editor.content.entities[&path], entity);
    assert_eq!(editor.draft.enemies.last().unwrap().definition, entity.name);
    assert!(editor.definitions.size(&entity.name).is_ok());
    game.save_editor().unwrap();
    let loaded = GameState::try_new(&assets.0).unwrap();
    assert!(loaded
        .room()
        .enemies
        .iter()
        .any(|enemy| enemy.name() == entity.name));
}

#[test]
fn removing_authored_layout_uses_loader_defaults_and_custom_rooms_are_discovered() {
    use hollow_steps::editor::{RoomDraft, RoomLayout};
    let assets = Assets::new();
    let rooms = hollow_steps::room::load_rooms(&assets.0).unwrap();
    let path = assets.0.join("rooms").join("second.ron");
    let mut document: RoomEnemies =
        ron::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let mut layout = RoomLayout::from_room(&rooms[1]);
    layout.bounds.0 = 0.0;
    layout.bounds.2 = 1000.0;
    document.layout = Some(layout);
    document.enemies[0].patrol_left = 50.0;
    std::fs::write(&path, serialize(&document).unwrap()).unwrap();
    let draft = DefinitionDraft::load(&assets.0).unwrap();
    document.layout = None;
    assert!(draft.plan(&[(path, document.clone())]).is_err());
    // A room not supplied by the caller still gets both validation and rename updates.
    let custom = assets.0.join("rooms").join("room_42.ron");
    document.layout = Some(RoomLayout::from_room(&rooms[0]));
    document.layout.as_mut().unwrap().exits.clear();
    std::fs::write(&custom, serialize(&document).unwrap()).unwrap();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let entity = draft.entity_path("patrol").unwrap();
    draft.entities.get_mut(&entity).unwrap().name = "renamed".into();
    draft.commit(&[]).unwrap();
    let saved: RoomEnemies = ron::from_str(&std::fs::read_to_string(&custom).unwrap()).unwrap();
    assert!(saved.enemies.iter().any(|e| e.definition == "renamed"));
    RoomDraft {
        enemies: saved.enemies,
        boss: saved.boss,
        layout: saved.layout.unwrap(),
    }
    .validate_local(&draft.definitions().unwrap())
    .unwrap();
}

#[test]
fn missing_required_rooms_and_new_external_files_refuse_saving() {
    use hollow_steps::definition_editor::write_files;
    let assets = Assets::new();
    let draft = DefinitionDraft::load(&assets.0).unwrap();
    let plan = draft.plan(&[]).unwrap();
    let path = assets.0.join("behaviors").join("external.ron");
    std::fs::write(&path, serialize(&behavior()).unwrap()).unwrap();
    assert!(write_files(&plan).unwrap_err().contains("external.ron"));
    assert!(draft.plan(&[]).unwrap_err().contains("external.ron"));
    std::fs::remove_file(assets.0.join("rooms").join("first.ron")).unwrap();
    let draft = DefinitionDraft::load(&assets.0).unwrap();
    assert!(draft.plan(&[]).unwrap_err().contains("first.ron"));
}

#[test]
fn parsed_float_equality_does_not_rewrite_signed_zero() {
    let assets = Assets::new();
    let path = assets.0.join("behaviors").join("patrol.ron");
    let mut b = behavior();
    b.states[0].actions = vec![Action::Wait { seconds: -0.0 }];
    std::fs::write(&path, serialize(&b).unwrap()).unwrap();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    draft.behaviors.get_mut("patrol").unwrap().states[0].actions =
        vec![Action::Wait { seconds: 0.0 }];
    assert!(draft.plan(&[]).unwrap().is_empty());
}

#[test]
fn behavior_filenames_are_portable_to_windows() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    for name in [
        "CON",
        "con.txt",
        "AUX",
        "NUL",
        "PRN",
        "COM1",
        "lpt9.ron",
        "COM¹",
        "CONIN$",
        "CONOUT$",
        "bad<",
        "bad>",
        "bad:",
        "bad\"",
        "bad/",
        "bad\\",
        "bad|",
        "bad?",
        "bad*",
        "bad\0",
        "bad\nname",
        "bad.",
        ".",
        "..",
    ] {
        assert!(draft.add_behavior(name, behavior()).is_err(), "{name:?}");
    }
    for name in ["context", "com10", "lpt0", "état", "valid_name-2"] {
        draft.add_behavior(name, behavior()).unwrap();
    }
}

#[test]
fn unsaved_entity_size_and_behavior_changes_apply_to_original_placement_names() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.entity_path("patrol").unwrap();
    let entity = draft.entities.get_mut(&path).unwrap();
    entity.name = "renamed".into();
    entity.size = (60.0, 42.0);
    let definitions = draft.placement_definitions().unwrap();
    assert_eq!(definitions.size("patrol").unwrap().x, 60.0);
    assert_eq!(definitions.size("renamed").unwrap().x, 60.0);
    assert_eq!(draft.entity_path("renamed"), Some(path));
}

#[test]
fn boss_save_reload_round_trip_updates_properties_actions_names_and_runtime() {
    use hollow_steps::boss_definition::AttackSelection;
    use hollow_steps::boss_editor::BossInputs;
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.edit_room(RoomId::Boss).unwrap();
    let editor = game.editor_mut().unwrap();
    assert!(editor
        .content
        .placement_catalog()
        .iter()
        .any(|(label, p)| label == "Boss: Ash Warden"
            && matches!(p, hollow_steps::editor::Placement::Boss(_))));
    let path = editor.content.boss_path("Ash Warden").unwrap();
    let mut input = BossInputs::from_definition(&editor.content.bosses[&path]);
    input.fields = [
        "Edited Warden".into(),
        "47".into(),
        "40".into(),
        "42".into(),
        "3".into(),
        "0.25".into(),
    ];
    input.selection = AttackSelection::Random;
    input.rename_attack(0, "Edited Slam").unwrap();
    input.move_attack(2, true).unwrap();
    let added = input.add_attack("New attack").unwrap();
    let shockwave = Action::ALL_TYPES
        .iter()
        .position(|n| *n == "spawn_shockwave")
        .unwrap();
    input.add_action(added, shockwave).unwrap();
    input.attacks[added].actions[0].values = vec!["123".into(), "5".into()];
    let expected = input.build().unwrap();
    editor.update_boss(&path, expected.clone()).unwrap();
    editor
        .move_selection(
            hollow_steps::editor::Selection::Boss,
            macroquad::math::vec2(600.0, 400.0),
        )
        .unwrap();
    game.save_editor().unwrap();
    let runtime = game.room().boss_special.as_ref().unwrap();
    assert_eq!(runtime.max_health(), 47);
    assert_eq!(runtime.contact_damage(), 3);
    assert_eq!(runtime.size(), macroquad::math::vec2(40.0, 42.0));
    assert_eq!(runtime.position(), macroquad::math::vec2(600.0, 400.0));
    let mut reloaded = GameState::try_new(&assets.0).unwrap();
    reloaded.edit_room(RoomId::Boss).unwrap();
    let editor = reloaded.editor().unwrap();
    assert_eq!(editor.content.bosses[&path], expected);
    assert_eq!(
        editor.draft.boss.as_ref().unwrap().definition,
        "Edited Warden"
    );
    assert_eq!(editor.draft.boss.as_ref().unwrap().position, (600.0, 400.0));
    // Snapshot refresh permits another rename and a content-only no-op save.
    let editor = game.editor_mut().unwrap();
    let mut boss = editor.content.bosses[&path].clone();
    boss.name = "Twice renamed".into();
    editor.update_boss(&path, boss).unwrap();
    game.save_editor().unwrap();
    let before = assets.snapshot();
    assert!(game.editor().unwrap().content.plan(&[]).unwrap().is_empty());
    game.save_editor().unwrap();
    assert_eq!(assets.snapshot(), before);
}

#[test]
fn invalid_boss_definitions_placements_and_disk_conflicts_block_all_writes() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.boss_path("Ash Warden").unwrap();
    draft.entities.values_mut().next().unwrap().health = 17;
    let before = assets.snapshot();
    let original = draft.bosses[&path].clone();
    draft.bosses.get_mut(&path).unwrap().health = 0;
    assert!(draft.commit(&[]).is_err());
    assert_eq!(assets.snapshot(), before);
    draft.bosses.insert(path.clone(), original);
    draft.bosses.get_mut(&path).unwrap().size.1 = 5000.0;
    let error = draft.commit(&[]).unwrap_err();
    assert_error_file(&error, &assets.0.join("rooms").join("boss.ron"));
    assert_eq!(assets.snapshot(), before);
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    draft.bosses.get_mut(&path).unwrap().health = 77;
    let plan = draft.plan(&[]).unwrap();
    std::fs::write(&path, serialize(&draft.bosses[&path]).unwrap()).unwrap();
    let changed = assets.snapshot();
    assert_error_file(
        &hollow_steps::definition_editor::write_files(&plan).unwrap_err(),
        &path,
    );
    assert_eq!(assets.snapshot(), changed);
}

#[test]
fn boss_edits_from_another_room_apply_and_deletion_and_replacement_survive_reload() {
    use hollow_steps::editor::{Placement, Selection};
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let path = editor.content.boss_path("Ash Warden").unwrap();
    let mut boss = editor.content.bosses[&path].clone();
    boss.health = 62;
    boss.name = "Shared rename".into();
    editor.update_boss(&path, boss).unwrap();
    game.save_editor().unwrap();
    game.edit_room(RoomId::Boss).unwrap();
    assert_eq!(game.room().boss_special.as_ref().unwrap().max_health(), 62);
    assert_eq!(
        game.editor()
            .unwrap()
            .draft
            .boss
            .as_ref()
            .unwrap()
            .definition,
        "Shared rename"
    );
    game.editor_mut()
        .unwrap()
        .draft
        .delete(Selection::Boss)
        .unwrap();
    game.save_editor().unwrap();
    assert!(game.room().boss_special.is_none());
    let mut reloaded = GameState::try_new(&assets.0).unwrap();
    reloaded.edit_room(RoomId::Boss).unwrap();
    assert!(reloaded.editor().unwrap().draft.boss.is_none());
    reloaded
        .editor_mut()
        .unwrap()
        .place(
            Placement::Boss("Ground Pound Example".into()),
            macroquad::math::vec2(500.0, 400.0),
        )
        .unwrap();
    reloaded.save_editor().unwrap();
    assert_eq!(
        reloaded.room().boss_special.as_ref().unwrap().max_health(),
        10
    );
    let mut final_game = GameState::try_new(&assets.0).unwrap();
    final_game.edit_room(RoomId::Boss).unwrap();
    assert_eq!(
        final_game
            .editor()
            .unwrap()
            .draft
            .boss
            .as_ref()
            .unwrap()
            .definition,
        "Ground Pound Example"
    );
}

#[test]
fn boss_staging_failures_and_new_external_boss_files_preserve_the_transaction() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.boss_path("Ash Warden").unwrap();
    draft.bosses.get_mut(&path).unwrap().health = 81;
    draft.entities.values_mut().next().unwrap().health = 19;
    let blocked = path.with_extension("ron.tmp");
    std::fs::create_dir(&blocked).unwrap();
    // Snapshot only authored RON files, excluding the deliberately blocked staging path.
    let before = std::fs::read(&path).unwrap();
    assert!(draft.commit(&[]).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(draft
        .entities
        .keys()
        .all(|p| !p.with_extension("ron.tmp").exists()));
    std::fs::remove_dir(&blocked).unwrap();
    draft.commit(&[]).unwrap();
    let plan = draft.plan(&[]).unwrap();
    let mut external = draft.bosses[&path].clone();
    external.name = "External boss".into();
    let external_path = assets.0.join("bosses").join("external.ron");
    std::fs::write(&external_path, serialize(&external).unwrap()).unwrap();
    assert_error_file(
        &hollow_steps::definition_editor::write_files(&plan).unwrap_err(),
        &external_path,
    );
}

#[test]
fn unrelated_editor_saves_preserve_damaged_boss_progress() {
    let assets = Assets::new();
    let game = GameState::try_new(&assets.0).unwrap();
    let spawn = game.room().spawn;
    let room_path = assets.0.join("rooms").join("first.ron");
    let mut document: RoomEnemies =
        ron::from_str(&std::fs::read_to_string(&room_path).unwrap()).unwrap();
    document.boss = Some(hollow_steps::room::BossPlacement {
        definition: "Ground Pound Example".into(),
        position: (spawn.x + 40.0, spawn.y),
    });
    std::fs::write(room_path, serialize(&document).unwrap()).unwrap();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(
        hollow_steps::player::PlayerInput {
            attack_pressed: true,
            ..Default::default()
        },
        0.0,
    );
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 9);
    game.toggle_editor().unwrap();
    game.save_editor().unwrap();
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 9);
    game.edit_room(RoomId::Second).unwrap();
    game.save_editor().unwrap();
    game.edit_room(RoomId::First).unwrap();
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 9);
}

#[test]
fn attack_validation_blocks_staged_saves_without_any_disk_changes() {
    let assets = Assets::new();
    let snapshot = assets.snapshot();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    for (name, field) in [
        ("lunge", "duration"),
        ("jump_to", "air time"),
        ("slam_down", "damage"),
        ("shoot_projectile", "count"),
        ("area_blast", "radius"),
    ] {
        let kind = Action::TYPES
            .iter()
            .position(|entry| *entry == name)
            .unwrap();
        let mut action = Action::defaults(kind).unwrap();
        match &mut action {
            Action::Lunge { duration_secs, .. } => *duration_secs = -1.0,
            Action::JumpTo { air_time_secs, .. } => *air_time_secs = -1.0,
            Action::ImpactSlam { attack, .. } => attack.damage = -1,
            Action::ShootProjectile { count, .. } => *count = 0,
            Action::RadialBlast { radius, .. } => *radius = 0.0,
            _ => unreachable!(),
        }
        draft.behaviors.get_mut("patrol").unwrap().states[0].actions = vec![action];
        let error = draft.plan(&[]).unwrap_err();
        assert_error_file(&error, &assets.0.join("behaviors").join("patrol.ron"));
        assert!(
            error.contains("action 0") && error.contains(field),
            "{error}"
        );
        assert_eq!(assets.snapshot(), snapshot);
    }
}
