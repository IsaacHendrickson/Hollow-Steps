mod support;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::game::GameState;
use hollow_steps::player_editor::{PlayerDraft, SpawnCheck, MAX_JUMP_APEX};
use hollow_steps::terrain::Terrain;
use hollow_steps::tuning::{Field, TuningConfig, FILE_NAME};
use macroquad::math::{Rect, Vec2};

/// A private copy of the fixture assets, so Save never touches the repository.
struct Assets(PathBuf);

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-player-editor-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(&support::fixture_assets(), &path);
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }

    fn player_file(&self) -> PathBuf {
        self.0.join(FILE_NAME)
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// A room whose spawn is at x = 100 with a solid starting at x = 120: a 32 px player (right edge
/// 116) fits, a 64 px one (right edge 132) starts inside it.
fn tower() -> SpawnCheck {
    SpawnCheck {
        room: "Tower".into(),
        spawn: Vec2::new(100.0, 200.0),
        terrain: Terrain::new(vec![Rect::new(120.0, 100.0, 40.0, 200.0)]),
    }
}

fn open_room() -> SpawnCheck {
    SpawnCheck {
        room: "Meadow".into(),
        spawn: Vec2::new(100.0, 200.0),
        terrain: Terrain::default(),
    }
}

#[test]
fn a_draft_is_clean_until_a_field_differs_and_clean_again_after_reset_or_save() {
    let mut draft = PlayerDraft::default();
    assert!(!draft.is_dirty());
    assert_eq!(draft.saved(), draft.edited());

    draft.set_field(Field::JumpSpeed, 500.0).unwrap();
    assert!(draft.is_dirty());
    assert!(draft.is_field_dirty(Field::JumpSpeed));
    assert!(!draft.is_field_dirty(Field::MoveSpeed));
    assert_eq!(draft.edited().player.jump_speed, 500.0);
    assert_eq!(
        draft.saved().player.jump_speed,
        TuningConfig::default().player.jump_speed
    );

    draft.reset_field(Field::JumpSpeed);
    assert!(!draft.is_dirty());

    draft.set_field(Field::DashSpeed, 900.0).unwrap();
    draft.set_field(Field::Width, 30.0).unwrap();
    draft.reset_field(Field::DashSpeed);
    assert!(draft.is_dirty(), "the width is still edited");
    assert!(!draft.is_field_dirty(Field::DashSpeed));
    draft.reset_all();
    assert!(!draft.is_dirty());
    assert_eq!(draft, PlayerDraft::default());

    draft.set_field(Field::Gravity, 1500.0).unwrap();
    draft.mark_saved();
    assert!(!draft.is_dirty());
    assert_eq!(draft.saved().player.gravity, 1500.0);
}

#[test]
fn every_field_rejects_values_outside_its_range_and_accepts_the_ends() {
    for field in Field::ALL {
        let spec = field.spec();
        for bad in [
            spec.min - spec.step.max(0.5),
            spec.max + spec.step.max(0.5),
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
        ] {
            let mut draft = PlayerDraft::default();
            let error = draft.set_field(field, bad).unwrap_err();
            assert!(error.contains(spec.key), "{bad}: {error}");
            assert!(!draft.is_dirty(), "{} took {bad}", spec.key);
        }
        for good in [spec.min, spec.max] {
            let mut draft = PlayerDraft::default();
            draft.set_field(field, good).unwrap();
            assert_eq!(field.get(draft.edited()), good, "{}", spec.key);
        }
    }
}

#[test]
fn a_whole_number_field_rounds_what_it_is_given() {
    let mut draft = PlayerDraft::default();
    draft.set_field(Field::EnemyDefaultHealth, 12.4).unwrap();
    assert_eq!(draft.edited().enemy.default_health, 12);
}

#[test]
fn the_shipped_defaults_have_no_problems() {
    let draft = PlayerDraft::default();
    assert!(draft.problems(&[open_room(), tower()]).is_empty());
    draft.validate(&[open_room(), tower()]).unwrap();
}

#[test]
fn a_held_jump_must_end_and_stay_under_the_height_limit() {
    let mut draft = PlayerDraft::default();
    draft
        .set_field(Field::HeldJumpGravityMultiplier, 0.0)
        .unwrap();
    let problems = draft.problems(&[]);
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("never end"), "{problems:?}");
    for field in [
        Field::JumpSpeed,
        Field::Gravity,
        Field::HeldJumpGravityMultiplier,
    ] {
        assert!(draft.problem_for(field, &[]).is_some(), "{field:?}");
    }
    assert!(draft.problem_for(Field::MoveSpeed, &[]).is_none());
    assert!(draft.validate(&[]).is_err());

    // Each number is in its own range, but together the jump rises too high.
    let mut draft = PlayerDraft::default();
    draft.set_field(Field::JumpSpeed, 1_500.0).unwrap();
    let message = draft.validate(&[]).unwrap_err();
    assert!(message.contains("a held jump would rise"), "{message}");
    assert!(
        message.contains(&format!("{MAX_JUMP_APEX:.0}")),
        "{message}"
    );
    // More gravity brings it back under the limit.
    draft.set_field(Field::Gravity, 3_000.0).unwrap();
    draft.validate(&[]).unwrap();
}

