mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor_guard::{
    perform, GuardChoice, GuardStep, GuardedAction, Prompt, UnsavedGuard,
};
use hollow_steps::game::GameState;
use hollow_steps::room::RoomId;

const ACTIONS: [GuardedAction; 3] = [
    GuardedAction::Close,
    GuardedAction::OpenRoom(RoomId::Second),
    GuardedAction::NewRoom,
];

#[test]
fn a_clean_session_acts_immediately_without_a_dialog() {
    for action in ACTIONS {
        let mut guard = UnsavedGuard::default();
        assert_eq!(
            guard.request(action.clone(), false),
            GuardStep::Perform(action)
        );
        assert!(!guard.is_open());
        assert!(guard.prompt().is_none());
    }
}

#[test]
fn a_dirty_session_opens_the_dialog_and_waits() {
    for action in ACTIONS {
        let mut guard = UnsavedGuard::default();
        assert_eq!(guard.request(action.clone(), true), GuardStep::Wait);
        assert!(guard.is_open());
        assert_eq!(
            guard.prompt(),
            Some(&Prompt {
                action,
                error: None
            })
        );
    }
}

#[test]
fn cancel_closes_the_dialog_and_performs_nothing() {
    for action in ACTIONS {
        let mut guard = UnsavedGuard::default();
        guard.request(action, true);
        assert_eq!(guard.choose(GuardChoice::Cancel), GuardStep::Wait);
        assert!(!guard.is_open());
        // The same request can be made again, and asks again.
        assert_eq!(guard.request(GuardedAction::Close, true), GuardStep::Wait);
        assert!(guard.is_open());
    }
}

#[test]
fn discard_performs_the_requested_action() {
    for action in ACTIONS {
        let mut guard = UnsavedGuard::default();
        guard.request(action.clone(), true);
        assert_eq!(
            guard.choose(GuardChoice::Discard),
            GuardStep::Perform(action)
        );
        assert!(!guard.is_open());
    }
}

#[test]
fn save_asks_for_a_save_and_only_a_successful_one_performs_the_action() {
    for action in ACTIONS {
        let mut guard = UnsavedGuard::default();
        guard.request(action.clone(), true);
        assert_eq!(guard.choose(GuardChoice::Save), GuardStep::Save);
        assert!(guard.is_open(), "the dialog stays up while saving");
        assert_eq!(guard.save_finished(Ok(())), GuardStep::Perform(action));
        assert!(!guard.is_open());
    }
}

#[test]
fn a_failed_save_keeps_the_dialog_open_with_the_error() {
    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::OpenRoom(RoomId::Boss), true);
    guard.choose(GuardChoice::Save);
    assert_eq!(
        guard.save_finished(Err("Save failed: disk full".into())),
        GuardStep::Wait
    );
    let prompt = guard.prompt().expect("dialog still open");
    assert_eq!(prompt.action, GuardedAction::OpenRoom(RoomId::Boss));
    assert_eq!(prompt.error.as_deref(), Some("Save failed: disk full"));

    // Retrying clears the old error; a second failure replaces it.
    assert_eq!(guard.choose(GuardChoice::Save), GuardStep::Save);
    assert_eq!(guard.prompt().unwrap().error, None);
    guard.save_finished(Err("Save failed: still full".into()));
    assert_eq!(
        guard.prompt().unwrap().error.as_deref(),
        Some("Save failed: still full")
    );

    // A later success performs the original action.
    guard.choose(GuardChoice::Save);
    assert_eq!(
        guard.save_finished(Ok(())),
        GuardStep::Perform(GuardedAction::OpenRoom(RoomId::Boss))
    );
}

#[test]
fn after_a_failed_save_the_user_can_still_cancel_or_discard() {
    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::Close, true);
    guard.choose(GuardChoice::Save);
    guard.save_finished(Err("nope".into()));
    assert_eq!(guard.choose(GuardChoice::Cancel), GuardStep::Wait);
    assert!(!guard.is_open());

    guard.request(GuardedAction::NewRoom, true);
    guard.choose(GuardChoice::Save);
    guard.save_finished(Err("nope".into()));
    assert_eq!(
        guard.choose(GuardChoice::Discard),
        GuardStep::Perform(GuardedAction::NewRoom)
    );
}

#[test]
fn requests_while_the_dialog_is_open_are_ignored() {
    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::Close, true);
    // Pressing F2 again, or Open room, must not replace or skip the question.
    assert_eq!(guard.request(GuardedAction::Close, true), GuardStep::Wait);
    assert_eq!(
        guard.request(GuardedAction::OpenRoom(RoomId::First), false),
        GuardStep::Wait
    );
    assert_eq!(guard.prompt().unwrap().action, GuardedAction::Close);
}

