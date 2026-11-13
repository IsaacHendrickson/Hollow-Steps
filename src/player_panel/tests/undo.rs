//! The Player panel's own undo history, as the panel and the editor's keys reach it.
use super::*;
use hollow_steps::editor_history::HistoryKey;

fn move_speed(editor: &EditorSession) -> f32 {
    Field::MoveSpeed.get(editor.player.edited())
}

#[test]
fn a_drag_is_one_step_and_the_keys_belong_to_the_panel_only_after_a_press_in_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let saved = move_speed(editor);
    assert!(
        !state.apply_history(editor, HistoryKey::Undo),
        "nothing has been pressed in the panel yet: the keys are the room's"
    );

    // Press, drag over several frames, release: one step.
    let start = rect_of(&ctx, field_id(Field::MoveSpeed)).center();
    frame(&ctx, pointer(start, Some(true)), editor, &mut state);
    for dx in [10.0, 20.0, 30.0, 40.0] {
        frame(
            &ctx,
            pointer(start + egui::vec2(dx, 0.0), None),
            editor,
            &mut state,
        );
    }
    assert!(state.history.gesture_open(), "the drag is still going");
    assert_eq!(state.history.undo_steps(), 0);
    frame(
        &ctx,
        pointer(start + egui::vec2(40.0, 0.0), Some(false)),
        editor,
        &mut state,
    );
    frame(&ctx, vec![], editor, &mut state);
    let dragged = move_speed(editor);
    assert!(dragged > saved);
    assert_eq!(state.history.undo_steps(), 1);
    assert!(state.focused);

    // Ctrl+Z reaches the panel's history and undoes the whole drag at once.
    assert!(state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(move_speed(editor), saved);
    assert!(!editor.player.is_dirty());
    assert!(state.apply_history(editor, HistoryKey::Redo));
    assert_eq!(move_speed(editor), dragged);

    // A press anywhere else (here: outside the panel, on the canvas) gives the keys back.
    let outside = egui::pos2(PANEL_WIDTH + 200.0, 50.0);
    frame(&ctx, pointer(outside, Some(true)), editor, &mut state);
    frame(&ctx, pointer(outside, Some(false)), editor, &mut state);
    assert!(!state.focused);
    assert!(!state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(move_speed(editor), dragged, "the panel's edit stays");
}

#[test]
fn typed_values_and_resets_are_steps_and_the_buttons_undo_and_redo_them() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let saved = move_speed(editor);

    type_into(&ctx, Field::MoveSpeed, "300", editor, &mut state);
    type_into(&ctx, Field::MoveSpeed, "320", editor, &mut state);
    assert_eq!(state.history.undo_steps(), 2);
    click(
        &ctx,
        egui::Id::new(("reset", Field::MoveSpeed.spec().key)),
        editor,
        &mut state,
    );
    assert_eq!(move_speed(editor), saved);
    assert_eq!(state.history.undo_steps(), 3, "a Reset is a step too");

    let undo = egui::Id::new(("button", "Undo player edit"));
    let redo = egui::Id::new(("button", "Redo player edit"));
    click(&ctx, undo, editor, &mut state);
    assert_eq!(move_speed(editor), 320.0);
    click(&ctx, undo, editor, &mut state);
    assert_eq!(move_speed(editor), 300.0);
    click(&ctx, redo, editor, &mut state);
    assert_eq!(move_speed(editor), 320.0);
    // The undo itself did not become a step: three back is still the start.
    for _ in 0..5 {
        click(&ctx, undo, editor, &mut state);
    }
    assert_eq!(move_speed(editor), saved);
    assert!(!state.history.can_undo());
}

#[test]
fn closing_the_panel_gives_the_keys_back_to_the_room() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    type_into(&ctx, Field::MoveSpeed, "300", editor, &mut state);
    assert!(state.focused);
    state.open = false;
    frame(&ctx, vec![], editor, &mut state);
    assert!(!state.focused);
    assert!(!state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(move_speed(editor), 300.0, "nothing was undone");
}
