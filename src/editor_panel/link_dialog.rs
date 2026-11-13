//! The windows of the exit-link flow before B opens: the room chooser, and the name window
//! "New room..." leads to (the New room window again). What the choices do is
//! `hollow_steps::exit_link`; this only draws them.
use super::name_dialog::{draw_name_dialog, NameChoice};
use super::widgets::{panel_button, panel_label};
use super::EditorSession;
use egui_macroquad::egui;
use hollow_steps::exit_link::Step;
use hollow_steps::room::RoomId;

/// What a link window decided this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LinkChoice {
    /// A room was picked in the chooser (the room the exit is in is refused later, with a
    /// message).
    Room(RoomId),
    /// "New room..." was pressed.
    NewRoom,
    /// A good name was confirmed in the name window.
    Name(String),
    /// Cancel, Escape or a click outside the window, at either step.
    Cancel,
}

/// The chooser's text for a room: its name, and what is special about it.
fn room_caption(title: &str, unsaved: bool, is_origin: bool) -> String {
    let mut caption = title.to_owned();
    if is_origin {
        caption.push_str(" (this room)");
    } else if unsaved {
        caption.push_str(" (unsaved)");
    }
    caption
}

/// Draws the window of the flow's current step, if it is at one that has a window, and
/// returns the decision of this frame. A modal backdrop blocks the panel and canvas.
pub(crate) fn draw_link_dialog(
    ctx: &egui::Context,
    editor: &mut EditorSession,
) -> Option<LinkChoice> {
    match editor.link.as_ref()?.step() {
        Step::Choosing => draw_chooser(ctx, editor),
        Step::Naming(_) => draw_naming(ctx, editor),
        Step::Connecting(_) => None,
    }
}

fn draw_chooser(ctx: &egui::Context, editor: &EditorSession) -> Option<LinkChoice> {
    let mut choice = None;
    let width = (ctx.screen_rect().width() - 48.0).clamp(120.0, 320.0);
    let candidates = editor.link_candidates();
    let modal = egui::Modal::new(egui::Id::new("link room chooser")).show(ctx, |ui| {
        ui.set_width(width);
        ui.heading("Link exit");
        panel_label(ui, "Where should this exit lead?");
        if !editor.message.is_empty() {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(&editor.message).color(egui::Color32::LIGHT_RED),
                )
                .wrap(),
            );
        }
        egui::ScrollArea::vertical()
            .id_salt("link rooms")
            .max_height((ctx.screen_rect().height() - 220.0).clamp(72.0, 260.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                for room in &candidates {
                    let caption = room_caption(&room.title, room.unsaved, room.is_origin);
                    if panel_button(ui, caption) {
                        choice = Some(LinkChoice::Room(room.id));
                    }
                }
            });
        if panel_button(ui, "New room...") {
            choice = Some(LinkChoice::NewRoom);
        }
        if panel_button(ui, "Cancel") {
            choice = Some(LinkChoice::Cancel);
        }
    });
    if modal.should_close() {
        choice = choice.or(Some(LinkChoice::Cancel));
    }
    choice
}

