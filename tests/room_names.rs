//! Room names: stored in the room file, shown wherever a room is listed, checked when typed
//! and again when saved, and never tied to a file name.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{RoomLayout, Selection};
use hollow_steps::editor_guard::{perform, GuardedAction};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEnemies, RoomId};
use hollow_steps::room_name::room_title;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-names-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }

    /// The names of the files in `rooms/`, sorted.
    fn room_files(&self) -> Vec<String> {
        let mut files: Vec<String> = std::fs::read_dir(self.0.join("rooms"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name().into_string().unwrap())
            .collect();
        files.sort();
        files
    }

    fn read(&self, file: &str) -> Vec<u8> {
        std::fs::read(self.0.join("rooms").join(file)).unwrap()
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn layout() -> RoomLayout {
    let game = GameState::try_new(&support::fixture_assets()).unwrap();
    RoomLayout::from_room(game.room())
}

fn round_trip(layout: &RoomLayout) -> (String, RoomLayout) {
    let text = ron::ser::to_string_pretty(
        &RoomEnemies {
            enemies: Vec::new(),
            boss: None,
            layout: Some(layout.clone()),
        },
        ron::ser::PrettyConfig::default(),
    )
    .unwrap();
    let back: RoomEnemies = ron::from_str(&text).unwrap();
    (text, back.layout.unwrap())
}

fn sorted(mut labels: Vec<String>) -> Vec<String> {
    labels.sort();
    labels
}

/// A game with its editor open on the first room.
fn editing(temp: &TempAssets) -> GameState {
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    game
}

#[test]
fn a_name_survives_the_ron_round_trip() {
    let mut named = layout();
    named.name = Some("Crypt of Echoes".into());
    let (text, back) = round_trip(&named);
    assert!(text.contains("name: Some(\"Crypt of Echoes\")"), "{text}");
    assert_eq!(back, named);
    assert_eq!(back.name.as_deref(), Some("Crypt of Echoes"));
}

#[test]
fn an_unnamed_room_round_trips_and_writes_no_name() {
    let mut unnamed = layout();
    unnamed.name = None;
    let (text, back) = round_trip(&unnamed);
    assert!(!text.contains("name"), "{text}");
    assert_eq!(back, unnamed);
    assert_eq!(back.name, None);
}

#[test]
fn an_old_room_file_without_a_name_loads_and_keeps_its_display_name() {
    // Layout written before names existed: no `name` field anywhere.
    let old = r#"(
        enemies: [],
        layout: Some((
            bounds: (0.0, 0.0, 960.0, 460.0),
            floor_y: 460.0,
            spawn: (180.0, 436.0),
            items: [],
            exits: [],
        )),
    )"#;
    let document: RoomEnemies = ron::from_str(old).unwrap();
    assert_eq!(document.layout.unwrap().name, None);

    // And the shipped fixture rooms, which have no layout at all, still load.
    let game = GameState::try_new(&support::fixture_assets()).unwrap();
    assert_eq!(game.room().name, None);
    assert_eq!(
        room_title(game.room().id, game.room().name.as_deref()),
        "First"
    );
}

#[test]
fn an_unknown_field_next_to_the_name_is_still_refused() {
    let text = r#"(enemies: [], layout: Some((
        name: Some("Crypt"), title: "x",
        bounds: (0.0, 0.0, 960.0, 460.0), floor_y: 460.0, spawn: (180.0, 436.0),
        items: [], exits: [],
    )))"#;
    assert!(ron::from_str::<RoomEnemies>(text).is_err());
}

#[test]
fn shipped_rooms_are_listed_by_their_display_names() {
    let temp = TempAssets::new();
    let game = editing(&temp);
    let editor = game.editor().unwrap();
    assert_eq!(editor.room_labels(), ["First", "Second", "Boss"]);
    assert_eq!(editor.room_title(), "First");
    assert_eq!(editor.room_name_text(), "First");
    assert_eq!(editor.title_of(RoomId::Boss), "Boss");
}

