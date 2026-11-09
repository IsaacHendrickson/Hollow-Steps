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
        for directory in ["entities", "behaviors", "rooms"] {
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
        for dir in ["entities", "behaviors", "rooms"] {
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
fn panel_fits_viewport_and_reserves_save_outside_scroll_bounds() {
    use hollow_steps::definition_editor::{wrap_label, PanelLayout};
    for (width, height) in [
        (960.0, 540.0),
        (1920.0, 1080.0),
        (360.0, 240.0),
        (200.0, 100.0),
        (16.0, 20.0),
        (0.0, 0.0),
    ] {
        let layout = PanelLayout::new(width, height);
        assert!(layout.panel.x >= 0.0 && layout.panel.right() <= width);
        assert_eq!(layout.panel.w, width.min(360.0));
        assert_eq!(layout.save.bottom(), height);
        assert_eq!(layout.body.bottom(), layout.save.y);
        assert!(layout.save.h <= height && layout.body.h >= 0.0);
        assert_eq!(layout.scroll_bounds(2000.0, -1.0), 0.0);
        assert_eq!(
            layout.scroll_bounds(2000.0, f32::MAX),
            (2000.0 - layout.body.h).max(0.0)
        );
        assert_eq!(layout.scroll_bounds(0.0, 100.0), 0.0);
    }
    let label = "player_distance_greater_than/long/path/ü";
    let lines = wrap_label(label, 8);
    assert_eq!(lines.concat(), label);
    assert!(lines.iter().all(|s| s.chars().count() <= 8));
    assert_eq!(wrap_label("ab", 0), ["a", "b"]);
    let invalid = PanelLayout::new(f32::NAN, f32::INFINITY);
    assert_eq!(invalid.panel.w, 0.0);
    assert_eq!(invalid.panel.h, 0.0);
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
        entity_from_fields, validate_entity_field, wrap_label, ActionInput, TransitionInput,
    };
    assert_eq!(wrap_label("\na\n\nü\n", 1), ["", "a", "", "ü", ""]);
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
fn entity_and_behavior_operations_check_references_and_delete_files_on_save() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let original = draft.entity_path("patrol").unwrap();
    assert!(draft.delete_entity(&original).is_err());
    assert!(draft.delete_behavior("patrol").is_err());
    let duplicate = draft.duplicate_entity(&original, " copy ").unwrap();
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
    draft.delete_behavior("renamed").unwrap();
    draft.commit(&[]).unwrap();
    assert!(!duplicate.exists());
    assert!(!assets.0.join("behaviors").join("renamed.ron").exists());
    assert!(draft.delete_entity(&duplicate).is_err());
    assert!(draft.delete_behavior("missing").is_err());
    GameState::try_new(&assets.0).unwrap();
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
fn panel_scroll_clamps_on_resize_and_content_changes() {
    use hollow_steps::definition_editor::PanelLayout;
    let small = PanelLayout::new(960.0, 240.0);
    let bottom = small.scroll_bounds(1000.0, f32::INFINITY);
    assert_eq!(bottom, 1000.0 - small.body.h);
    assert_eq!(small.scroll_bounds(1000.0, 75.0), 75.0);
    assert_eq!(small.scroll_bounds(1000.0, f32::NEG_INFINITY), 0.0);
    assert_eq!(small.scroll_bounds(1000.0, f32::NAN), 0.0);
    assert_eq!(small.scroll_bounds(10.0, bottom), 0.0);
    let large = PanelLayout::new(960.0, 1500.0);
    assert_eq!(large.scroll_bounds(1000.0, bottom), 0.0);
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
