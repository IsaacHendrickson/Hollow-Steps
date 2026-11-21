//! The hazard controls of the Room tools panel: add a hazard (`editor_hazard`) and edit the
//! selected one's size and hit. A hazard is moved by dragging it on the canvas like an item.
use std::ops::RangeInclusive;

use super::widgets::{panel_button, panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::{tracked, History};
use hollow_steps::editor_tools::EditorTool;
use hollow_steps::hazard::{
    HazardFields, MAX_HAZARD_DAMAGE, MAX_HAZARD_KNOCKBACK_SECS, MAX_HAZARD_SPEED, MIN_HAZARD_SIZE,
};

/// The widest or tallest a hazard can be set to in the panel; the room still has to fit it.
const MAX_SIDE: f32 = 2000.0;
const SIZE: RangeInclusive<f32> = MIN_HAZARD_SIZE..=MAX_SIDE;
const SPEED: RangeInclusive<f32> = 0.0..=MAX_HAZARD_SPEED;
const SECONDS: RangeInclusive<f32> = 0.0..=MAX_HAZARD_KNOCKBACK_SECS;

/// "Add hazard": a standard hazard standing on the floor next to the room's spawn point (drag
/// it where it belongs). One undo step.
pub(super) fn draw_add_hazard(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    if !panel_button(ui, "Add hazard") {
        return;
    }
    match tracked(&mut state.history, editor, |e| e.draft.add_hazard()) {
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

/// Width, height, damage and the knockback of hazard `index`. Dragging a field edits every
/// frame and the whole drag is one undo step, like the room size fields.
pub(super) fn draw_hazard_fields(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
) {
    let Some(current) = editor.draft.hazard(index).map(|hazard| hazard.fields()) else {
        return;
    };
    section(ui, "Hazard", |ui| {
        let mut edited = current;
        let rows = egui::Grid::new("hazard fields")
            .num_columns(2)
            .show(ui, |ui| {
                [
                    float_row(ui, "Hazard width", &mut edited.width, SIZE, " px", 0),
                    float_row(ui, "Hazard height", &mut edited.height, SIZE, " px", 0),
                    damage_row(ui, &mut edited.damage),
                    float_row(
                        ui,
                        "Knockback speed",
                        &mut edited.knockback_speed,
                        SPEED,
                        " px/s",
                        0,
                    ),
                    float_row(ui, "Lift speed", &mut edited.lift_speed, SPEED, " px/s", 0),
                    float_row(
                        ui,
                        "Knockback time",
                        &mut edited.knockback_secs,
                        SECONDS,
                        " s",
                        2,
                    ),
                ]
            });
        if rows.inner.iter().any(|row| row.drag_started()) {
            state.history.begin_gesture(&editor.draft);
        }
        apply_fields(editor, &mut state.history, index, current, edited);
        if rows.inner.iter().any(|row| row.drag_stopped()) {
            state.history.end_gesture(&editor.draft);
        }
        panel_label(ui, "Pushes away from the hazard's center.");
    });
}

fn float_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    suffix: &str,
    decimals: usize,
) -> egui::Response {
    panel_label(ui, label);
    let response = ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(if decimals == 0 { 1.0 } else { 0.01 })
            .fixed_decimals(decimals)
            .suffix(suffix),
    );
    probe(ui, || {
        (egui::Id::new(("hazard field", label)), response.rect)
    });
    ui.end_row();
    response
}

fn damage_row(ui: &mut egui::Ui, value: &mut u16) -> egui::Response {
    panel_label(ui, "Damage");
    let response = ui.add(
        egui::DragValue::new(value)
            .range(1..=MAX_HAZARD_DAMAGE)
            .speed(0.2),
    );
    probe(ui, || {
        (egui::Id::new(("hazard field", "Damage")), response.rect)
    });
    ui.end_row();
    response
}

/// Applies an edit made this frame. A refused value leaves the draft as it was, so the field
/// snaps back next frame, and says why in the editor message. Outside a drag each accepted
/// edit is its own undo step; during one, the drag as a whole is recorded when it ends.
fn apply_fields(
    editor: &mut EditorSession,
    history: &mut History,
    index: usize,
    current: HazardFields,
    edited: HazardFields,
) {
    if edited == current {
        return;
    }
    let change = |e: &mut EditorSession| {
        let hazard = e.draft.hazard(index).ok_or("missing hazard")?;
        let placement = hazard.with_fields(edited);
        e.draft.replace_hazard(index, placement)
    };
    let result = if history.gesture_open() {
        change(editor)
    } else {
        tracked(history, editor, change)
    };
    match result {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
        }
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests;
