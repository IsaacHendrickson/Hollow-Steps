mod support;

use hollow_steps::behavior::{Action, BehaviorDefinition, Condition, Transition};
use hollow_steps::definition_editor::{
    add_state, edit_state_lists, remove_state, rename_state, serialize, DefinitionDraft,
};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEnemies, RoomId};
use std::path::PathBuf;
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
fn pure_save_plan_does_not_touch_files_and_staging_failure_preserves_them() {
    let assets = Assets::new();
    let before = assets.snapshot();
    let draft = DefinitionDraft::load(&assets.0).unwrap();
    let plan = draft.plan(&mut []).unwrap();
    assert_eq!(assets.snapshot(), before);
    let occupied = plan[1].0.with_extension("ron.tmp");
    std::fs::write(&occupied, "another writer").unwrap();
    assert!(hollow_steps::definition_editor::write_files(&plan).is_err());
    assert_eq!(
        std::fs::read_to_string(&occupied).unwrap(),
        "another writer"
    );
    std::fs::remove_file(occupied).unwrap();
    assert_eq!(assets.snapshot(), before);
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
