//! The unsaved-changes dialog lists the player when it has unsaved work.
use super::*;
use crate::editor_panel::draw_unsaved_dialog_titled;
use hollow_steps::editor_guard::{GuardedAction, UnsavedGuard};

fn dialog_text(ctx: &egui::Context, guard: &UnsavedGuard, rooms: &[String]) -> String {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 540.0),
            )),
            ..Default::default()
        },
        |ctx| {
            draw_unsaved_dialog_titled(
                ctx,
                guard,
                &hollow_steps::room_name::default_display_name,
                rooms,
            );
        },
    );
    drawn_text(&output)
}

fn open_dialog(editor: &EditorSession) -> String {
    let rooms = editor.unsaved_rooms();
    let mut guard = UnsavedGuard::default();
    guard.request_for(GuardedAction::Close, Some(editor));
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = dialog_text(&ctx, &guard, &rooms);
    }
    text
}

#[test]
fn a_dirty_player_is_listed_in_the_dialog_alone_or_with_the_room() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    editor.player.set_field(Field::MoveSpeed, 300.0).unwrap();
    let text = open_dialog(editor);
    assert!(text.contains("Unsaved changes"), "{text}");
    assert!(text.contains("Unsaved: Player"), "{text}");
    for button in ["Save", "Discard", "Cancel"] {
        assert!(text.contains(button), "{text}");
    }

    editor.dirty = true;
    let text = open_dialog(editor);
    assert!(
        text.contains("Unsaved rooms: "),
        "the room is listed: {text}"
    );
    assert!(text.contains("Unsaved: Player"), "and the player: {text}");

    // The room alone is listed as a room, with no mention of the player.
    editor.player.reset_all();
    let text = open_dialog(editor);
    assert!(text.contains("Unsaved rooms: "), "{text}");
    assert!(!text.contains("Player"), "{text}");
}

#[test]
fn a_dialog_opened_without_a_list_shows_no_list() {
    let mut guard = UnsavedGuard::default();
    guard.request(GuardedAction::Close, true);
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = dialog_text(&ctx, &guard, &[]);
    }
    assert!(
        text.contains("Save your changes before you close the editor"),
        "{text}"
    );
    assert!(!text.contains("Unsaved rooms"), "{text}");
    assert!(!text.contains("Unsaved:"), "{text}");
}
