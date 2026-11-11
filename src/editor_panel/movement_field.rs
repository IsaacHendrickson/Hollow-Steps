//! The "Movement" rows of the Entity section: ground or flying, and a flier's hover height,
//! bob size and bob speed, as drag boxes limited to what the definition accepts.
use egui_macroquad::egui;
use hollow_steps::enemy::{
    Flap, Movement, MAX_BOB_AMPLITUDE, MAX_BOB_SPEED, MAX_FLAP_RATE, MAX_FLAP_RATE_PER_SPEED,
    MAX_HOVER_HEIGHT,
};

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

/// Draws the movement kind picker and, for a flier, its drag boxes and wing flap. Dragging past
/// either end of a range stops there, so a value drawn here is always valid. `pose_names` are
/// the poses the flap's two strokes can pick from.
pub(super) fn movement_rows(ui: &mut egui::Ui, movement: &mut Movement, pose_names: &[String]) {
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
        flap,
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
    flap_rows(ui, flap, pose_names);
}

/// The wing flap: a switch, the two stroke poses and the beat rates.
fn flap_rows(ui: &mut egui::Ui, flap: &mut Option<Flap>, pose_names: &[String]) {
    let mut on = flap.is_some();
    if ui
        .checkbox(&mut on, "Wing flap")
        .on_hover_text(
            "Alternate the body between two poses (wings up, wings down), beating faster the \
             faster the enemy flies. Needs a body.",
        )
        .changed()
    {
        *flap = on.then(Flap::default_flap);
    }
    let Some(flap) = flap else {
        return;
    };
    pose_row(ui, "Wings up pose", &mut flap.up_pose, pose_names);
    pose_row(ui, "Wings down pose", &mut flap.down_pose, pose_names);
    drag_row(
        ui,
        "Hover flap rate",
        &mut flap.hover_rate,
        0.01..=MAX_FLAP_RATE,
        0.05,
        " beats/s",
        "Wing beats per second while hovering on the spot.",
    );
    // The fastest rate can never sit below the hover rate.
    flap.max_rate = flap.max_rate.max(flap.hover_rate);
    drag_row(
        ui,
        "Flap rate per speed",
        &mut flap.rate_per_speed,
        0.0..=MAX_FLAP_RATE_PER_SPEED,
        0.001,
        " beats/s per px/s",
        "Extra wing beats per second for each pixel per second of horizontal speed.",
    );
    drag_row(
        ui,
        "Max flap rate",
        &mut flap.max_rate,
        flap.hover_rate..=MAX_FLAP_RATE,
        0.05,
        " beats/s",
        "The fastest the wings may beat, however fast the enemy flies.",
    );
}

fn pose_row(ui: &mut egui::Ui, label: &str, value: &mut String, pose_names: &[String]) {
    ui.horizontal(|ui| {
        ui.label(label);
        egui::ComboBox::from_id_salt(("flap pose", label))
            .selected_text(value.as_str())
            .show_ui(ui, |ui| {
                for name in pose_names {
                    ui.selectable_value(value, name.clone(), name);
                }
            });
    });
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
                .max_decimals(if speed < 0.01 { 3 } else { 2 })
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
            flap: None,
        }
    }

    fn flapping(flap: Flap) -> Movement {
        Movement::Flying {
            hover_height: 56.0,
            bob_amplitude: 5.0,
            bob_speed: 0.7,
            flap: Some(flap),
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

    /// Like `open`, but the entity also has a body (a flap needs one) built from the fixture
    /// template "small"; the fixture pose catalog holds the poses "hurt" and "windup".
    fn open_with_body(
        game: &mut hollow_steps::game::GameState,
        movement: Movement,
    ) -> (PathBuf, EditorUi) {
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        editor.content.entities.get_mut(&path).unwrap().body =
            Some(hollow_steps::body::BodyVisual::for_template("small"));
        open(game, movement)
    }

    fn wings() -> Flap {
        Flap {
            up_pose: "hurt".into(),
            down_pose: "windup".into(),
            hover_rate: 3.0,
            rate_per_speed: 0.02,
            max_rate: 7.0,
        }
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
            state.property.fields.movement = bad.clone();
            let ctx = egui::Context::default();
            let editor = game.editor_mut().unwrap();
            frame(&ctx, editor, &mut state);
            // The drag boxes pull a typed or dragged value back into range, so what is saved
            // (and what the panel keeps holding) always passes the definition's own check.
            let saved = editor.content.entities[&path].movement.clone();
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

    #[test]
    fn a_flap_is_shown_with_its_poses_and_rates_and_edits_reach_the_definition() {
        let mut game = editing_game();
        let (path, mut state) = open_with_body(&mut game, flapping(wings()));
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        let text = frame(&ctx, editor, &mut state);
        for label in [
            "Wing flap",
            "Wings up pose",
            "Wings down pose",
            "Hover flap rate",
            "Flap rate per speed",
            "Max flap rate",
            "3.00 beats/s",
            "0.020 beats/s per px/s",
            "7.00 beats/s",
        ] {
            assert!(text.contains(label), "{label}: {text:?}");
        }
        assert!(!editor.dirty, "drawing alone changes nothing");

        let faster = Flap {
            hover_rate: 4.0,
            max_rate: 9.0,
            ..wings()
        };
        state.property.fields.movement = flapping(faster.clone());
        frame(&ctx, editor, &mut state);
        assert_eq!(editor.content.entities[&path].movement, flapping(faster));
        assert!(editor.dirty);
    }

    #[test]
    fn a_hover_rate_above_the_max_rate_pulls_the_max_up_with_it() {
        let mut game = editing_game();
        let (path, mut state) = open_with_body(&mut game, flapping(wings()));
        state.property.fields.movement = flapping(Flap {
            hover_rate: 12.0,
            max_rate: 7.0,
            ..wings()
        });
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        let saved = editor.content.entities[&path].movement.clone();
        assert!(saved.validate().is_ok(), "{saved:?}");
        let flap = saved.flap().expect("still flapping");
        assert!(flap.max_rate >= flap.hover_rate, "{flap:?}");
    }

    #[test]
    fn a_flap_without_a_body_or_naming_a_missing_pose_is_reported_and_not_saved() {
        // No body: the fixture entity has none.
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, Movement::default_flying());
        state.property.fields.movement = flapping(wings());
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        assert_eq!(
            editor.content.entities[&path].movement,
            Movement::default_flying(),
            "a flap with no body to play on must not be saved"
        );
        let error = state.property.error.clone().unwrap_or_default();
        assert!(error.contains("body"), "{error:?}");

        // A body, but a pose the catalog does not have.
        let mut game = editing_game();
        let (path, mut state) = open_with_body(&mut game, Movement::default_flying());
        state.property.fields.movement = flapping(Flap {
            up_pose: "no_such_pose".into(),
            ..wings()
        });
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        assert_eq!(
            editor.content.entities[&path].movement,
            Movement::default_flying()
        );
        let error = state.property.error.clone().unwrap_or_default();
        assert!(error.contains("no_such_pose"), "{error:?}");
    }
}
