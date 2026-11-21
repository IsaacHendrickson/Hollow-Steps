use super::super::history_panel::apply_history;
use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use crate::editor_panel::Selection;
use hollow_steps::editor_history::HistoryKey;

/// One pass of the whole panel in a tall window; returns what it drew.
fn pass(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> String {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 3000.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            super::super::draw_panel(ctx, editor, state);
        },
    );
    drawn_text(&output)
}

fn ready(editor: &mut EditorSession, state: &mut EditorUi) -> (egui::Context, String) {
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = pass(&ctx, vec![], editor, state);
    }
    (ctx, text)
}

fn click_on(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut EditorUi) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"));
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    for _ in 0..2 {
        pass(ctx, vec![], editor, state);
    }
}

fn choose(ctx: &egui::Context, label: &str, editor: &mut EditorSession, state: &mut EditorUi) {
    click_on(ctx, egui::Id::new(("exit lock",)), editor, state);
    click_on(ctx, egui::Id::new(("choice", label)), editor, state);
}

fn lock(editor: &EditorSession, index: usize) -> Option<u8> {
    editor.draft.layout.exits[index].locked_by
}

fn undo(editor: &mut EditorSession, state: &mut EditorUi) {
    apply_history(editor, state, HistoryKey::Undo);
}

fn selecting_the_edge_exit() -> EditorUi {
    EditorUi {
        selected: Some(Selection::Exit(0)),
        ..EditorUi::default()
    }
}

#[test]
fn a_selected_exit_shows_its_lock_and_nothing_is_shown_without_one() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let (_, text) = ready(editor, &mut EditorUi::default());
    assert!(!text.contains("Locked by"), "no exit selected: {text}");

    let mut state = selecting_the_edge_exit();
    let (_, text) = ready(editor, &mut state);
    assert!(text.contains("Locked by"), "{text}");
    assert!(text.contains("None"), "{text}");

    editor.draft.set_exit_lock(0, Some(3)).unwrap();
    let (_, text) = ready(editor, &mut state);
    assert!(text.contains("Key 3"), "{text}");
    assert!(!editor.dirty, "showing the lock is not an edit");
    assert!(!state.history.can_undo());
}

#[test]
fn the_dropdown_offers_none_and_every_key() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let (ctx, _) = ready(editor, &mut state);

    click_on(&ctx, egui::Id::new(("exit lock",)), editor, &mut state);
    let text = pass(&ctx, vec![], editor, &mut state);

    for number in 1..=8 {
        assert!(
            text.contains(&format!("Key {number}")),
            "Key {number}: {text}"
        );
    }
    assert!(!text.contains("Key 9") && !text.contains("Key 0"), "{text}");
    assert!(!editor.dirty, "opening the list is not an edit");
}

#[test]
fn choosing_a_key_locks_the_exit_marks_the_room_unsaved_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    choose(&ctx, "Key 3", editor, &mut state);

    assert_eq!(lock(editor, 0), Some(3));
    assert!(editor.dirty);
    assert!(state.history.can_undo());
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo(), "one step");
}

#[test]
fn choosing_none_unlocks_the_exit_and_choosing_the_same_lock_changes_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(6)).unwrap();
    let mut state = selecting_the_edge_exit();
    let (ctx, _) = ready(editor, &mut state);

    choose(&ctx, "Key 6", editor, &mut state);
    assert_eq!(lock(editor, 0), Some(6));
    assert!(!editor.dirty, "the lock it already had is not a change");
    assert!(!state.history.can_undo());

    choose(&ctx, "None", editor, &mut state);
    assert_eq!(lock(editor, 0), None);
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(lock(editor, 0), Some(6));
}

#[test]
fn a_door_is_locked_from_the_same_dropdown() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.target.selected = Some(0);
    let (ctx, _) = ready(editor, &mut state);
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("button", "Add door"))))
        .expect("an Add door button");
    let [press, release] = click(rect.center());
    pass(&ctx, press, editor, &mut state);
    pass(&ctx, release, editor, &mut state);
    pass(&ctx, vec![], editor, &mut state);
    pass(&ctx, vec![], editor, &mut state);
    let door = editor.draft.layout.exits.len() - 1;
    assert_eq!(state.selected, Some(Selection::Exit(door)));
    assert!(editor.draft.layout.exits[door].is_door());

    choose(&ctx, "Key 2", editor, &mut state);

    assert_eq!(lock(editor, door), Some(2));
    assert_eq!(lock(editor, 0), None, "no other exit is touched");
}

#[test]
fn a_refused_lock_says_why_and_changes_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let before = editor.draft.clone();

    set_lock(editor, &mut state, 0, Some(9));
    assert_eq!(editor.draft, before);
    assert!(editor.message.contains("exit lock"), "{}", editor.message);
    assert!(!editor.dirty);
    assert!(!state.history.can_undo());

    set_lock(editor, &mut state, 77, Some(1));
    assert_eq!(editor.draft, before);
    assert_eq!(editor.message, "missing exit");
}