#[test]
fn stray_choices_and_save_results_do_nothing() {
    let mut guard = UnsavedGuard::default();
    for choice in [GuardChoice::Save, GuardChoice::Discard, GuardChoice::Cancel] {
        assert_eq!(guard.choose(choice), GuardStep::Wait);
    }
    assert_eq!(guard.save_finished(Ok(())), GuardStep::Wait);
    assert_eq!(guard.save_finished(Err("x".into())), GuardStep::Wait);
    assert!(!guard.is_open());

    // A save result nobody asked for cannot perform the action.
    guard.request(GuardedAction::Close, true);
    assert_eq!(guard.save_finished(Ok(())), GuardStep::Wait);
    assert!(guard.is_open());
    // While a save is running, other buttons are ignored.
    guard.choose(GuardChoice::Save);
    assert_eq!(guard.choose(GuardChoice::Discard), GuardStep::Wait);
    assert_eq!(guard.choose(GuardChoice::Cancel), GuardStep::Wait);
    assert!(guard.is_open());
}

#[test]
fn the_dialog_text_names_the_action() {
    assert_eq!(GuardedAction::Close.describe(), "close the editor");
    assert_eq!(GuardedAction::NewRoom.describe(), "start a new room");
    assert!(GuardedAction::OpenRoom(RoomId::Second)
        .describe()
        .contains("Second"));
}

/// A writable copy of the fixtures, so a test can really save.
struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-editor-guard-{}-{}",
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
        Self(path)
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn editing(assets: &Assets) -> GameState {
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn make_dirty(game: &mut GameState) {
    let session = game.editor_mut().unwrap();
    session.draft.layout.spawn.0 += 8.0;
    session.dirty = true;
}

#[test]
fn cancelling_keeps_the_open_session_and_its_edits() {
    let assets = Assets::new();
    let mut game = editing(&assets);
    make_dirty(&mut game);
    let spawn = game.editor().unwrap().draft.layout.spawn;
    let mut guard = UnsavedGuard::default();
    let dirty = game.editor().unwrap().dirty;
    assert_eq!(guard.request(GuardedAction::Close, dirty), GuardStep::Wait);
    assert_eq!(guard.choose(GuardChoice::Cancel), GuardStep::Wait);
    let session = game.editor().expect("editor stayed open");
    assert!(session.dirty);
    assert_eq!(session.draft.layout.spawn, spawn);
}

#[test]
fn discarding_closes_without_writing_the_file() {
    let assets = Assets::new();
    let mut game = editing(&assets);
    make_dirty(&mut game);
    let path = game.editor().unwrap().path.clone();
    let before = std::fs::read(&path).unwrap();
    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::Close, true);
    let GuardStep::Perform(action) = guard.choose(GuardChoice::Discard) else {
        panic!("discard must perform")
    };
    perform(&mut game, &action).unwrap();
    assert!(game.editor().is_none());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn saving_then_continuing_writes_the_file_and_opens_the_other_room() {
    let assets = Assets::new();
    let mut game = editing(&assets);
    make_dirty(&mut game);
    let path = game.editor().unwrap().path.clone();
    let before = std::fs::read(&path).unwrap();
    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::OpenRoom(RoomId::Second), true);
    assert_eq!(guard.choose(GuardChoice::Save), GuardStep::Save);
    let result = game.save_editor();
    assert!(result.is_ok(), "{result:?}");
    assert!(!game.editor().unwrap().dirty);
    let GuardStep::Perform(action) = guard.save_finished(result) else {
        panic!("a good save must perform")
    };
    perform(&mut game, &action).unwrap();
    assert_ne!(std::fs::read(&path).unwrap(), before);
    assert_eq!(game.editor().unwrap().id, RoomId::Second);
}

#[test]
fn a_failed_save_leaves_the_session_dirty_and_the_dialog_open() {
    let assets = Assets::new();
    let mut game = editing(&assets);
    // A new room whose file already exists cannot be saved.
    perform(&mut game, &GuardedAction::NewRoom).unwrap();
    let session = game.editor().unwrap();
    assert!(session.new && session.dirty);
    std::fs::write(&session.path, "occupied").unwrap();

    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::Close, true);
    assert_eq!(guard.choose(GuardChoice::Save), GuardStep::Save);
    let result = game.save_editor();
    let error = result.clone().unwrap_err();
    assert_eq!(guard.save_finished(result), GuardStep::Wait);
    assert_eq!(
        guard.prompt().unwrap().error.as_deref(),
        Some(error.as_str())
    );
    assert!(game.editor().expect("still open").dirty);
}

#[test]
fn perform_new_room_starts_a_dirty_draft_and_open_room_switches_rooms() {
    let assets = Assets::new();
    let mut game = editing(&assets);
    perform(&mut game, &GuardedAction::NewRoom).unwrap();
    assert!(matches!(game.editor().unwrap().id, RoomId::Custom(_)));
    perform(&mut game, &GuardedAction::OpenRoom(RoomId::Boss)).unwrap();
    assert_eq!(game.editor().unwrap().id, RoomId::Boss);
    assert!(!game.editor().unwrap().dirty);
    assert!(perform(&mut game, &GuardedAction::OpenRoom(RoomId::Custom(99))).is_err());
}

#[test]
fn performing_close_with_the_editor_already_closed_is_harmless() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    perform(&mut game, &GuardedAction::Close).unwrap();
    assert!(game.editor().is_none());
    assert!(perform(&mut game, &GuardedAction::NewRoom).is_err());
}