#[test]
fn a_new_room_starts_with_the_next_free_default_name() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    assert_eq!(game.editor().unwrap().suggested_room_name(), "Room 1");
    game.editor_mut().unwrap().new_room();
    assert_eq!(
        game.editor().unwrap().draft.layout.name.as_deref(),
        Some("Room 1")
    );
    game.save_editor().unwrap();

    // The saved room holds its name, and the next suggestion moves on.
    game.editor_mut().unwrap().new_room();
    assert_eq!(
        game.editor().unwrap().draft.layout.name.as_deref(),
        Some("Room 2")
    );
}

#[test]
fn a_new_room_takes_a_typed_name_and_a_bad_name_changes_nothing() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.clone();
    for bad in ["", "   ", "Boss", "first", "Cr!pt", &"x".repeat(33)] {
        assert!(editor.new_room_named(bad).is_err(), "{bad:?}");
        assert_eq!(editor.draft, before, "{bad:?}");
        assert_eq!(editor.id, RoomId::First, "{bad:?}");
        assert!(!editor.new);
    }
    editor.new_room_named("  The Crypt  ").unwrap();
    assert_eq!(editor.draft.layout.name.as_deref(), Some("The Crypt"));
    assert_eq!(editor.id, RoomId::Custom(0));
    assert!(editor.new && editor.dirty);
    // A new room does not inherit its template's name.
    assert_ne!(
        editor.draft.layout.name.as_deref(),
        before.layout.name.as_deref()
    );
}

#[test]
fn a_new_room_is_created_and_named_through_the_guarded_action() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    perform(&mut game, &GuardedAction::NewNamedRoom("Cellar".into())).unwrap();
    assert_eq!(
        game.editor().unwrap().draft.layout.name.as_deref(),
        Some("Cellar")
    );
    game.save_editor().unwrap();
    let saved: RoomEnemies =
        ron::from_str(&String::from_utf8(temp.read("room_0.ron")).unwrap()).unwrap();
    assert_eq!(saved.layout.unwrap().name.as_deref(), Some("Cellar"));

    // A taken name is refused and the editor stays where it was.
    let error = perform(&mut game, &GuardedAction::NewNamedRoom("cellar".into())).unwrap_err();
    assert!(error.contains("already called"), "{error}");
    assert_eq!(game.editor().unwrap().id, RoomId::Custom(0));
    assert!(!game.editor().unwrap().new);
}

#[test]
fn the_default_new_room_action_still_works() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    perform(&mut game, &GuardedAction::NewRoom).unwrap();
    assert_eq!(
        game.editor().unwrap().draft.layout.name.as_deref(),
        Some("Room 1")
    );
}

#[test]
fn a_named_room_is_loaded_by_the_game_and_shown_by_its_name() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    game.editor_mut().unwrap().new_room_named("Cellar").unwrap();
    game.save_editor().unwrap();
    assert_eq!(game.room().name.as_deref(), Some("Cellar"));

    let mut restarted = temp.game();
    restarted.edit_room(RoomId::Custom(0)).unwrap();
    let room = restarted.room();
    assert_eq!(room.name.as_deref(), Some("Cellar"));
    assert_eq!(room_title(room.id, room.name.as_deref()), "Cellar");
    let editor = restarted.editor().unwrap();
    assert_eq!(editor.room_labels(), ["First", "Second", "Boss", "Cellar"]);
    assert_eq!(editor.room_title(), "Cellar");
}

#[test]
fn renaming_a_room_saves_the_name_and_touches_no_file_name() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    game.editor_mut().unwrap().new_room_named("Cellar").unwrap();
    game.save_editor().unwrap();
    let files = temp.room_files();
    let other_files: Vec<_> = ["first.ron", "second.ron", "boss.ron"]
        .iter()
        .map(|file| (file, temp.read(file)))
        .collect();

    let editor = game.editor_mut().unwrap();
    editor.set_room_name_text("Wine Cellar").unwrap();
    assert!(editor.dirty);
    game.save_editor().unwrap();

    assert_eq!(
        temp.room_files(),
        files,
        "no room file renamed, added or deleted"
    );
    for (file, bytes) in other_files {
        assert_eq!(temp.read(file), bytes, "{file} must not change");
    }
    let saved: RoomEnemies =
        ron::from_str(&String::from_utf8(temp.read("room_0.ron")).unwrap()).unwrap();
    assert_eq!(saved.layout.unwrap().name.as_deref(), Some("Wine Cellar"));
    assert_eq!(
        game.editor().unwrap().title_of(RoomId::Custom(0)),
        "Wine Cellar"
    );
    assert!(game
        .editor()
        .unwrap()
        .room_labels()
        .contains(&"Wine Cellar".to_owned()));
}

