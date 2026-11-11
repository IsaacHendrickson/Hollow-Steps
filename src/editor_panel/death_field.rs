//! The "Death time" drag box in the Entity section: how long a defeated enemy lingers.
use egui_macroquad::egui;
use hollow_steps::enemy::MAX_DEATH_SECS;

/// A labeled drag box for an entity's `death_secs`, limited to the range its definition
/// accepts. Dragging past either end stops there, so the value is always valid.
pub(super) fn death_secs_row(ui: &mut egui::Ui, value: &mut f32) -> egui::Response {
    ui.horizontal(|ui| {
        ui.label("Death time");
        ui.add(
            egui::DragValue::new(value)
                .range(0.0..=MAX_DEATH_SECS)
                .speed(0.01)
                .max_decimals(2)
                .suffix(" s"),
        )
        .on_hover_text(
            "How long a defeated enemy stays, harmless and fading, before it is removed. \
             0 removes it the moment its health runs out.",
        )
    })
    .inner
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{drawn_text, editing_game};
    use super::super::{draw_panel, EditorSession, EditorUi, PropertyUi};
    use super::*;
    use std::path::PathBuf;

    fn frame(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) -> String {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 3000.0),
                )),
                ..Default::default()
            },
            |ctx| {
                draw_panel(ctx, editor, state);
            },
        );
        drawn_text(&output)
    }

    /// The "chase" entity open in the property panel, with its stored death time set.
    fn open(game: &mut hollow_steps::game::GameState, death_secs: f32) -> (PathBuf, EditorUi) {
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        editor.content.entities.get_mut(&path).unwrap().death_secs = death_secs;
        let state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path.clone())),
            ..Default::default()
        };
        (path, state)
    }

    #[test]
    fn opening_an_entity_loads_its_death_time_and_the_panel_labels_it() {
        let mut game = editing_game();
        let (_, mut state) = open(&mut game, 1.25);
        assert_eq!(state.property.death_secs, 1.25);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        let text = frame(&ctx, editor, &mut state);
        assert!(text.contains("Death time"), "{text:?}");
        assert!(text.contains("1.25 s"), "{text:?}");
    }

    #[test]
    fn changing_the_drag_box_value_edits_the_shared_definition_and_marks_the_session_dirty() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, 0.4);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        assert!(!editor.dirty, "drawing alone changes nothing");

        state.property.death_secs = 2.5;
        frame(&ctx, editor, &mut state);
        assert_eq!(editor.content.entities[&path].death_secs, 2.5);
        assert!(editor.dirty);
    }

    #[test]
    fn an_ordinary_field_edit_keeps_the_stored_death_time() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, 2.0);
        state.property.fields[1] = "7".into();
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        let entity = &editor.content.entities[&path];
        assert_eq!(entity.health, 7);
        assert_eq!(entity.death_secs, 2.0, "a health edit must not reset it");
    }

    #[test]
    fn zero_is_a_valid_death_time_and_is_saved() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, 0.4);
        state.property.death_secs = 0.0;
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        assert_eq!(editor.content.entities[&path].death_secs, 0.0);
    }

    #[test]
    fn an_out_of_range_value_never_reaches_the_definition() {
        for bad in [99.0, -1.0, f32::NAN] {
            let mut game = editing_game();
            let (path, mut state) = open(&mut game, 0.4);
            state.property.death_secs = bad;
            let ctx = egui::Context::default();
            let editor = game.editor_mut().unwrap();
            frame(&ctx, editor, &mut state);
            let saved = editor.content.entities[&path].death_secs;
            assert!(
                (0.0..=MAX_DEATH_SECS).contains(&saved),
                "{bad} saved as {saved}"
            );
            assert!(
                hollow_steps::definition_editor::entity_with_body_and_death(
                    &state.property.fields,
                    state.property.body.clone(),
                    state.property.death_secs
                )
                .is_ok(),
                "{bad} left the panel holding an invalid value ({})",
                state.property.death_secs
            );
        }
    }
}