/// The New room window, over the chooser's state: the prompt lives in the link.
fn draw_naming(ctx: &egui::Context, editor: &mut EditorSession) -> Option<LinkChoice> {
    let mut prompt = editor.link_prompt()?.clone();
    let choice = draw_name_dialog(ctx, editor, &mut prompt);
    if let Some(stored) = editor.link_prompt() {
        *stored = prompt;
    }
    choice.map(|choice| match choice {
        NameChoice::Create(name) => LinkChoice::Name(name),
        NameChoice::Cancel => LinkChoice::Cancel,
    })
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{click, drawn_text, editing_game};
    use super::*;
    use hollow_steps::game::GameState;

    fn frame(
        ctx: &egui::Context,
        editor: &mut EditorSession,
        events: Vec<egui::Event>,
    ) -> (Option<LinkChoice>, String) {
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
            |ctx| choice = draw_link_dialog(ctx, editor),
        );
        (choice, drawn_text(&output))
    }

    fn settle(ctx: &egui::Context, editor: &mut EditorSession) -> String {
        let mut text = String::new();
        for _ in 0..3 {
            text = frame(ctx, editor, vec![]).1;
        }
        text
    }

    fn press(ctx: &egui::Context, editor: &mut EditorSession, label: &str) -> Option<LinkChoice> {
        settle(ctx, editor);
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", label))))
            .unwrap_or_else(|| panic!("no {label} button"));
        let [down, up] = click(rect.center());
        let results = [
            frame(ctx, editor, down).0,
            frame(ctx, editor, up).0,
            frame(ctx, editor, vec![]).0,
        ];
        results.into_iter().flatten().next()
    }

    fn linking(game: &mut GameState) {
        game.editor_mut().unwrap().begin_link(0).unwrap();
    }

    #[test]
    fn nothing_is_drawn_without_a_link_or_while_b_is_open() {
        let mut game = editing_game();
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        assert_eq!(settle(&ctx, editor), "");
        editor.begin_link(0).unwrap();
        editor.link_choose(RoomId::Second).unwrap();
        assert_eq!(settle(&ctx, editor), "");
        assert_eq!(frame(&ctx, editor, vec![]).0, None);
    }

    #[test]
    fn the_chooser_lists_every_room_by_name_with_new_room_and_cancel() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let text = settle(&ctx, game.editor_mut().unwrap());
        for expected in [
            "Link exit",
            "Where should this exit lead?",
            "First (this room)",
            "Second",
            "Boss",
            "New room...",
            "Cancel",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text}");
        }
    }

    #[test]
    fn an_unsaved_room_is_marked_in_the_chooser() {
        let mut game = editing_game();
        let editor = game.editor_mut().unwrap();
        editor.begin_link(0).unwrap();
        editor.link_choose(RoomId::Second).unwrap();
        editor.link_connect(0).unwrap();
        editor.begin_link(0).unwrap();
        let ctx = egui::Context::default();
        let text = settle(&ctx, editor);
        assert!(text.contains("Second (unsaved)"), "{text}");
        assert!(text.contains("First (this room)"), "{text}");
    }

    #[test]
    fn clicking_a_room_chooses_it() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        assert_eq!(
            press(&ctx, editor, "Second"),
            Some(LinkChoice::Room(RoomId::Second))
        );
    }

    #[test]
    fn the_room_the_exit_is_in_can_be_clicked_and_reports_its_refusal() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        assert_eq!(
            press(&ctx, editor, "First (this room)"),
            Some(LinkChoice::Room(RoomId::First))
        );
        // The glue calls link_choose; its message is then shown in the window.
        let error = editor.link_choose(RoomId::First).unwrap_err();
        assert!(settle(&ctx, editor).contains(&error));
    }

    #[test]
    fn new_room_and_cancel_are_decisions() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        assert_eq!(
            press(&ctx, editor, "New room..."),
            Some(LinkChoice::NewRoom)
        );
        assert_eq!(press(&ctx, editor, "Cancel"), Some(LinkChoice::Cancel));
    }

    #[test]
    fn escape_cancels_the_chooser() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        settle(&ctx, editor);
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        let (choice, _) = frame(&ctx, editor, vec![escape]);
        assert_eq!(choice, Some(LinkChoice::Cancel));
    }

    #[test]
    fn the_name_window_replaces_the_chooser_and_confirms_a_trimmed_name() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        editor.link_new_room().unwrap();
        let text = settle(&ctx, editor);
        assert!(text.contains("Name the new room"), "{text}");
        assert!(!text.contains("Where should this exit lead?"), "{text}");
        // The suggested name is in the field and is accepted as it is.
        assert_eq!(
            press(&ctx, editor, "Create room"),
            Some(LinkChoice::Name("Room 1".into()))
        );
    }

    #[test]
    fn a_taken_name_is_refused_in_the_window_and_cancel_goes_back() {
        let mut game = editing_game();
        linking(&mut game);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        editor.link_new_room().unwrap();
        editor.link_prompt().unwrap().text = "second".into();
        assert_eq!(press(&ctx, editor, "Create room"), None);
        let text = settle(&ctx, editor);
        assert!(
            text.contains("Second") && text.contains("already"),
            "{text}"
        );
        assert_eq!(press(&ctx, editor, "Cancel"), Some(LinkChoice::Cancel));
    }
}
