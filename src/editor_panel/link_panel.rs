//! The panel shown while B is open in the exit-link flow: the banner, the Two-way checkbox,
//! "New exit here", Connect and Cancel. It replaces the room tools, so nothing else in the
//! room can be changed while the flow is open and Cancel never has to undo anything.
use super::widgets::{panel_button, panel_label, probe};
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;

/// What the panel says about the exit that is selected now.
fn selection_text(editor: &EditorSession, selected: Option<Selection>) -> String {
    match selected {
        Some(Selection::Exit(index)) if index < editor.draft.layout.exits.len() => {
            let kind = if editor.link_placed_exit() == Some(index) {
                "New exit"
            } else {
                "Exit"
            };
            let leads = editor.exit_leads_to(index).unwrap_or_default();
            format!("{kind} {} is selected; it leads to {leads} now.", index + 1)
        }
        _ => "Select an exit (outlined in yellow) or place a new one.".to_owned(),
    }
}

pub(super) fn draw_link_panel(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    let Some(banner) = editor.link_banner() else {
        return;
    };
    ui.separator();
    ui.add(egui::Label::new(egui::RichText::new(banner).strong()).wrap());
    panel_label(ui, &selection_text(editor, state.selected));
    let placed = editor.link_placed_exit();
    // A new exit has no target of its own to leave alone, so it is always two-way.
    let forced = placed.is_some() && state.selected == placed.map(Selection::Exit);
    let mut two_way = forced || editor.link_two_way_setting();
    let response = ui.add_enabled(!forced, egui::Checkbox::new(&mut two_way, "Two-way"));
    probe(ui, || {
        (egui::Id::new(("checkbox", "Two-way")), response.rect)
    });
    if response.changed() {
        editor.link_set_two_way(two_way);
    }
    panel_label(
        ui,
        if forced {
            "A new exit always leads back, so Two-way stays on."
        } else {
            "Two-way also points the chosen exit back at the first room."
        },
    );
    if panel_button(ui, "New exit here") {
        state.link_placing = !state.link_placing;
    }
    if state.link_placing {
        panel_label(ui, "Click in the room to place the exit.");
    }
    if panel_button(ui, "Connect") {
        state.finish_link(editor);
    }
    if panel_button(ui, "Cancel") {
        state.cancel_link(editor);
    }
    panel_label(
        ui,
        "Only exits can be selected, placed and dragged here. Cancel puts everything back.",
    );
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{click, drawn_text, editing_game};
    use super::*;
    use hollow_steps::game::GameState;
    use hollow_steps::room::RoomId;

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
                    egui::vec2(360.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| draw_link_panel(ui, editor, state));
            },
        );
        drawn_text(&output)
    }

    fn laid_out(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, vec![], editor, state);
        }
        ctx
    }

    fn press(
        ctx: &egui::Context,
        id: (&'static str, &'static str),
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) {
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(id)))
            .unwrap_or_else(|| panic!("no {id:?}"));
        let [down, up] = click(rect.center());
        frame(ctx, down, editor, state);
        frame(ctx, up, editor, state);
        frame(ctx, vec![], editor, state);
    }

    /// First's exit is being linked to Second, and the flow is in Second.
    fn in_second(game: &mut GameState, state: &mut EditorUi) {
        let editor = game.editor_mut().unwrap();
        state.start_link(editor, 0);
        state.apply_link_choice(
            editor,
            crate::editor_panel::LinkChoice::Room(RoomId::Second),
        );
    }

    #[test]
    fn nothing_is_drawn_without_a_link_in_b() {
        let mut game = editing_game();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let ctx = egui::Context::default();
        assert_eq!(frame(&ctx, vec![], editor, &mut state), "");
        state.start_link(editor, 0);
        assert_eq!(
            frame(&ctx, vec![], editor, &mut state),
            "",
            "still choosing"
        );
    }

    #[test]
    fn the_panel_shows_the_banner_the_hints_and_the_buttons() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        let ctx = laid_out(editor, &mut state);
        let text = frame(&ctx, vec![], editor, &mut state);
        for expected in [
            "Linking First exit to Second",
            "Select an exit (outlined in yellow) or place a new one.",
            "Two-way",
            "New exit here",
            "Connect",
            "Cancel",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text}");
        }
        assert!(!text.contains("Click in the room"), "{text}");
    }

    #[test]
    fn the_selected_exit_is_described_with_where_it_leads() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        state.selected = Some(Selection::Exit(1));
        let ctx = laid_out(editor, &mut state);
        let text = frame(&ctx, vec![], editor, &mut state);
        assert!(
            text.contains("Exit 2 is selected; it leads to Boss now."),
            "{text}"
        );
    }

    #[test]
    fn two_way_starts_on_and_can_be_turned_off() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        state.selected = Some(Selection::Exit(0));
        let ctx = laid_out(editor, &mut state);
        assert!(editor.link_two_way(), "on by default");
        press(&ctx, ("checkbox", "Two-way"), editor, &mut state);
        assert!(!editor.link_two_way(), "unchecked");
        press(&ctx, ("checkbox", "Two-way"), editor, &mut state);
        assert!(editor.link_two_way(), "checked again");
    }

    #[test]
    fn two_way_stays_on_for_a_new_exit() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        editor.link_set_two_way(false);
        state.place_link_exit(editor, macroquad::math::vec2(480.0, 44.0));
        assert_eq!(state.selected, Some(Selection::Exit(2)));
        let ctx = laid_out(editor, &mut state);
        let text = frame(&ctx, vec![], editor, &mut state);
        assert!(text.contains("New exit 3 is selected"), "{text}");
        assert!(text.contains("always leads back"), "{text}");
        // The checkbox is disabled: clicking it changes nothing.
        press(&ctx, ("checkbox", "Two-way"), editor, &mut state);
        assert!(!editor.link_two_way_setting());
        assert!(editor.link_two_way());
        // Selecting an existing exit gives the checkbox back, as the user left it.
        state.selected = Some(Selection::Exit(0));
        let text = frame(&ctx, vec![], editor, &mut state);
        assert!(text.contains("also points the chosen exit back"), "{text}");
        assert!(!text.contains("always leads back"), "{text}");
    }

    #[test]
    fn new_exit_here_arms_the_next_click_and_a_second_press_disarms_it() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        let ctx = laid_out(editor, &mut state);
        press(&ctx, ("button", "New exit here"), editor, &mut state);
        assert!(state.link_placing);
        let text = frame(&ctx, vec![], editor, &mut state);
        assert!(
            text.contains("Click in the room to place the exit."),
            "{text}"
        );
        press(&ctx, ("button", "New exit here"), editor, &mut state);
        assert!(!state.link_placing);
    }

    #[test]
    fn connect_without_a_selected_exit_says_what_to_do_and_stays_in_b() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        let ctx = laid_out(editor, &mut state);
        press(&ctx, ("button", "Connect"), editor, &mut state);
        assert!(
            editor.message.contains("Select an exit in Second"),
            "{}",
            editor.message
        );
        assert_eq!(editor.id, RoomId::Second);
        assert!(editor.link_active());
    }

    #[test]
    fn connect_links_the_selected_exit_and_goes_back_to_the_first_room() {
        let mut game = editing_game();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        state.selected = Some(Selection::Exit(0));
        let ctx = laid_out(editor, &mut state);
        press(&ctx, ("button", "Connect"), editor, &mut state);
        assert_eq!(editor.id, RoomId::First);
        assert!(!editor.link_active());
        assert_eq!(editor.message, "Linked First <-> Second");
        assert_eq!(editor.draft.layout.exits[0].target_room, RoomId::Second);
    }

    #[test]
    fn cancel_goes_back_to_the_first_room_with_nothing_changed() {
        let mut game = editing_game();
        let before = game.editor().unwrap().draft.clone();
        let mut state = EditorUi::default();
        in_second(&mut game, &mut state);
        let editor = game.editor_mut().unwrap();
        state.place_link_exit(editor, macroquad::math::vec2(480.0, 44.0));
        let ctx = laid_out(editor, &mut state);
        press(&ctx, ("button", "Cancel"), editor, &mut state);
        assert_eq!(editor.id, RoomId::First);
        assert_eq!(editor.draft, before);
        assert!(!editor.dirty && editor.parked.is_empty());
    }
}
