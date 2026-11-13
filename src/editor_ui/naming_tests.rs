//! The New room flow as `draw_editor` runs it: the name window first, then the
//! unsaved-changes guard, then the room is started.
use super::*;
use crate::editor_panel::NameChoice;
use hollow_steps::editor_guard::GuardChoice;
use hollow_steps::room::RoomId;

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

/// One frame of `draw_editor`'s tail: the request goes through the guard, and an approved
/// action is carried out.
fn run_requests(game: &mut GameState, state: &mut EditorUi, choice: Option<GuardChoice>) {
    let dirty = game.editor().is_some_and(|session| session.dirty);
    let mut step = GuardStep::Wait;
    if let Some(action) = state.requested.take() {
        step = state.guard.request(action, dirty);
    }
    if let Some(choice) = choice {
        step = state.guard.choose(choice);
    }
    if let GuardStep::Perform(action) = step {
        perform(game, &action).unwrap();
        *state = state.for_new_session();
    }
}

#[test]
fn a_confirmed_name_becomes_a_guarded_request_and_closes_the_window() {
    let mut state = EditorUi {
        name_prompt: Some(NamePrompt::new("Room 1".into())),
        ..EditorUi::default()
    };
    state.finish_naming(Some(NameChoice::Create("Cellar".into())));
    assert!(state.name_prompt.is_none());
    assert_eq!(
        state.requested,
        Some(GuardedAction::NewNamedRoom("Cellar".into()))
    );
}

#[test]
fn cancelling_the_name_window_starts_nothing() {
    let mut state = EditorUi {
        name_prompt: Some(NamePrompt::new("Room 1".into())),
        ..EditorUi::default()
    };
    state.finish_naming(Some(NameChoice::Cancel));
    assert!(state.name_prompt.is_none());
    assert_eq!(state.requested, None);
}

#[test]
fn no_decision_leaves_the_window_open() {
    let mut state = EditorUi {
        name_prompt: Some(NamePrompt::new("Room 1".into())),
        ..EditorUi::default()
    };
    state.finish_naming(None);
    assert!(state.name_prompt.is_some());
    assert_eq!(state.requested, None);
}

#[test]
fn a_clean_session_starts_the_named_room_at_once() {
    let mut game = editing();
    let mut state = EditorUi::default();
    state.finish_naming(Some(NameChoice::Create("Cellar".into())));
    run_requests(&mut game, &mut state, None);
    let editor = game.editor().unwrap();
    assert_eq!(editor.id, RoomId::Custom(0));
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Cellar"));
    assert!(!state.guard.is_open());
}

#[test]
fn unsaved_changes_are_asked_about_after_the_name_and_cancel_keeps_the_session() {
    let mut game = editing();
    let mut state = EditorUi::default();
    game.editor_mut()
        .unwrap()
        .set_room_name_text("Entrance")
        .unwrap();
    state.finish_naming(Some(NameChoice::Create("Cellar".into())));
    run_requests(&mut game, &mut state, None);
    assert!(
        state.guard.is_open(),
        "the dialog opens after the name was given"
    );
    assert_eq!(game.editor().unwrap().id, RoomId::First);

    run_requests(&mut game, &mut state, Some(GuardChoice::Cancel));
    let editor = game.editor().unwrap();
    assert_eq!(editor.id, RoomId::First);
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Entrance"));
    assert!(editor.dirty && !state.guard.is_open());
}

#[test]
fn discarding_starts_the_named_room() {
    let mut game = editing();
    let mut state = EditorUi::default();
    game.editor_mut()
        .unwrap()
        .set_room_name_text("Entrance")
        .unwrap();
    state.finish_naming(Some(NameChoice::Create("Cellar".into())));
    run_requests(&mut game, &mut state, None);
    run_requests(&mut game, &mut state, Some(GuardChoice::Discard));
    let editor = game.editor().unwrap();
    assert_eq!(editor.id, RoomId::Custom(0));
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Cellar"));
    assert!(state.name_prompt.is_none() && state.requested.is_none());
}
