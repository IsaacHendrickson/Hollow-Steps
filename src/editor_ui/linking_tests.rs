//! The exit-link flow as `draw_editor` runs it, without a window: what happens to the
//! selection, view, tool and undo history of room A while B is open.
use super::*;
use crate::editor_panel::LinkChoice;
use hollow_steps::editor::EditorSession;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::room::{RoomEdge, RoomId};

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn zoomed() -> CanvasView {
    CanvasView {
        offset: vec2(10.0, 20.0),
        scale: 2.0,
    }
}

/// A state that has selected First's exit and zoomed, with one undo step already taken.
fn busy_state(editor: &mut EditorSession) -> EditorUi {
    let mut state = EditorUi {
        selected: Some(Selection::Exit(0)),
        view: Some(zoomed()),
        tool: EditorTool::Select,
        ..EditorUi::default()
    };
    tracked(&mut state.history, editor, |e| {
        e.draft.delete(Selection::Item(0))
    })
    .unwrap();
    editor.dirty = false;
    state
}

/// First's exit 0 linked to Second, with Second open.
fn in_second(editor: &mut EditorSession, state: &mut EditorUi) {
    state.start_link(editor, 0);
    state.apply_link_choice(editor, LinkChoice::Room(RoomId::Second));
}

#[test]
fn starting_a_link_selects_the_exit_and_opens_the_chooser() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.start_link(editor, 0);
    assert_eq!(state.selected, Some(Selection::Exit(0)));
    assert!(linking::link_dialog_open(editor) && !linking::link_connecting(editor));
    assert!(
        state.link_stash.is_none(),
        "A's state is only set aside once B opens"
    );
}

#[test]
fn an_exit_that_does_not_exist_cannot_start_a_link() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.start_link(editor, 9);
    assert!(!editor.link_active());
    assert_eq!(state.selected, None);
    assert!(!editor.message.is_empty());
}

#[test]
fn a_new_exit_is_selected_and_the_tool_is_select() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let count = editor.draft.layout.exits.len();
    let mut state = EditorUi {
        tool: EditorTool::Terrain,
        destination: Some(0),
        ..EditorUi::default()
    };
    state.start_link_new_exit(editor, RoomEdge::Top);
    assert_eq!(state.selected, Some(Selection::Exit(count)));
    assert_eq!((state.tool, state.destination), (EditorTool::Select, None));
    assert!(linking::link_dialog_open(editor));
}

#[test]
fn opening_b_sets_a_s_window_state_aside_and_starts_b_fresh() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = busy_state(editor);
    state.properties = true;
    in_second(editor, &mut state);
    assert!(linking::link_connecting(editor));
    assert_eq!(editor.id, RoomId::Second);
    assert_eq!(state.selected, None);
    assert!(state.view.is_none(), "B fits its own room");
    assert!(!state.history.can_undo(), "B starts with a scratch history");
    assert!(!state.properties && state.tool == EditorTool::Select);
    assert!(state.link_stash.is_some());
}

#[test]
fn connecting_puts_back_the_selection_and_view_and_keeps_the_message() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = busy_state(editor);
    in_second(editor, &mut state);
    state.selected = Some(Selection::Exit(0));
    state.view = Some(CanvasView {
        offset: Vec2::ZERO,
        scale: 0.5,
    });
    state.finish_link(editor);
    assert_eq!(editor.id, RoomId::First);
    assert_eq!(state.selected, Some(Selection::Exit(0)));
    assert_eq!(state.view, Some(zoomed()));
    assert_eq!(editor.message, "Linked First <-> Second");
    assert!(state.link_stash.is_none() && !state.link_placing);
}

