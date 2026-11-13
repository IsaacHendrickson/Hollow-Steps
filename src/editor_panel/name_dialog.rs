//! The "New room" window: it asks for the new room's name before anything else happens.
//! The rules and the suggested name come from `hollow_steps::room_name` and the session.
use super::widgets::{panel_button, panel_label};
use super::EditorSession;
use egui_macroquad::egui;
use hollow_steps::room_name::NamePrompt;

/// What the window decided this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NameChoice {
    /// A good name was confirmed (OK or Enter); it is already trimmed.
    Create(String),
    /// Cancel, Escape or a click outside the window.
    Cancel,
}

/// Draws the window for `prompt` and returns the decision of this frame. A modal backdrop
/// blocks the panel and canvas behind it. A refused name stays in `prompt.error`, shown
/// under the text field, and the window stays open.
pub(crate) fn draw_name_dialog(
    ctx: &egui::Context,
    editor: &EditorSession,
    prompt: &mut NamePrompt,
) -> Option<NameChoice> {
    let mut choice = None;
    let width = (ctx.screen_rect().width() - 48.0).clamp(120.0, 320.0);
    // The new room is a room that does not exist yet, so no id of an existing room fits.
    let id = editor.free_room_id();
    let modal = egui::Modal::new(egui::Id::new("new room name")).show(ctx, |ui| {
        ui.set_width(width);
        ui.heading("New room");
        panel_label(ui, "Name the new room");
        let field = egui::Id::new("new room name field");
        let response = ui.add(
            egui::TextEdit::singleline(&mut prompt.text)
                .id(field)
                .desired_width(ui.available_width()),
        );
        let enter = response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if ui.memory(|memory| memory.focused().is_none()) {
            // Typing can start at once, and goes back to the field after a click elsewhere.
            response.request_focus();
        }
        let live = prompt.live_error(|text| editor.check_room_name(id, text));
        if response.changed() {
            prompt.error = None;
        }
        if let Some(error) = prompt.error.clone().or(live) {
            ui.add(
                egui::Label::new(egui::RichText::new(error).color(egui::Color32::LIGHT_RED)).wrap(),
            );
        }
        let create = panel_button(ui, "Create room");
        if panel_button(ui, "Cancel") {
            choice = Some(NameChoice::Cancel);
        }
        if (create || enter) && choice.is_none() {
            choice = prompt
                .confirm(|text| editor.check_room_name(id, text))
                .map(NameChoice::Create);
        }
    });
    if modal.should_close() {
        choice = choice.or(Some(NameChoice::Cancel));
    }
    choice
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{click, drawn_text, editing_game};
    use super::*;
    use hollow_steps::game::GameState;

    fn frame(
        ctx: &egui::Context,
        editor: &EditorSession,
        prompt: &mut NamePrompt,
        events: Vec<egui::Event>,
    ) -> (Option<NameChoice>, String) {
        let mut choice = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 540.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| choice = draw_name_dialog(ctx, editor, prompt),
        );
        (choice, drawn_text(&output))
    }

    fn open(game: &GameState) -> (egui::Context, NamePrompt) {
        let editor = game.editor().unwrap();
        let mut prompt = NamePrompt::new(editor.suggested_room_name());
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, editor, &mut prompt, vec![]);
        }
        (ctx, prompt)
    }

    fn pressed(
        ctx: &egui::Context,
        game: &GameState,
        prompt: &mut NamePrompt,
        label: &str,
    ) -> Option<NameChoice> {
        // Let the layout settle first: an error line above the buttons moves them.
        for _ in 0..2 {
            frame(ctx, game.editor().unwrap(), prompt, vec![]);
        }
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", label))))
            .unwrap_or_else(|| panic!("no {label} button"));
        let editor = game.editor().unwrap();
        let [press, release] = click(rect.center());
        let results = [
            frame(ctx, editor, prompt, press).0,
            frame(ctx, editor, prompt, release).0,
            frame(ctx, editor, prompt, vec![]).0,
        ];
        results.into_iter().flatten().next()
    }

    fn key(key: egui::Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }
    }

    #[test]
    fn the_window_asks_for_a_name_and_starts_with_the_default() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        let (choice, text) = frame(&ctx, game.editor().unwrap(), &mut prompt, vec![]);
        assert_eq!(choice, None);
        for expected in [
            "New room",
            "Name the new room",
            "Room 1",
            "Create room",
            "Cancel",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text:?}");
        }
        assert_eq!(prompt.text, "Room 1");
    }

    #[test]
    fn create_room_returns_the_default_name_untouched() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        assert_eq!(
            pressed(&ctx, &game, &mut prompt, "Create room"),
            Some(NameChoice::Create("Room 1".into()))
        );
    }

    #[test]
    fn a_typed_name_is_returned_trimmed() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        prompt.text = String::new();
        let editor = game.editor().unwrap();
        frame(
            &ctx,
            editor,
            &mut prompt,
            vec![egui::Event::Text("  Crypt ".into())],
        );
        assert_eq!(
            prompt.text, "  Crypt ",
            "typing reaches the field without a click"
        );
        assert_eq!(
            pressed(&ctx, &game, &mut prompt, "Create room"),
            Some(NameChoice::Create("Crypt".into()))
        );
    }

    #[test]
    fn enter_confirms_a_good_name() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        let editor = game.editor().unwrap();
        let (choice, _) = frame(&ctx, editor, &mut prompt, vec![key(egui::Key::Enter)]);
        let (later, _) = frame(&ctx, editor, &mut prompt, vec![]);
        assert_eq!(choice.or(later), Some(NameChoice::Create("Room 1".into())));
    }

    #[test]
    fn a_refused_name_shows_its_error_and_the_window_stays_open() {
        for (typed, expected) in [
            ("", "empty"),
            ("second", "already called"),
            ("a/b", "cannot contain"),
            (&"q".repeat(33), "at most 32"),
        ] {
            let game = editing_game();
            let (ctx, mut prompt) = open(&game);
            prompt.text = typed.to_owned();
            assert_eq!(
                pressed(&ctx, &game, &mut prompt, "Create room"),
                None,
                "{typed:?}"
            );
            let (choice, text) = frame(&ctx, game.editor().unwrap(), &mut prompt, vec![]);
            assert_eq!(choice, None);
            assert!(text.contains(expected), "{typed:?}: {text:?}");
            assert!(prompt.error.is_some());
            assert_eq!(prompt.text, typed, "the text stays so it can be fixed");
        }
    }

    #[test]
    fn a_name_that_breaks_a_rule_is_flagged_while_it_is_typed() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        prompt.text = "Boss".into();
        let (_, text) = frame(&ctx, game.editor().unwrap(), &mut prompt, vec![]);
        assert!(text.contains("already called"), "{text:?}");
        prompt.text = "Cellar".into();
        let (_, text) = frame(&ctx, game.editor().unwrap(), &mut prompt, vec![]);
        assert!(!text.contains("already called"), "{text:?}");
    }

    #[test]
    fn the_name_must_differ_from_the_unsaved_draft_too() {
        let mut game = editing_game();
        game.editor_mut()
            .unwrap()
            .set_room_name_text("Cellar")
            .unwrap();
        let (ctx, mut prompt) = open(&game);
        prompt.text = "cellar".into();
        assert_eq!(pressed(&ctx, &game, &mut prompt, "Create room"), None);
        assert!(prompt.error.as_deref().unwrap().contains("already called"));
    }

    #[test]
    fn cancel_escape_and_a_click_outside_cancel() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        assert_eq!(
            pressed(&ctx, &game, &mut prompt, "Cancel"),
            Some(NameChoice::Cancel)
        );

        let (ctx, mut prompt) = open(&game);
        let (choice, _) = frame(
            &ctx,
            game.editor().unwrap(),
            &mut prompt,
            vec![key(egui::Key::Escape)],
        );
        assert_eq!(choice, Some(NameChoice::Cancel));

        let (ctx, mut prompt) = open(&game);
        let editor = game.editor().unwrap();
        let [press, release] = click(egui::pos2(6.0, 6.0));
        let first = frame(&ctx, editor, &mut prompt, press).0;
        let second = frame(&ctx, editor, &mut prompt, release).0;
        assert_eq!(first.or(second), Some(NameChoice::Cancel));
    }

    #[test]
    fn idle_frames_decide_nothing() {
        let game = editing_game();
        let (ctx, mut prompt) = open(&game);
        for _ in 0..3 {
            assert_eq!(
                frame(&ctx, game.editor().unwrap(), &mut prompt, vec![]).0,
                None
            );
        }
    }
}
