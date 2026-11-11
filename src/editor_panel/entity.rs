//! The Entity properties panel: the entity fields, then the body, behavior and state
//! sections, which each live in their own module.
use super::behavior::{
    draw_behavior_creation, draw_behavior_picker, draw_preset_picker, run_operation,
    select_and_guard,
};
use super::body_visual::{draw_body_visual, draw_pose_assets};
use super::death_field;
use super::key_values::draw_key_values;
use super::property::{EntityField, PropertyUi};
use super::states::draw_states_section;
use super::visual_size_field::{hitbox_side, visual_size_rows};
use super::widgets::{panel_input, panel_label, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;

/// Draws the whole panel for the selected entity and applies this frame's edits to the
/// editor's content. Anything wrong with the pending text is kept in `state.property.error`
/// and the editor message, so a collapsed section cannot hide it from Save.
pub(super) fn draw_definition_panel(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    let p = &mut state.property;
    if let Some(error) = &p.error {
        panel_label(ui, error);
    }
    let mut error = None;
    let path = p
        .path
        .clone()
        .filter(|path| editor.content.entities.contains_key(path));
    draw_entity_section(ui, p, path.is_some());
    if path.is_some() {
        draw_body_visual(ui, editor, p);
    }
    draw_pose_assets(ui, editor, p);
    draw_preset_picker(ui, editor, p, path.is_some());
    if draw_behavior_creation(ui, editor, p, path.as_deref()) {
        return;
    }
    let choice = draw_behavior_picker(ui, editor, p, path.is_some());
    if let Some(path) = &path {
        if let Err(e) = p
            .definition()
            .and_then(|entity| editor.update_entity(path, entity))
        {
            error = Some(e);
        }
    }
    if let Some(operation) = choice.operation {
        run_operation(
            ui,
            editor,
            p,
            path.as_deref(),
            operation,
            choice.selected.as_deref(),
        );
        return;
    }
    let Some(selected_behavior) = choice.selected else {
        panel_label(ui, "Select a behavior to edit its states");
        p.error = error;
        return;
    };
    if select_and_guard(ui, editor, p, selected_behavior, path.is_some()) {
        p.error = error;
        return;
    }
    draw_key_values(ui, editor, p, &mut error);
    if ui
        .checkbox(&mut p.rows, "Advanced: structured rows")
        .changed()
    {
        p.loaded_state = None;
    }
    draw_states_section(ui, editor, p, &mut error);
    if let Some(pending) = pending_error(editor, p) {
        error = Some(pending);
    }
    p.error = error.clone();
    if let Some(e) = &error {
        editor.message = e.clone();
    }
    if let Some(e) = error {
        panel_label(ui, &e);
    }
}

/// The Entity section: the shared definition's text fields and death time.
fn draw_entity_section(ui: &mut egui::Ui, p: &mut PropertyUi, has_entity: bool) {
    section(ui, "Entity", |ui| {
        if has_entity {
            panel_label(ui, "Shared definition (all rooms)");
            for field in EntityField::ALL {
                panel_input(
                    ui,
                    egui::Id::new(("entity field", field.index())),
                    field.label(),
                    p.fields.text_mut(field),
                );
                if let Err(e) = p.fields.validate(field) {
                    ui.colored_label(egui::Color32::LIGHT_RED, &e);
                }
            }
            visual_size_rows(
                ui,
                &mut p.visual_size,
                (hitbox_side(&p.fields.width), hitbox_side(&p.fields.height)),
            );
            death_field::death_secs_row(ui, &mut p.death_secs);
            panel_label(ui, &format!("Assigned behavior: {}", p.fields.behavior));
        } else {
            panel_label(ui, "Select an entity in Room tools or on the canvas");
        }
    });
}

/// The first problem anywhere in the pending edits, including sections that are collapsed or
/// filtered out, so the editor cannot save over something it did not show.
pub(super) fn pending_error(editor: &EditorSession, p: &PropertyUi) -> Option<String> {
    use hollow_steps::definition_editor::edit_state_lists;
    for (name, pose) in &editor.content.poses {
        if let Err(error) = pose.validate() {
            return Some(format!("pose {name:?}: {error}"));
        }
    }
    for (name, template) in &editor.content.bodies {
        if let Err(error) = template.validate() {
            return Some(format!("body template {name:?}: {error}"));
        }
    }
    if p.path
        .as_ref()
        .is_some_and(|path| editor.content.entities.contains_key(path))
    {
        if let Err(error) = p.definition().and_then(|entity| {
            if let Some(body) = &entity.body {
                body.validate_template(&editor.content.bodies)?;
            }
            if let Some(behavior) = editor.content.behaviors.get(&entity.behavior) {
                hollow_steps::pose::validate_entity(&entity, behavior, &editor.content.poses)?;
            }
            Ok(())
        }) {
            return Some(error);
        }
    }
    if let Some((name, values)) = &p.key_values {
        if let Some(behavior) = editor.content.behaviors.get(name) {
            let mut candidate = behavior.clone();
            if let Err(error) = values.apply(&mut candidate).and_then(|_| {
                hollow_steps::pose::validate_behavior(&candidate, &editor.content.poses)
            }) {
                return Some(error);
            }
        }
    }
    if let Some((behavior, index)) = &p.loaded_state {
        if let Some(behavior) = editor.content.behaviors.get(behavior) {
            if let Some(state) = behavior.states.get(*index) {
                let mut candidate = state.clone();
                let names = behavior
                    .states
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>();
                let result = if p.advanced {
                    edit_state_lists(&mut candidate, &p.actions, &p.transitions)
                } else {
                    p.structured.apply(&mut candidate, &names)
                };
                if let Err(error) = result {
                    return Some(error);
                }
                let mut behavior = behavior.clone();
                behavior.states[*index] = candidate;
                if let Err(error) =
                    hollow_steps::pose::validate_behavior(&behavior, &editor.content.poses)
                {
                    return Some(error);
                }
            }
        }
    }
    editor.content.definitions().err()
}
