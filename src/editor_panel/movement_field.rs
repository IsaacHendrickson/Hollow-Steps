//! The "Movement" rows of the Entity section: ground or flying, and a flier's hover height,
//! bob size and bob speed, as drag boxes limited to what the definition accepts.
use egui_macroquad::egui;
use hollow_steps::enemy::{Movement, MAX_BOB_AMPLITUDE, MAX_BOB_SPEED, MAX_HOVER_HEIGHT};

/// Switches between ground and flying. Choosing flying on a ground enemy starts from the
/// flying defaults; choosing it again on a flier keeps its values, and so does staying on
/// ground.
pub(super) fn switch_to(movement: &mut Movement, flying: bool) {
    match (flying, movement.is_flying()) {
        (true, false) => *movement = Movement::default_flying(),
        (false, true) => *movement = Movement::Ground,
        _ => {}
    }
}

/// Draws the movement kind picker and, for a flier, its drag boxes. Dragging past either end of
/// a range stops there, so a value drawn here is always valid.
pub(super) fn movement_rows(ui: &mut egui::Ui, movement: &mut Movement) {
    let mut flying = movement.is_flying();
    ui.horizontal(|ui| {
        ui.label("Movement");
        egui::ComboBox::from_id_salt("entity movement")
            .selected_text(if flying { "Flying" } else { "Ground" })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut flying, false, "Ground");
                ui.selectable_value(&mut flying, true, "Flying");
            })
            .response
            .on_hover_text(
                "Ground enemies are pulled down to the floor and stopped by terrain. Flying \
                 enemies hover at a set height above the floor, bobbing, and ignore terrain.",
            );
    });
    switch_to(movement, flying);
    let Movement::Flying {
        hover_height,
        bob_amplitude,
        bob_speed,
    } = movement
    else {
        return;
    };
    drag_row(
        ui,
        "Hover height",
        hover_height,
        0.0..=MAX_HOVER_HEIGHT,
        0.5,
        " px",
        "Gap between the floor and the bottom of the enemy at the middle of its bob.",
    );
    drag_row(
        ui,
        "Bob amplitude",
        bob_amplitude,
        0.0..=MAX_BOB_AMPLITUDE,
        0.1,
        " px",
        "How far it rises and sinks around the hover height. 0 holds it steady.",
    );
    drag_row(
        ui,
        "Bob speed",
        bob_speed,
        0.01..=MAX_BOB_SPEED,
        0.01,
        " cycles/s",
        "Full up-and-down bobs per second.",
    );
}

fn drag_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    speed: f64,
    suffix: &str,
    hover: &str,
) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(value)
                .range(range)
                .speed(speed)
                .max_decimals(2)
                .suffix(suffix),
        )
        .on_hover_text(hover);
    });
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{drawn_text, editing_game};
    use super::super::{draw_panel, EditorSession, EditorUi, PropertyUi};
    use super::*;
    use std::path::PathBuf;

    fn flying(hover_height: f32, bob_amplitude: f32, bob_speed: f32) -> Movement {
        Movement::Flying {
            hover_height,
            bob_amplitude,
            bob_speed,
        }
    }

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

    /// The "chase" entity open in the property panel, with its stored movement set.
    fn open(game: &mut hollow_steps::game::GameState, movement: Movement) -> (PathBuf, EditorUi) {
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        editor.content.entities.get_mut(&path).unwrap().movement = movement;
        let state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path.clone())),
            ..Default::default()
        };
        (path, state)
    }

    #[test]
    fn switching_kinds_keeps_a_fliers_values_and_starts_new_fliers_from_the_defaults() {
        let mut movement = Movement::Ground;
        switch_to(&mut movement, false);
        assert_eq!(movement, Movement::Ground);
        switch_to(&mut movement, true);
        assert_eq!(movement, Movement::default_flying());
        assert!(movement.validate().is_ok());

        let mut tuned = flying(90.0, 3.0, 1.5);
        switch_to(&mut tuned, true);
        assert_eq!(tuned, flying(90.0, 3.0, 1.5), "already flying: untouched");
        switch_to(&mut tuned, false);
        assert_eq!(tuned, Movement::Ground);
    }

    #[test]
    fn a_ground_entity_shows_the_picker_but_no_flight_rows() {
        let mut game = editing_game();
        let (_, mut state) = open(&mut game, Movement::Ground);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        let text = frame(&ctx, editor, &mut state);
        assert!(text.contains("Movement"), "{text:?}");
        assert!(text.contains("Ground"), "{text:?}");
        assert!(!text.contains("Hover height"), "{text:?}");
    }

    #[test]
    fn a_flier_opens_with_its_values_and_the_panel_labels_each_one() {
        let mut game = editing_game();
        let (_, mut state) = open(&mut game, flying(56.0, 5.0, 0.7));
        assert_eq!(state.property.fields.movement, flying(56.0, 5.0, 0.7));
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        let text = frame(&ctx, editor, &mut state);
        for label in ["Flying", "Hover height", "Bob amplitude", "Bob speed"] {
            assert!(text.contains(label), "{label}: {text:?}");
        }
        for value in ["56.0 px", "5.0 px", "0.70 cycles/s"] {
            assert!(text.contains(value), "{value}: {text:?}");
        }
    }

    #[test]
    fn drawing_alone_changes_nothing_but_an_edit_reaches_the_shared_definition() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, flying(56.0, 5.0, 0.7));
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        assert!(!editor.dirty, "drawing alone changes nothing");

        state.property.fields.movement = flying(90.0, 2.0, 1.2);
        frame(&ctx, editor, &mut state);
        assert_eq!(
            editor.content.entities[&path].movement,
            flying(90.0, 2.0, 1.2)
        );
        assert!(editor.dirty);
    }

    #[test]
    fn an_ordinary_field_edit_keeps_the_stored_movement() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, flying(56.0, 5.0, 0.7));
        state.property.fields.health = "7".into();
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        let entity = &editor.content.entities[&path];
        assert_eq!(entity.health, 7);
        assert_eq!(
            entity.movement,
            flying(56.0, 5.0, 0.7),
            "a health edit must not reset it"
        );
    }

    #[test]
    fn switching_to_ground_in_the_panel_state_is_saved_as_ground() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, flying(56.0, 5.0, 0.7));
        switch_to(&mut state.property.fields.movement, false);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        assert_eq!(editor.content.entities[&path].movement, Movement::Ground);
    }

    #[test]
    fn an_out_of_range_value_never_reaches_the_definition_invalid() {
        for bad in [
            flying(-1.0, 5.0, 0.7),
            flying(56.0, f32::NAN, 0.7),
            flying(56.0, 5.0, 0.0),
            flying(MAX_HOVER_HEIGHT + 1.0, 5.0, 0.7),
            flying(f32::INFINITY, 5.0, 0.7),
        ] {
            let mut game = editing_game();
            let (path, mut state) = open(&mut game, flying(56.0, 5.0, 0.7));
            state.property.fields.movement = bad;
            let ctx = egui::Context::default();
            let editor = game.editor_mut().unwrap();
            frame(&ctx, editor, &mut state);
            // The drag boxes pull a typed or dragged value back into range, so what is saved
            // (and what the panel keeps holding) always passes the definition's own check.
            let saved = editor.content.entities[&path].movement;
            assert!(saved.validate().is_ok(), "{bad:?} saved as {saved:?}");
            assert!(
                state
                    .property
                    .fields
                    .to_definition(state.property.body.clone(), state.property.death_secs)
                    .is_ok(),
                "{bad:?} left the panel holding an invalid value ({:?})",
                state.property.fields.movement
            );
        }
    }
}
