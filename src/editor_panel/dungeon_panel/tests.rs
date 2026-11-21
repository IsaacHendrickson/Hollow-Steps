use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;

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

fn press_check(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) -> String {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("button", "Check dungeon"))))
        .expect("a Check dungeon button");
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    pass(ctx, vec![], editor, state);
    pass(ctx, vec![], editor, state)
}

#[test]
fn nothing_is_listed_until_the_button_is_pressed_and_pressing_is_not_an_edit() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, text) = ready(editor, &mut state);
    assert!(text.contains("Check dungeon"), "{text}");
    assert!(!text.contains("problem"), "{text}");

    press_check(&ctx, editor, &mut state);

    assert!(state.checks.dungeon.is_some());
    assert!(!editor.dirty, "checking changes no room");
    assert!(!state.history.can_undo());
}

#[test]
fn a_dungeon_without_problems_says_so() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    assert_eq!(
        editor.check_dungeon(),
        [],
        "the fixture rooms are a valid dungeon"
    );
    let mut state = EditorUi::default();
    let (ctx, _) = ready(editor, &mut state);
    let text = press_check(&ctx, editor, &mut state);
    assert!(text.contains("No problems found"), "{text}");
}

#[test]
fn every_problem_is_listed_as_a_sentence_and_counts_the_unsaved_room() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    // The unsaved draft of the room being edited is what is checked: lock its exit with a key
    // that lies nowhere.
    editor.draft.set_exit_lock(0, Some(4)).unwrap();
    let mut state = EditorUi::default();
    let (ctx, _) = ready(editor, &mut state);

    let text = press_check(&ctx, editor, &mut state);

    assert!(text.contains("problem(s) found"), "{text}");
    assert!(text.contains("locked by Key 4"), "{text}");
    assert!(text.contains("no Key 4 can be reached"), "{text}");
    assert!(
        !editor.dirty,
        "the lock was set directly, the check did not touch it"
    );
}

#[test]
fn pressing_again_checks_again() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, _) = ready(editor, &mut state);
    let text = press_check(&ctx, editor, &mut state);
    assert!(text.contains("No problems found"), "{text}");

    editor.draft.set_exit_lock(0, Some(4)).unwrap();
    let text = press_check(&ctx, editor, &mut state);
    assert!(text.contains("Key 4"), "{text}");
    assert!(!text.contains("No problems found"), "{text}");
}