#[test]
fn naming_a_shipped_room_does_not_touch_the_other_room_files() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let second = temp.read("second.ron");
    game.editor_mut()
        .unwrap()
        .set_room_name_text("Entrance")
        .unwrap();
    game.save_editor().unwrap();
    assert_eq!(temp.room_files(), ["boss.ron", "first.ron", "second.ron"]);
    assert_eq!(temp.read("second.ron"), second);
    let mut restarted = temp.game();
    assert_eq!(restarted.room().name.as_deref(), Some("Entrance"));
    restarted.toggle_editor().unwrap();
    assert_eq!(
        restarted.editor().unwrap().room_labels(),
        ["Entrance", "Second", "Boss"]
    );
}

#[test]
fn good_text_goes_into_the_draft_trimmed_and_bad_text_waits_with_its_error() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let editor = game.editor_mut().unwrap();
    assert!(!editor.dirty);

    editor.set_room_name_text("  Hall of Bones ").unwrap();
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Hall of Bones"));
    assert!(editor.dirty);
    assert_eq!(editor.name_error(), None);
    // The text stays as typed while it is typed.
    assert_eq!(editor.room_name_text(), "  Hall of Bones ");
    editor.finish_name_edit();
    assert_eq!(editor.room_name_text(), "Hall of Bones");

    // A refused name leaves the draft on the last good name and blocks Save.
    assert!(editor.set_room_name_text("Hall!").is_err());
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Hall of Bones"));
    assert!(editor.name_error().unwrap().contains("cannot contain"));
    assert_eq!(editor.room_name_text(), "Hall!");
    editor.finish_name_edit();
    assert!(
        editor.name_error().is_some(),
        "refused text stays after focus is lost"
    );
    assert!(game.save_editor().unwrap_err().contains("cannot contain"));

    // Fixing it lifts the block.
    let editor = game.editor_mut().unwrap();
    editor.set_room_name_text("Hall of Ash").unwrap();
    assert_eq!(editor.name_error(), None);
    game.save_editor().unwrap();
}

#[test]
fn a_refused_name_blocks_save_and_writes_nothing() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let before = temp.read("first.ron");
    for bad in ["", "Second", "BOSS", "a/b", &"y".repeat(40)] {
        let editor = game.editor_mut().unwrap();
        assert!(editor.set_room_name_text(bad).is_err(), "{bad:?}");
        assert!(game.save_editor().is_err(), "{bad:?}");
        assert_eq!(temp.read("first.ron"), before, "{bad:?}");
    }
}

#[test]
fn names_are_unique_ignoring_case_across_all_rooms() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let editor = game.editor_mut().unwrap();
    for taken in ["Second", "second", "BOSS"] {
        let error = editor.set_room_name_text(taken).unwrap_err();
        assert!(error.contains("already called"), "{taken}: {error}");
    }
    // The room's own display name is fine.
    editor.set_room_name_text("first").unwrap();
    assert_eq!(editor.draft.layout.name.as_deref(), Some("first"));
}

#[test]
fn a_name_must_also_differ_from_the_unsaved_draft() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    // The first room is renamed but not saved: that draft already owns the name.
    let editor = game.editor_mut().unwrap();
    editor.set_room_name_text("Cellar").unwrap();
    for taken in ["Cellar", "cellar", " CELLAR "] {
        let error = editor.new_room_named(taken).unwrap_err();
        assert!(error.contains("already called"), "{taken}: {error}");
        assert_eq!(editor.id, RoomId::First);
    }
    assert_ne!(editor.suggested_room_name(), "Cellar");
    editor.new_room_named("Cellar 2").unwrap();
}

