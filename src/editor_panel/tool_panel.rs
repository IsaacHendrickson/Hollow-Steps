//! The tool selector (Select, Place, Terrain, Erase) and the Delete selected button. What
//! each tool does on the canvas lives in `hollow_steps::editor_tools` and `terrain_canvas`.
use super::{panel_button, panel_label, section, EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor::Selection;
use hollow_steps::editor_tools::EditorTool;

/// Switches tool, from a panel button or a key. Place needs an entity or item chosen in the
/// list below; without one the current tool stays and the message says why. Tools other
/// than Select act on the authored room, so they leave the arrival-spawn view.
pub(crate) fn choose_tool(editor: &mut EditorSession, state: &mut EditorUi, tool: EditorTool) {
    if tool == EditorTool::Place {
        if state.definition.selected.is_none() {
            editor.message = "Choose an entity or item in the Entity list first, then Place".into();
            return;
        }
        state.selected = None;
        state.terrain.clear();
    }
    if tool != EditorTool::Select {
        state.destination = None;
        if matches!(state.selected, Some(Selection::Destination(_))) {
            state.selected = None;
        }
    }
    state.tool = tool;
}

pub(super) fn draw_tool_section(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    section(ui, "Tools", |ui| {
        ui.horizontal_wrapped(|ui| {
            for tool in EditorTool::ALL {
                let label = format!("{} ({})", tool.label(), tool.key());
                let response = ui
                    .add(egui::Button::new(label.as_str()).selected(state.tool == tool))
                    .on_hover_text(tool.hint());
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new(("button", label.as_str())), response.rect)
                });
                if response.clicked() {
                    choose_tool(editor, state, tool);
                }
            }
        });
        panel_label(ui, state.tool.hint());
        if panel_button(ui, "Delete selected") {
            crate::terrain_canvas::delete_selected(state, editor);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{click_button, editing_game as editor, frame, laid_out};
    use super::*;
    use hollow_steps::editor_history::undo_session;
    use macroquad::math::Rect;

    #[test]
    fn every_tool_has_a_button_with_its_shortcut_and_clicking_picks_it() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let text = frame(&egui::Context::default(), vec![], editor, &mut state);
        for label in ["Select (V)", "Place (P)", "Terrain (T)", "Erase (E)"] {
            assert!(text.contains(label), "{label} missing from:\n{text}");
        }
        assert_eq!(state.tool, EditorTool::Select);
        let ctx = laid_out(editor, &mut state);
        click_button(&ctx, "Terrain (T)", editor, &mut state);
        assert_eq!(state.tool, EditorTool::Terrain);
        click_button(&ctx, "Erase (E)", editor, &mut state);
        assert_eq!(state.tool, EditorTool::Erase);
        click_button(&ctx, "Select (V)", editor, &mut state);
        assert_eq!(state.tool, EditorTool::Select);
    }

    #[test]
    fn place_needs_a_chosen_entity_and_says_so_otherwise() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi {
            selected: Some(Selection::Item(0)),
            ..EditorUi::default()
        };
        choose_tool(editor, &mut state, EditorTool::Place);
        assert_eq!(state.tool, EditorTool::Select, "the tool is unchanged");
        assert_eq!(state.selected, Some(Selection::Item(0)));
        assert!(editor.message.contains("Entity"), "{}", editor.message);

        state.definition.selected = Some(0);
        state.terrain.select(Some(0));
        choose_tool(editor, &mut state, EditorTool::Place);
        assert_eq!(state.tool, EditorTool::Place);
        assert_eq!((state.selected, state.terrain.selected()), (None, None));
    }

    #[test]
    fn terrain_and_erase_leave_the_arrival_view_but_select_stays_in_it() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi {
            destination: Some(0),
            selected: Some(Selection::Destination(0)),
            ..EditorUi::default()
        };
        choose_tool(editor, &mut state, EditorTool::Select);
        assert_eq!(state.destination, Some(0));
        choose_tool(editor, &mut state, EditorTool::Terrain);
        assert_eq!(state.destination, None);
        assert_eq!(state.selected, None);
        assert_eq!(state.tool, EditorTool::Terrain);
    }

    #[test]
    fn delete_selected_removes_a_selected_solid_in_one_undo_step() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        editor
            .draft
            .add_terrain(Rect::new(600.0, 380.0, 120.0, 20.0))
            .unwrap();
        let with_solid = editor.draft.clone();
        state.terrain.select(Some(0));
        let ctx = laid_out(editor, &mut state);

        click_button(&ctx, "Delete selected", editor, &mut state);
        assert!(editor.draft.layout.terrain.is_empty());
        assert_eq!(state.terrain.selected(), None);
        assert!(editor.dirty);

        assert_eq!(undo_session(&mut state.history, editor), Ok(true));
        assert_eq!(editor.draft, with_solid);
    }

    /// One frame of the whole panel, returning what it reported.
    fn panel(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) -> super::super::PanelResponse {
        let mut response = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 3000.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| response = Some(super::super::draw_panel(ctx, editor, state)),
        );
        response.expect("the panel ran")
    }

    #[test]
    fn the_test_room_button_reports_a_click_and_nothing_else() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let ctx = egui::Context::default();
        for _ in 0..3 {
            assert!(!panel(&ctx, vec![], editor, &mut state).test);
        }
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", "Test room (F6)"))))
            .expect("a Test room button");
        let [press, release] = super::super::test_support::click(rect.center());
        assert!(!panel(&ctx, press, editor, &mut state).test);
        let clicked = panel(&ctx, release, editor, &mut state);
        assert!(clicked.test && !clicked.save);
        assert!(!panel(&ctx, vec![], editor, &mut state).test);
        // The panel only reports the click: nothing was started or changed.
        assert!(!editor.dirty && editor.message.is_empty());
    }

    #[test]
    fn delete_selected_with_nothing_selected_changes_nothing() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let before = editor.draft.clone();
        let ctx = laid_out(editor, &mut state);
        click_button(&ctx, "Delete selected", editor, &mut state);
        assert_eq!(editor.draft, before);
        assert!(!editor.dirty && !state.history.can_undo());
    }
}