#[test]
fn a_size_that_starts_inside_a_solid_is_rejected_naming_the_room() {
    let mut draft = PlayerDraft::default();
    draft.set_field(Field::Width, 64.0).unwrap();
    let spawns = [open_room(), tower()];
    let message = draft.validate(&spawns).unwrap_err();
    assert!(message.contains("Tower"), "{message}");
    assert!(!message.contains("Meadow"), "{message}");
    assert!(draft.problem_for(Field::Width, &spawns).is_some());
    assert!(draft.problem_for(Field::Height, &spawns).is_some());
    assert!(draft.problem_for(Field::JumpSpeed, &spawns).is_none());
    // Without that room the same size is fine, and so is a narrower one.
    draft.validate(&[open_room()]).unwrap();
    draft.set_field(Field::Width, 36.0).unwrap();
    draft.validate(&spawns).unwrap();
}

#[test]
fn save_writes_the_edited_values_and_they_load_back() {
    let assets = Assets::new();
    let mut draft = PlayerDraft::default();
    draft.set_field(Field::JumpSpeed, 500.0).unwrap();
    draft.set_field(Field::DashCooldown, 0.75).unwrap();
    assert!(!assets.player_file().exists());

    let path = draft.save(&assets.0, &[open_room()]).unwrap();
    assert_eq!(path, assets.player_file());
    assert!(!draft.is_dirty());
    let loaded = TuningConfig::load(&assets.0).unwrap();
    assert_eq!(loaded, *draft.edited());
    assert_eq!(loaded.player.jump_speed, 500.0);
    assert!(!assets.0.join("player.ron.tmp").exists());
    // The shipped file is not touched: only the override in the assets folder is written.
    assert_eq!(
        TuningConfig::parse(include_str!("../data/player.ron"), "data").unwrap(),
        TuningConfig::default()
    );
}

#[test]
fn an_invalid_draft_writes_nothing() {
    let assets = Assets::new();
    let mut draft = PlayerDraft::default();
    draft.set_field(Field::Width, 64.0).unwrap();
    let error = draft.save(&assets.0, &[tower()]).unwrap_err();
    assert!(error.contains("Tower"), "{error}");
    assert!(!assets.player_file().exists());
    assert!(draft.is_dirty(), "a refused save leaves the draft unsaved");

    // An existing file is left exactly as it was.
    let mut ok = PlayerDraft::default();
    ok.set_field(Field::MoveSpeed, 300.0).unwrap();
    ok.save(&assets.0, &[open_room()]).unwrap();
    let before = std::fs::read(assets.player_file()).unwrap();
    let mut bad = PlayerDraft::new(*ok.saved());
    bad.set_field(Field::HeldJumpGravityMultiplier, 0.0)
        .unwrap();
    assert!(bad.save(&assets.0, &[open_room()]).is_err());
    assert_eq!(std::fs::read(assets.player_file()).unwrap(), before);
}

/// The names of every number in the file, as `section.field`, read from the text the editor
/// writes for the default values.
fn keys_in_default_file() -> BTreeSet<String> {
    let text = TuningConfig::default().to_ron().unwrap();
    let value: ron::Value = ron::from_str(&text).unwrap();
    let ron::Value::Map(sections) = value else {
        panic!("the file is a struct: {text}");
    };
    let mut keys = BTreeSet::new();
    for (section, fields) in sections.iter() {
        let (ron::Value::String(section), ron::Value::Map(fields)) = (section, fields) else {
            panic!("unexpected shape in {text}");
        };
        for (field, _) in fields.iter() {
            let ron::Value::String(field) = field else {
                panic!("unexpected field name in {text}");
            };
            keys.insert(format!("{section}.{field}"));
        }
    }
    keys
}

