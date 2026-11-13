//! The "Room name" text field at the top of the Room tools panel. What a name must look like
//! is decided in `hollow_steps::room_name`; the session keeps the draft and the refused text
//! (`hollow_steps::editor_name`). An error is shown right under the field and blocks Save.
use super::widgets::panel_label;
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;

pub(super) fn draw_room_name_field(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    panel_label(ui, "Room name");
    let mut text = editor.room_name_text();
    let response = ui
        .add(
            egui::TextEdit::singleline(&mut text)
                .id(egui::Id::new("room name"))
                .desired_width(ui.available_width()),
        )
        .on_hover_text(
            "1-32 letters, digits, spaces, - or _; no other room may have the same name",
        );
    if response.changed() {
        // The whole stretch of typing is one undo step, closed when the field loses focus.
        state.history.begin_gesture(&editor.draft);
        // A refused name is reported by `name_error` below and by Save.
        let _ = editor.set_room_name_text(&text);
    }
    if response.lost_focus() {
        editor.finish_name_edit();
        state.history.end_gesture(&editor.draft);
    }
    if let Some(error) = editor.name_error() {
        ui.add(egui::Label::new(egui::RichText::new(error).color(egui::Color32::LIGHT_RED)).wrap());
    }
}

#[cfg(test)]
mod tests {
    use super::super::room_tools::draw_room_tools;
    use super::super::test_support::{click, drawn_text, editing_game};
    use super::*;
    use hollow_steps::room::RoomId;

    const FIELD: &str = "room name";

    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) -> String {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(360.0, 1600.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_room_name_field(ui, editor, state);
                });
            },
        );
        drawn_text(&output)
    }

    fn focused(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
        let ctx = egui::Context::default();
        for _ in 0..2 {
            frame(&ctx, vec![], editor, state);
        }
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(FIELD)));
        frame(&ctx, vec![], editor, state);
        ctx
    }

    fn type_text(
        ctx: &egui::Context,
        text: &str,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) -> String {
        let select_all = egui::Event::Key {
            key: egui::Key::A,
            physical_key: Some(egui::Key::A),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        };
        frame(ctx, vec![select_all], editor, state);
        let typed = if text.is_empty() {
            // Nothing to type: delete the selection instead.
            egui::Event::Key {
                key: egui::Key::Backspace,
                physical_key: Some(egui::Key::Backspace),
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }
        } else {
            egui::Event::Text(text.into())
        };
        frame(ctx, vec![typed], editor, state);
        frame(ctx, vec![], editor, state)
    }

    /// A click on empty space, which takes the focus away from the field.
    fn click_away(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) {
        let [press, release] = click(egui::pos2(300.0, 1500.0));
        frame(ctx, press, editor, state);
        frame(ctx, release, editor, state);
        frame(ctx, vec![], editor, state);
    }

    #[test]
    fn the_field_shows_the_room_name_under_its_label() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let ctx = egui::Context::default();
        let text = frame(&ctx, vec![], game.editor_mut().unwrap(), &mut state);
        assert!(text.starts_with("Room name\n"), "{text:?}");
        assert!(
            text.contains("First"),
            "the shipped room shows its display name: {text:?}"
        );
    }

    #[test]
    fn the_field_is_the_first_thing_in_the_room_tools() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let labels = editor.room_labels();
        let ctx = egui::Context::default();
        let mut open = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(360.0, 1600.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_room_tools(ui, editor, &mut state, &[], &[], &labels, &mut open);
                });
            },
        );
        let text = drawn_text(&output);
        assert!(text.starts_with("Room name\n"), "{text:?}");
        assert!(text.find("Room name") < text.find("Room\n"), "{text:?}");
    }

    #[test]
    fn typing_a_good_name_changes_the_draft_and_marks_it_unsaved() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let ctx = focused(editor, &mut state);
        assert!(!editor.dirty);
        type_text(&ctx, "Entrance Hall", editor, &mut state);
        assert_eq!(editor.draft.layout.name.as_deref(), Some("Entrance Hall"));
        assert!(editor.dirty);
        assert_eq!(editor.name_error(), None);
    }

    #[test]
    fn a_refused_name_shows_its_error_next_to_the_field_and_keeps_the_draft_name() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let ctx = focused(editor, &mut state);
        for (typed, expected) in [
            ("", "empty"),
            ("Boss", "already called"),
            ("two.words", "cannot contain"),
            (&"z".repeat(33), "at most 32"),
        ] {
            let text = type_text(&ctx, typed, editor, &mut state);
            assert!(text.contains(expected), "{typed:?}: {text:?}");
            assert_eq!(editor.draft.layout.name, None, "{typed:?}");
            assert!(editor.name_error().is_some());
        }
        // Fixing the text clears the error.
        let text = type_text(&ctx, "Fixed", editor, &mut state);
        assert!(
            !text.contains("cannot contain") && !text.contains("already"),
            "{text:?}"
        );
        assert_eq!(editor.draft.layout.name.as_deref(), Some("Fixed"));
    }

    #[test]
    fn refused_text_stays_in_the_field_after_it_loses_focus() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let ctx = focused(editor, &mut state);
        type_text(&ctx, "Second", editor, &mut state);
        click_away(&ctx, editor, &mut state);
        assert_eq!(editor.room_name_text(), "Second");
        assert!(editor.name_error().unwrap().contains("already called"));
        assert!(game.save_editor().is_err());
    }

    #[test]
    fn good_text_is_tidied_when_the_field_loses_focus() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let ctx = focused(editor, &mut state);
        type_text(&ctx, "  Crypt  ", editor, &mut state);
        assert_eq!(editor.room_name_text(), "  Crypt  ");
        click_away(&ctx, editor, &mut state);
        assert_eq!(editor.room_name_text(), "Crypt");
    }

    #[test]
    fn one_stretch_of_typing_is_one_undo_step() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let ctx = focused(editor, &mut state);
        frame(
            &ctx,
            vec![egui::Event::Text("A".into())],
            editor,
            &mut state,
        );
        frame(
            &ctx,
            vec![egui::Event::Text("b".into())],
            editor,
            &mut state,
        );
        frame(
            &ctx,
            vec![egui::Event::Text("c".into())],
            editor,
            &mut state,
        );
        assert!(
            !state.history.can_undo(),
            "undo waits while the field is being typed in"
        );
        click_away(&ctx, editor, &mut state);
        assert!(state.history.can_undo());
        assert!(hollow_steps::editor_history::undo_session(&mut state.history, editor).unwrap());
        assert_eq!(
            editor.draft.layout.name, None,
            "all three characters undone at once"
        );
        assert!(!state.history.can_undo());
    }

    #[test]
    fn nothing_is_recorded_when_nothing_was_typed() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        let editor = game.editor_mut().unwrap();
        let ctx = focused(editor, &mut state);
        click_away(&ctx, editor, &mut state);
        assert!(!state.history.can_undo());
        assert!(!editor.dirty);
        assert_eq!(editor.id, RoomId::First);
    }
}
