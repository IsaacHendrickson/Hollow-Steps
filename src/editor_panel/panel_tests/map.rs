//! The room tools' dungeon map controls: the Map cell row (Auto, the two numbers, where the room
//! is shown), the Dungeon map button and window, and a click in it opening a room through the
//! unsaved-changes guard like the Open room button.
use super::*;
use crate::editor_panel::test_support::drawn_text;
use hollow_steps::editor_guard::{GuardStep, GuardedAction, UnsavedGuard};
use hollow_steps::editor_history::undo_session;
use hollow_steps::room::RoomId;

/// Wide enough that the map window and the panel do not overlap, tall enough to need no scrolling.
pub(super) const SIZE: egui::Vec2 = egui::vec2(1200.0, 3200.0);

pub(super) fn run(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> (PanelResponse, String) {
    let mut response = None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SIZE)),
            events,
            ..Default::default()
        },
        |ctx| response = Some(draw_panel(ctx, editor, state)),
    );
    (response.unwrap(), drawn_text(&output))
}

pub(super) fn laid_out(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
    let ctx = egui::Context::default();
    for _ in 0..4 {
        run(&ctx, vec![], editor, state);
    }
    ctx
}

pub(super) fn text(
    ctx: &egui::Context,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> String {
    run(ctx, vec![], editor, state).1
}

/// Presses and releases at `at`; the response of the frame that saw the release.
pub(super) fn press(
    ctx: &egui::Context,
    at: egui::Pos2,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> PanelResponse {
    // The pointer arrives a frame before it presses, as a hand's does; otherwise egui reads the
    // jump from the last click as a drag of the window.
    run(ctx, vec![egui::Event::PointerMoved(at)], editor, state);
    run(ctx, pointer(at, true), editor, state);
    let released = run(ctx, pointer(at, false), editor, state).0;
    run(ctx, vec![], editor, state);
    released
}

pub(super) fn probed(
    ctx: &egui::Context,
    id: impl std::hash::Hash + Send + Sync + 'static,
) -> egui::Rect {
    ctx.data(|data| data.get_temp(egui::Id::new(id)))
        .expect("the widget was drawn")
}

fn room_box(ctx: &egui::Context, room: RoomId) -> egui::Rect {
    probed(ctx, ("map room", room))
}

fn open_window(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) {
    let at = button(ctx, "Dungeon map").center();
    press(ctx, at, editor, state);
    assert!(state.checks.map_window, "the button opens the window");
    for _ in 0..3 {
        run(ctx, vec![], editor, state);
    }
}

// ---- the Map cell row ---------------------------------------------------------------------

#[test]
fn the_row_says_auto_and_where_the_map_puts_the_room() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    let shown = editor.resolved_map_cell().unwrap();
    let text = text(&ctx, editor, &mut state);
    for expected in ["Map cell", "Auto (from the exits)", "Dungeon map"] {
        assert!(text.contains(expected), "{expected} is missing: {text}");
    }
    assert!(
        text.contains(&format!("Shown at ({}, {})", shown.0, shown.1)),
        "{text}"
    );
    assert!(!text.contains("another room has this cell"), "{text}");
}

#[test]
fn turning_auto_off_pins_the_room_where_it_is_as_one_undo_step_that_marks_it_unsaved() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    let shown = editor.resolved_map_cell().unwrap();
    assert!(!editor.dirty && editor.draft.layout.map_cell.is_none());

    press(
        &ctx,
        probed(&ctx, ("map cell", "auto")).center(),
        editor,
        &mut state,
    );
    assert_eq!(editor.draft.layout.map_cell, Some(shown), "nothing jumps");
    assert!(editor.dirty, "the room is unsaved");
    assert!(state.history.can_undo());
    let text = text(&ctx, editor, &mut state);
    assert!(
        text.contains('X') && text.contains('Y'),
        "the numbers appear: {text}"
    );

    // Back to Auto, then undo twice: pinned again, then Auto as at the start.
    press(
        &ctx,
        probed(&ctx, ("map cell", "auto")).center(),
        editor,
        &mut state,
    );
    assert_eq!(editor.draft.layout.map_cell, None);
    assert_eq!(undo_session(&mut state.history, editor), Ok(true));
    assert_eq!(editor.draft.layout.map_cell, Some(shown));
    assert_eq!(undo_session(&mut state.history, editor), Ok(true));
    assert_eq!(editor.draft.layout.map_cell, None);
    assert_eq!(undo_session(&mut state.history, editor), Ok(false));
}

#[test]
fn dragging_a_number_changes_the_cell_and_the_whole_drag_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_map_cell(Some((0, 0))).unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    let field = probed(&ctx, ("map cell", "X"));
    let start = field.center();

    run(&ctx, pointer(start, true), editor, &mut state);
    for step in 1..=4 {
        let to = start + egui::vec2(10.0 * step as f32, 0.0);
        run(
            &ctx,
            vec![egui::Event::PointerMoved(to)],
            editor,
            &mut state,
        );
    }
    let end = start + egui::vec2(40.0, 0.0);
    run(&ctx, pointer(end, false), editor, &mut state);
    run(&ctx, vec![], editor, &mut state);

    let (x, y) = editor.draft.layout.map_cell.unwrap();
    assert!(x > 0 && y == 0, "dragged right: {x}, {y}");
    assert!(editor.dirty);
    assert_eq!(undo_session(&mut state.history, editor), Ok(true));
    assert_eq!(
        editor.draft.layout.map_cell,
        Some((0, 0)),
        "one step undoes the drag"
    );
    assert_eq!(undo_session(&mut state.history, editor), Ok(false));
}