#[test]
fn the_unsaved_draft_keeps_its_name_reserved_after_the_room_is_replaced() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    game.editor_mut()
        .unwrap()
        .set_room_name_text("Cellar")
        .unwrap();
    game.save_editor().unwrap();
    game.editor_mut().unwrap().new_room_named("Attic").unwrap();
    // Neither the saved room nor this new draft can be given the other's name.
    let editor = game.editor_mut().unwrap();
    assert!(editor.set_room_name_text("cellar").is_err());
    assert!(
        editor.set_room_name_text("ATTIC").is_ok(),
        "its own name, other case"
    );
    editor.set_room_name_text("Attic").unwrap();
    game.save_editor().unwrap();
    assert_eq!(
        sorted(game.editor().unwrap().room_labels()),
        ["Attic", "Boss", "Cellar", "Second"]
    );
}

#[test]
fn unnamed_custom_rooms_cannot_be_copied_by_name() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    game.editor_mut().unwrap().new_room_named("Cellar").unwrap();
    game.save_editor().unwrap();
    // Take the name away by editing the file, the way an older version would have it.
    let source = String::from_utf8(temp.read("room_0.ron")).unwrap();
    let nameless = source.replace("name: Some(\"Cellar\"),", "");
    assert_ne!(source, nameless);
    std::fs::write(temp.0.join("rooms/room_0.ron"), nameless).unwrap();

    let mut game = editing(&temp);
    let editor = game.editor_mut().unwrap();
    assert_eq!(editor.room_labels(), ["First", "Second", "Boss", "Room 1"]);
    let error = editor.set_room_name_text("room 1").unwrap_err();
    assert!(error.contains("already called \"Room 1\""), "{error}");
    assert_eq!(editor.suggested_room_name(), "Room 2");
}

#[test]
fn the_name_dirties_the_session_only_when_it_changes() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let editor = game.editor_mut().unwrap();
    // Typing the room's own display name back is no change at all.
    editor.set_room_name_text("First").unwrap();
    assert!(!editor.dirty);
    assert_eq!(editor.draft.layout.name, None);
    editor.set_room_name_text("Entrance").unwrap();
    assert!(editor.dirty);
}

#[test]
fn undo_and_redo_restore_the_name_field_from_the_draft() {
    use hollow_steps::editor_history::{redo_session, tracked, undo_session, History};
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    tracked(&mut history, editor, |e| e.set_room_name_text("Entrance")).unwrap();
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Entrance"));

    assert!(editor.set_room_name_text("Bad!").is_err());
    assert!(undo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft.layout.name, None);
    assert_eq!(
        editor.room_name_text(),
        "First",
        "pending bad text is dropped"
    );
    assert_eq!(editor.name_error(), None);

    assert!(redo_session(&mut history, editor).unwrap());
    assert_eq!(editor.room_name_text(), "Entrance");
}

#[test]
fn the_exit_target_list_and_dialogs_name_rooms_by_their_names() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    game.editor_mut().unwrap().new_room_named("Cellar").unwrap();
    game.save_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    editor.set_room_name_text("Larder").unwrap();
    // The list shows the room being edited by its draft name; the others by theirs.
    assert_eq!(editor.title_of(RoomId::Custom(0)), "Larder");
    assert_eq!(editor.title_of(RoomId::First), "First");
    assert_eq!(
        sorted(editor.room_labels()),
        ["Boss", "First", "Larder", "Second"]
    );
    let action = GuardedAction::OpenRoom(RoomId::Custom(0));
    assert_eq!(
        action.describe_with(&|id| editor.title_of(id)),
        "open room Larder"
    );
    assert_eq!(action.describe(), "open room Room 1");
    assert_eq!(
        GuardedAction::NewNamedRoom("X".into()).describe(),
        "start a new room"
    );
}

#[test]
fn exits_keep_working_with_named_rooms() {
    // A name is only a label: exits still point at room ids.
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    game.editor_mut().unwrap().new_room_named("Cellar").unwrap();
    game.save_editor().unwrap();
    game.edit_room(RoomId::First).unwrap();
    let editor = game.editor_mut().unwrap();
    let selection = editor.draft.add_exit(
        hollow_steps::room::RoomEdge::Right,
        RoomId::Custom(0),
        macroquad::math::Vec2::new(180.0, 436.0),
    );
    let index = editor.draft.layout.exits.len() - 1;
    assert_eq!(selection, Selection::Exit(index));
    assert_eq!(
        editor.draft.layout.exits[index].target_room,
        RoomId::Custom(0)
    );
}
