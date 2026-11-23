//! The "Flippable" checkbox drawn in the Room tools panel: ticking it marks the draft, it is one
//! undo step, and it marks the room unsaved.
use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use hollow_steps::editor_history::HistoryKey;

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

fn click_box(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("flip field", "flippable"))))
        .expect("the Flippable box was drawn");
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    for _ in 0..2 {
        pass(ctx, vec![], editor, state);
    }
}

#[test]
fn the_room_tools_panel_offers_the_flippable_box_unticked() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (_ctx, text) = ready(editor, &mut state);
    assert!(text.contains("Seed flip"), "{text}");
    assert!(text.contains("Flippable"), "{text}");
    assert!(!editor.draft.layout.flippable);
}

#[test]
fn ticking_the_box_marks_the_draft_and_the_room_unsaved_and_undo_clears_it() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, _) = ready(editor, &mut state);
    editor.dirty = false;

    click_box(&ctx, editor, &mut state);
    assert!(editor.draft.layout.flippable);
    assert!(editor.dirty);

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert!(!editor.draft.layout.flippable, "one undo step clears it");
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Redo);
    assert!(editor.draft.layout.flippable, "and redo sets it again");
}

#[test]
fn unticking_the_box_clears_the_mark() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.set_flippable(true).unwrap();
    let (ctx, _) = ready(editor, &mut state);
    click_box(&ctx, editor, &mut state);
    assert!(!editor.draft.layout.flippable);
}