#[test]
fn the_link_is_one_undo_step_in_a_and_one_in_b() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = busy_state(editor);
    let a_before = editor.draft.clone();
    let steps_before = state.history.can_undo();
    assert!(steps_before);
    in_second(editor, &mut state);
    state.selected = Some(Selection::Exit(0));
    state.finish_link(editor);
    let linked = editor.draft.clone();
    assert_ne!(linked.layout.exits[0], a_before.layout.exits[0]);

    // B's history is kept for B and has the link as its one step.
    assert_eq!(state.parked_histories.len(), 1);
    assert_eq!(state.parked_histories[0].0, RoomId::Second);
    assert!(state.parked_histories[0].1.can_undo());

    // Undo in A takes back the link in A, and then A's earlier step.
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft.layout.exits, a_before.layout.exits);
    assert_eq!(editor.draft.layout.items, a_before.layout.items);
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_ne!(editor.draft.layout.items, a_before.layout.items);
    assert!(!state.history.can_undo());
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Redo);
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Redo);
    assert_eq!(editor.draft, linked);
}

#[test]
fn b_keeps_its_history_for_a_later_link_and_each_link_adds_one_step() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    state.selected = Some(Selection::Exit(0));
    state.finish_link(editor);
    // A second exit in First, linked to Second again.
    state.start_link_new_exit(editor, RoomEdge::Top);
    state.apply_link_choice(editor, LinkChoice::Room(RoomId::Second));
    assert!(state.parked_histories.is_empty(), "B's history is in use");
    state.selected = Some(Selection::Exit(1));
    state.finish_link(editor);
    assert_eq!(state.parked_histories.len(), 1);
    let mut history = state.parked_histories.remove(0).1;
    let mut draft = game_draft(&game);
    assert!(history.undo(&mut draft, |_| Ok(())).unwrap());
    assert!(
        history.undo(&mut draft, |_| Ok(())).unwrap(),
        "two links, two steps"
    );
    assert!(!history.undo(&mut draft, |_| Ok(())).unwrap());
}

fn game_draft(game: &GameState) -> hollow_steps::editor::RoomDraft {
    game.editor()
        .unwrap()
        .parked
        .iter()
        .find(|work| work.id == RoomId::Second)
        .unwrap()
        .draft
        .clone()
}

#[test]
fn cancelling_in_b_puts_everything_back_and_leaves_the_history_alone() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = busy_state(editor);
    let draft = editor.draft.clone();
    in_second(editor, &mut state);
    state.place_link_exit(editor, vec2(480.0, 44.0));
    assert!(state.cancel_link(editor));
    assert_eq!(editor.id, RoomId::First);
    assert_eq!(editor.draft, draft);
    assert!(!editor.dirty);
    assert_eq!(state.selected, Some(Selection::Exit(0)));
    assert_eq!(state.view, Some(zoomed()));
    assert!(state.history.can_undo(), "A's earlier step is still there");
    assert!(state.parked_histories.is_empty());
    assert_eq!(editor.message, "Link cancelled");
    assert!(state.link_stash.is_none());
}

#[test]
fn cancelling_puts_back_the_history_b_had_before_the_flow() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    state.selected = Some(Selection::Exit(0));
    state.finish_link(editor);
    state.start_link(editor, 0);
    state.apply_link_choice(editor, LinkChoice::Room(RoomId::Second));
    assert!(state.parked_histories.is_empty());
    state.cancel_link(editor);
    assert_eq!(state.parked_histories.len(), 1);
    assert!(state.parked_histories[0].1.can_undo());
}

#[test]
fn cancelling_before_b_opens_has_nothing_to_put_back() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = busy_state(editor);
    assert!(!state.cancel_link(editor), "no flow, nothing to cancel");
    state.start_link(editor, 0);
    assert!(state.cancel_link(editor));
    assert!(!editor.link_active());
    assert!(state.history.can_undo() && state.view == Some(zoomed()));
}

#[test]
fn the_name_window_cancel_goes_back_to_the_chooser_and_the_next_one_ends_the_flow() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.start_link(editor, 0);
    state.apply_link_choice(editor, LinkChoice::NewRoom);
    assert!(editor.link_prompt().is_some());
    state.apply_link_choice(editor, LinkChoice::Cancel);
    assert!(editor.link_prompt().is_none());
    assert!(linking::link_dialog_open(editor));
    state.apply_link_choice(editor, LinkChoice::Cancel);
    assert!(!editor.link_active());
}

