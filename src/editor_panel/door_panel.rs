//! The door controls of the Room tools panel: add a door (`door_exit`) and set the size of the
//! selected one. A door is moved by dragging it on the canvas like any other exit.
use super::widgets::{panel_button, panel_label, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::door_exit::MIN_DOOR_SIDE;
use hollow_steps::editor_history::tracked;
use hollow_steps::editor_tools::EditorTool;

/// The widest or tallest a door can be set to in the panel; the room still has to fit it.
const MAX_DOOR_SIDE: f32 = 2000.0;

/// "Add door" and "Add door and link...": like the two exit buttons above them, but the exit
/// is a door standing at the room's spawn point (drag it where it belongs).
pub(super) fn draw_add_door(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    if panel_button(ui, "Add door") {
        let target = state
            .target
            .selected
            .and_then(|i| editor.rooms.get(i))
            .map(|(id, layout)| (*id, layout.spawn));
        match target {
            None => editor.message = "choose an exit target first".into(),
            Some((id, spawn)) => {
                let added = tracked(&mut state.history, editor, |e| {
                    e.draft.add_door(id, spawn.into())
                });
                match added {
                    Ok(selection) => {
                        state.selected = Some(selection);
                        editor.dirty = true;
                        editor.message.clear();
                        state.tool = EditorTool::Select;
                        state.destination = None;
                    }
                    Err(error) => editor.message = error,
                }
            }
        }
    }
    if panel_button(ui, "Add door and link...") {
        // Places the door and asks which room it leads to; Cancel takes it out again.
        state.start_link_new_door(editor);
    }
}

/// Width and height of exit `index` when it is a door; nothing for an edge exit. Dragging a
/// field resizes every frame and the whole drag is one undo step, like the room size fields.
pub(super) fn draw_door_size(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
) {
    let Some((_, _, width, height)) = editor
        .draft
        .layout
        .exits
        .get(index)
        .and_then(|exit| exit.door)
    else {
        return;
    };
    section(ui, "Door", |ui| {
        let mut edited = (width, height);
        let rows = egui::Grid::new("door size").num_columns(2).show(ui, |ui| {
            [
                size_row(ui, "Door width", &mut edited.0),
                size_row(ui, "Door height", &mut edited.1),
            ]
        });
        if rows.inner.iter().any(|row| row.drag_started()) {
            state.history.begin_gesture(&editor.draft);
        }
        if edited != (width, height) {
            let resize = |e: &mut EditorSession| e.draft.resize_door(index, edited.0, edited.1);
            let result = if state.history.gesture_open() {
                resize(editor)
            } else {
                tracked(&mut state.history, editor, resize)
            };
            match result {
                Ok(()) => {
                    editor.dirty = true;
                    editor.message.clear();
                }
                Err(error) => editor.message = error,
            }
        }
        if rows.inner.iter().any(|row| row.drag_stopped()) {
            state.history.end_gesture(&editor.draft);
        }
    });
}

fn size_row(ui: &mut egui::Ui, label: &str, value: &mut f32) -> egui::Response {
    panel_label(ui, label);
    let response = ui.add(
        egui::DragValue::new(value)
            .range(MIN_DOOR_SIDE..=MAX_DOOR_SIDE)
            .speed(1.0)
            .fixed_decimals(0)
            .suffix(" px"),
    );
    ui.end_row();
    response
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{click, drawn_text, editing_game as editor};
    use super::*;
    use crate::editor_panel::Selection;

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

    fn click_panel(
        ctx: &egui::Context,
        label: &str,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) {
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", label))))
            .unwrap_or_else(|| panic!("no {label} button"));
        let [press, release] = click(rect.center());
        pass(ctx, press, editor, state);
        pass(ctx, release, editor, state);
        for _ in 0..2 {
            pass(ctx, vec![], editor, state);
        }
    }

    #[test]
    fn the_panel_offers_both_door_buttons_and_explains_the_orange_marker() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let (_, text) = ready(editor, &mut EditorUi::default());
        for expected in [
            "Add door",
            "Add door and link...",
            "Orange: door (Up to enter)",
        ] {
            assert!(text.contains(expected), "missing {expected:?}");
        }
    }

    #[test]
    fn add_door_puts_a_door_in_the_room_and_is_one_undo_step() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        state.target.selected = Some(0);
        let before = editor.draft.clone();
        let (ctx, _) = ready(editor, &mut state);

        click_panel(&ctx, "Add door", editor, &mut state);
        let exits = &editor.draft.layout.exits;
        assert_eq!(exits.len(), before.layout.exits.len() + 1);
        assert!(exits.last().unwrap().is_door());
        assert_eq!(state.selected, Some(Selection::Exit(exits.len() - 1)));
        assert!(editor.dirty);

        click_panel(&ctx, "Undo", editor, &mut state);
        assert_eq!(editor.draft, before);
    }

    #[test]
    fn add_door_without_an_exit_target_says_so_and_changes_nothing() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let before = editor.draft.clone();
        let (ctx, _) = ready(editor, &mut state);

        click_panel(&ctx, "Add door", editor, &mut state);
        assert_eq!(editor.draft, before);
        assert_eq!(editor.message, "choose an exit target first");
        assert!(!editor.dirty);
    }

    #[test]
    fn door_size_fields_show_for_a_selected_door_and_not_for_an_edge_exit() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        state.target.selected = Some(0);
        let (ctx, _) = ready(editor, &mut state);
        let edge = editor
            .draft
            .layout
            .exits
            .iter()
            .position(|exit| !exit.is_door())
            .expect("the fixture room has an edge exit");

        state.selected = Some(Selection::Exit(edge));
        let text = pass(&ctx, vec![], editor, &mut state);
        assert!(!text.contains("Door width"), "{text}");

        click_panel(&ctx, "Add door", editor, &mut state);
        let text = pass(&ctx, vec![], editor, &mut state);
        assert!(
            text.contains("Door width") && text.contains("Door height"),
            "{text}"
        );
    }

    #[test]
    fn add_door_and_link_opens_the_chooser_and_cancel_takes_the_door_out_again() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let before = editor.draft.clone();
        let (ctx, _) = ready(editor, &mut state);

        click_panel(&ctx, "Add door and link...", editor, &mut state);
        assert!(editor.link_active());
        assert!(editor.draft.layout.exits.last().unwrap().is_door());

        state.cancel_link(editor);
        assert!(!editor.link_active());
        assert_eq!(editor.draft, before);
    }
}
