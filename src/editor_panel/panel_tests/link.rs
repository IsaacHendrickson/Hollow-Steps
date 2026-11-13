//! The room tools' link buttons, and the panel while an exit link is open.
use super::*;
use crate::editor_panel::test_support::drawn_text;
use hollow_steps::editor::Selection;
use hollow_steps::room::RoomId;

/// Tall enough that the whole panel is on screen without scrolling.
const SIZE: egui::Vec2 = egui::vec2(400.0, 3200.0);

fn text_of(
    ctx: &egui::Context,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> (PanelResponse, String) {
    let mut response = None;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SIZE)),
            ..Default::default()
        },
        |ctx| response = Some(draw_panel(ctx, editor, state)),
    );
    (response.unwrap(), drawn_text(&output))
}

fn laid_out(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&ctx, SIZE, vec![], editor, state);
    }
    ctx
}

fn press(ctx: &egui::Context, label: &str, editor: &mut EditorSession, state: &mut EditorUi) {
    let at = button(ctx, label).center();
    click(ctx, SIZE, at, editor, state);
}

#[test]
fn a_selected_exit_shows_where_it_leads_and_a_link_button() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    let (_, text) = text_of(&ctx, editor, &mut state);
    assert!(!text.contains("Leads to"), "no exit selected: {text}");
    assert!(text.contains("Add exit and link..."), "{text}");
    state.selected = Some(Selection::Exit(0));
    let _ = text_of(&ctx, editor, &mut state);
    let (_, text) = text_of(&ctx, editor, &mut state);
    assert!(text.contains("Leads to Second"), "{text}");
    for old in ["Add exit", "Set exit target", "Arrival spawn", "Link..."] {
        assert!(text.contains(old), "{old} is missing from {text}");
    }
}

#[test]
fn the_link_button_opens_the_room_chooser_for_that_exit() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi {
        selected: Some(Selection::Exit(0)),
        ..EditorUi::default()
    };
    let ctx = laid_out(editor, &mut state);
    press(&ctx, "Link...", editor, &mut state);
    let link = editor.link.as_ref().expect("the flow started");
    assert_eq!(link.origin_exit(), 0);
    assert_eq!(link.origin_room(), RoomId::First);
    assert!(!link.is_connecting(), "the chooser comes first");
    assert!(!editor.dirty, "choosing a room is not an edit");
}

#[test]
fn add_exit_and_link_places_an_exit_on_the_chosen_edge_and_opens_the_chooser() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.layout.exits.len();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    press(&ctx, "Add exit and link...", editor, &mut state);
    assert_eq!(editor.draft.layout.exits.len(), before + 1);
    assert!(editor.link.is_some());
    assert_eq!(state.selected, Some(Selection::Exit(before)));
    // Cancel takes the exit out again.
    state.cancel_link(editor);
    assert_eq!(editor.draft.layout.exits.len(), before);
}

#[test]
fn the_old_add_exit_button_still_adds_an_exit_without_a_link() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.layout.exits.len();
    let mut state = EditorUi::default();
    state.target.selected = Some(1);
    let ctx = laid_out(editor, &mut state);
    press(&ctx, "Add exit", editor, &mut state);
    assert_eq!(editor.draft.layout.exits.len(), before + 1);
    assert!(editor.link.is_none());
}

#[test]
fn while_b_is_open_only_the_link_panel_is_shown_and_save_and_test_are_disabled() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.start_link(editor, 0);
    state.apply_link_choice(
        editor,
        crate::editor_panel::LinkChoice::Room(RoomId::Second),
    );
    let ctx = laid_out(editor, &mut state);
    let (_, text) = text_of(&ctx, editor, &mut state);
    assert!(text.contains("Linking First exit to Second"), "{text}");
    for hidden in [
        "New room",
        "Open room",
        "Add exit",
        "Entity properties",
        "Undo",
    ] {
        assert!(!text.contains(hidden), "{hidden} should be hidden: {text}");
    }
    for shown in ["Connect", "Cancel", "New exit here", "Two-way"] {
        assert!(text.contains(shown), "{shown} is missing: {text}");
    }
    // Save and Test room are drawn but cannot be pressed.
    let mut pressed = false;
    for label in ["Save", "Test room (F6)"] {
        let at = button(&ctx, label).center();
        for events in [pointer(at, true), pointer(at, false), vec![]] {
            let response = frame(&ctx, SIZE, events, editor, &mut state);
            pressed |= response.save || response.test;
        }
    }
    assert!(!pressed, "disabled buttons report nothing");
    assert!(editor.link_active());
}

#[test]
fn the_panel_lists_the_other_unsaved_rooms() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    let (_, text) = text_of(&ctx, editor, &mut state);
    assert!(!text.contains("Unsaved:"), "{text}");
    state.start_link(editor, 0);
    state.apply_link_choice(
        editor,
        crate::editor_panel::LinkChoice::Room(RoomId::Second),
    );
    state.selected = Some(Selection::Exit(0));
    state.finish_link(editor);
    let _ = text_of(&ctx, editor, &mut state);
    let (_, text) = text_of(&ctx, editor, &mut state);
    assert!(text.contains("Unsaved: First, Second"), "{text}");
}