#[test]
fn a_new_room_opens_as_b_and_a_bad_name_is_reported_in_the_window() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.start_link(editor, 0);
    state.apply_link_choice(editor, LinkChoice::NewRoom);
    state.apply_link_choice(editor, LinkChoice::Name("Second".into()));
    assert!(editor
        .link_prompt()
        .unwrap()
        .error
        .as_ref()
        .unwrap()
        .contains("already"));
    assert!(!linking::link_connecting(editor));
    state.apply_link_choice(editor, LinkChoice::Name("Cellar".into()));
    assert!(linking::link_connecting(editor));
    assert_eq!(editor.title_of(editor.id), "Cellar");
    state.cancel_link(editor);
    assert!(editor.parked.is_empty());
    assert_eq!(editor.id, RoomId::First);
}

#[test]
fn choosing_the_room_the_exit_is_in_keeps_the_chooser_open_with_the_reason() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.start_link(editor, 0);
    state.apply_link_choice(editor, LinkChoice::Room(RoomId::First));
    assert!(editor.message.contains("cannot lead to the room it is in"));
    assert!(linking::link_dialog_open(editor));
    assert!(state.link_stash.is_none());
}

#[test]
fn connect_with_no_exit_selected_stays_in_b() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    state.finish_link(editor);
    assert!(
        editor.message.contains("Select an exit in Second"),
        "{}",
        editor.message
    );
    assert!(linking::link_connecting(editor) && state.link_stash.is_some());
    // A selected point that is not an exit does not count either.
    state.selected = Some(Selection::Spawn);
    state.finish_link(editor);
    assert!(linking::link_connecting(editor));
}

#[test]
fn placing_an_exit_selects_it_and_disarms_new_exit_here_but_a_refusal_keeps_it_armed() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    editor.draft.layout.terrain = vec![(80.0, 100.0, 60.0, 200.0)];
    state.link_placing = true;
    state.place_link_exit(editor, vec2(84.0, 200.0));
    assert!(
        editor.message.contains("inside terrain"),
        "{}",
        editor.message
    );
    assert!(state.link_placing, "try again somewhere else");
    assert_eq!(state.selected, None);
    state.place_link_exit(editor, vec2(480.0, 44.0));
    assert!(!state.link_placing);
    assert_eq!(state.selected, Some(Selection::Exit(2)));
    assert!(editor.message.is_empty());
}

#[test]
fn undo_and_redo_do_nothing_while_the_link_is_open() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    // A drag in B would record into the scratch history; undo must still not touch B.
    tracked(&mut state.history, editor, |e| {
        e.draft.layout.exits[0].offset += 10.0;
        Ok(())
    })
    .unwrap();
    let draft = editor.draft.clone();
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, draft);
}

#[test]
fn a_dragged_exit_in_b_stays_in_b_after_connect_and_is_cancelled_with_it() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    let offset = editor.draft.layout.exits[0].offset;
    editor
        .move_selection(Selection::Exit(0), vec2(60.0, 300.0))
        .unwrap();
    assert_ne!(editor.draft.layout.exits[0].offset, offset);
    state.selected = Some(Selection::Exit(0));
    state.finish_link(editor);
    let kept = editor
        .parked
        .iter()
        .find(|work| work.id == RoomId::Second)
        .unwrap();
    assert_ne!(
        kept.draft.layout.exits[0].offset, offset,
        "the adjusted exit is kept"
    );

    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    editor
        .move_selection(Selection::Exit(0), vec2(60.0, 300.0))
        .unwrap();
    state.cancel_link(editor);
    assert!(editor.parked.is_empty(), "and thrown away with a Cancel");
}

#[test]
fn a_new_session_forgets_the_link_state_too() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    in_second(editor, &mut state);
    state.selected = Some(Selection::Exit(0));
    state.finish_link(editor);
    state.link_placing = true;
    assert!(!state.parked_histories.is_empty());
    let next = state.for_new_session();
    assert!(next.parked_histories.is_empty() && !next.link_placing);
    assert!(next.link_stash.is_none());
}