#[test]
fn every_number_in_the_file_has_a_table_entry_and_the_other_way_round() {
    let table: BTreeSet<String> = Field::ALL
        .iter()
        .map(|field| field.spec().key.to_owned())
        .collect();
    let file = keys_in_default_file();
    assert!(file.len() > 10, "{file:?}");
    let missing: Vec<_> = file.difference(&table).collect();
    assert!(
        missing.is_empty(),
        "written to player.ron but not in the field table (tuning_fields.rs): {missing:?}"
    );
    let unused: Vec<_> = table.difference(&file).collect();
    assert!(
        unused.is_empty(),
        "in the field table but not written to player.ron: {unused:?}"
    );
}

#[test]
fn the_game_applies_an_edit_at_once_and_refuses_a_save_that_breaks_a_rule() {
    let assets = Assets::new();
    let mut game = assets.game();
    game.set_tuning_field(Field::Width, 36.0).unwrap();
    assert_eq!(game.player().size().x, 36.0);
    assert!(game.tuning_unsaved());
    assert!(game.player_draft().is_dirty());
    assert!(game
        .set_tuning_field(Field::Width, 100_000.0)
        .unwrap_err()
        .contains("player.width"));
    assert_eq!(game.player().size().x, 36.0);

    // Every room the game has is checked.
    let spawns = game.spawn_checks();
    assert_eq!(spawns.len(), 3);

    // A combination that only fails across fields is applied (and shown), but never saved.
    game.set_tuning_field(Field::HeldJumpGravityMultiplier, 0.0)
        .unwrap();
    let error = game.save_tuning().unwrap_err();
    assert!(error.contains("never end"), "{error}");
    assert!(!assets.player_file().exists());
    assert!(game.tuning_unsaved());

    game.set_tuning_field(Field::HeldJumpGravityMultiplier, 0.5)
        .unwrap();
    game.save_tuning().unwrap();
    assert!(!game.tuning_unsaved());
    assert_eq!(
        TuningConfig::load(&assets.0).unwrap().player.size.x,
        36.0,
        "the saved file has the new width"
    );
}

#[test]
fn the_editor_session_edits_a_copy_that_the_game_takes_into_effect() {
    let assets = Assets::new();
    let mut game = assets.game();
    game.set_tuning_field(Field::MoveSpeed, 250.0).unwrap();
    game.toggle_editor().unwrap();
    // The session starts from the game's draft, F4 edits included.
    assert_eq!(game.editor().unwrap().player, *game.player_draft());
    assert!(game.editor().unwrap().player.is_dirty());

    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 520.0).unwrap();
    assert_eq!(game.tuning().player.jump_speed, 480.0, "not yet in effect");
    game.sync_player_from_editor();
    assert_eq!(game.tuning().player.jump_speed, 520.0);
    assert_eq!(game.player().config().jump_speed, 520.0);
    assert_eq!(game.tuning().player.move_speed, 250.0);

    // Save goes through the game and leaves both copies clean and equal.
    game.save_tuning().unwrap();
    assert!(!game.tuning_unsaved());
    let session = game.editor().unwrap();
    assert!(!session.player.is_dirty());
    assert_eq!(session.player, *game.player_draft());
    let saved = TuningConfig::load(&assets.0).unwrap();
    assert_eq!(saved.player.jump_speed, 520.0);
    assert_eq!(saved.player.move_speed, 250.0);
}

#[test]
fn the_editor_checks_the_size_against_the_room_as_it_is_being_edited() {
    let assets = Assets::new();
    let mut game = assets.game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    // Put a solid beside the spawn in the draft only: nothing is saved yet.
    let (x, y) = session.draft.layout.spawn;
    session
        .draft
        .layout
        .terrain
        .push((x + 20.0, y - 100.0, 40.0, 200.0));
    session.player.set_field(Field::Width, 64.0).unwrap();
    let spawns = session.spawn_checks();
    assert!(
        session.player.validate(&spawns).is_err(),
        "the draft's own solid counts"
    );
    session.player.set_field(Field::Width, 32.0).unwrap();
    session.player.validate(&session.spawn_checks()).unwrap();
}