#[test]
fn a_room_sharing_its_cell_says_where_the_map_put_it() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    assert!(!text(&ctx, editor, &mut state).contains("another room has this cell"));

    // Second names the cell First names: First has the lower id and keeps it.
    editor.draft.set_map_cell(Some((3, 3))).unwrap();
    editor
        .rooms
        .iter_mut()
        .find(|(id, _)| *id == RoomId::Second)
        .unwrap()
        .1
        .map_cell = Some((3, 3));
    let text = text(&ctx, editor, &mut state);
    assert!(
        text.contains("Shown at (3, 3): another room has this cell"),
        "{text}"
    );
}

// ---- the window ---------------------------------------------------------------------------

#[test]
fn the_window_is_closed_until_the_button_is_pressed_and_the_button_closes_it_again() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    assert!(!state.checks.map_window);
    assert!(!text(&ctx, editor, &mut state).contains("Click a room to open it"));

    open_window(&ctx, editor, &mut state);
    assert!(text(&ctx, editor, &mut state).contains("Click a room to open it"));

    press(
        &ctx,
        button(&ctx, "Dungeon map").center(),
        editor,
        &mut state,
    );
    assert!(!state.checks.map_window);
    assert!(!text(&ctx, editor, &mut state).contains("Click a room to open it"));
}

#[test]
fn the_window_names_every_room_including_an_unsaved_new_one() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.new_room_named("Cellar").unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);
    let text = text(&ctx, editor, &mut state);
    for name in ["First", "Second", "Boss", "Cellar"] {
        assert!(text.contains(name), "{name} is missing: {text}");
    }
    assert!(room_box(&ctx, editor.id).width() > 0.0);
}

#[test]
fn the_window_marks_rooms_that_share_a_cell_and_rooms_nothing_leads_to() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    // A new room has no exit to it and, with the cell it names, takes First's.
    editor.new_room_named("Cellar").unwrap();
    editor.draft.set_map_cell(Some((0, 0))).unwrap();
    editor
        .rooms
        .iter_mut()
        .find(|(id, _)| *id == RoomId::First)
        .unwrap()
        .1
        .map_cell = Some((0, 0));
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);
    let text = text(&ctx, editor, &mut state);
    assert!(text.contains("same cell"), "{text}");
    assert!(text.contains("no way in"), "{text}");
}

#[test]
fn a_room_that_exits_lead_to_is_not_marked() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);
    // The fixture rooms lead to each other and nothing is on one cell.
    let text = text(&ctx, editor, &mut state);
    assert!(!text.contains("same cell"), "{text}");
}

#[test]
fn clicking_another_room_asks_to_open_it() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);
    assert_eq!(editor.id, RoomId::First);

    let second = room_box(&ctx, RoomId::Second).center();
    assert_eq!(
        press(&ctx, second, editor, &mut state).open,
        Some(RoomId::Second)
    );
    let boss = room_box(&ctx, RoomId::Boss).center();
    assert_eq!(
        press(&ctx, boss, editor, &mut state).open,
        Some(RoomId::Boss)
    );
}

#[test]
fn clicking_the_room_being_edited_or_empty_space_asks_for_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);

    let here = room_box(&ctx, editor.id).center();
    assert_eq!(press(&ctx, here, editor, &mut state).open, None);
    let canvas = probed(&ctx, "map window canvas");
    let corner = canvas.left_top() + egui::vec2(3.0, 3.0);
    assert_eq!(press(&ctx, corner, editor, &mut state).open, None);
    assert!(state.checks.map_window, "the window stays open");
}

#[test]
fn the_click_asks_the_same_question_as_the_open_room_button_and_the_guard_answers_alike() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);

    let from_map = press(
        &ctx,
        room_box(&ctx, RoomId::Second).center(),
        editor,
        &mut state,
    )
    .open;
    state.room.selected = editor
        .rooms
        .iter()
        .position(|(id, _)| *id == RoomId::Second);
    let from_button = press(&ctx, button(&ctx, "Open room").center(), editor, &mut state).open;
    assert_eq!(from_map, Some(RoomId::Second));
    assert_eq!(from_map, from_button);

    // Clean: the room opens at once. With unsaved changes: the dialog asks first.
    let action = GuardedAction::OpenRoom(RoomId::Second);
    let mut guard = UnsavedGuard::default();
    assert_eq!(
        guard.request_for(action.clone(), Some(editor)),
        GuardStep::Perform(action.clone())
    );
    editor.dirty = true;
    let mut guard = UnsavedGuard::default();
    assert_eq!(
        guard.request_for(action.clone(), Some(editor)),
        GuardStep::Wait
    );
    assert_eq!(
        guard.prompt().map(|prompt| prompt.action.clone()),
        Some(action)
    );
}

#[test]
fn opening_another_room_closes_the_window_with_the_rest_of_the_session_state() {
    let mut state = EditorUi::default();
    state.checks.map_window = true;
    assert!(!state.for_new_session().checks.map_window);
}

#[test]
fn the_window_is_not_drawn_while_an_exit_link_is_open() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open_window(&ctx, editor, &mut state);
    state.start_link(editor, 0);
    state.apply_link_choice(
        editor,
        crate::editor_panel::LinkChoice::Room(RoomId::Second),
    );
    assert!(editor.link_active());
    for _ in 0..3 {
        run(&ctx, vec![], editor, &mut state);
    }
    assert!(!text(&ctx, editor, &mut state).contains("Click a room to open it"));
    assert!(state.checks.map_window, "it comes back when the link ends");
}
