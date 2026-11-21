//! The hazard controls of the Room tools panel: add a hazard (`editor_hazard`) and edit the
//! selected one's size, hit and look (shape, facing, color). A hazard is moved by dragging it
//! on the canvas like an item.
use std::ops::RangeInclusive;

use super::look_panel::same_color;
use super::widgets::{choice, panel_button, panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::{tracked, History};
use hollow_steps::editor_tools::EditorTool;
use hollow_steps::hazard::{
    HazardFields, MAX_HAZARD_DAMAGE, MAX_HAZARD_KNOCKBACK_SECS, MAX_HAZARD_SPEED, MIN_HAZARD_SIZE,
};
use hollow_steps::hazard_look::{HazardFacing, HazardStyle};

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

/// Width, height, damage, the knockback and the look of hazard `index`. Dragging a field edits
/// every frame and the whole drag is one undo step, like the room size fields; so is a drag in
/// the color picker.
pub(super) fn draw_hazard_fields(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
) {
    let Some(current) = editor.draft.hazard(index).map(|hazard| hazard.fields()) else {
        return;
    };
    // Held at the start of the frame: a color change made while it is down is a drag.
    let held = ui.input(|input| input.pointer.any_down());
    section(ui, "Hazard", |ui| {
        let mut edited = current;
        let mut color_changed = false;
        let rows = egui::Grid::new("hazard fields")
            .num_columns(2)
            .show(ui, |ui| {
                let rows = [
                    float_row(ui, "Hazard width", &mut edited.width, SIZE, " px", 0, ""),
                    float_row(ui, "Hazard height", &mut edited.height, SIZE, " px", 0, ""),
                    damage_row(ui, &mut edited.damage),
                    float_row(
                        ui,
                        "Knockback speed",
                        &mut edited.knockback_speed,
                        SPEED,
                        " px/s",
                        0,
                        KNOCKBACK_HINT,
                    ),
                    float_row(
                        ui,
                        "Lift speed",
                        &mut edited.lift_speed,
                        SPEED,
                        " px/s",
                        0,
                        LIFT_HINT,
                    ),
                    float_row(
                        ui,
                        "Knockback time",
                        &mut edited.knockback_secs,
                        SECONDS,
                        " s",
                        2,
                        TIME_HINT,
                    ),
                ];
                edited.style = pick_row(
                    ui,
                    "Shape",
                    edited.style,
                    &HazardStyle::ALL,
                    HazardStyle::label,
                );
                edited.facing = pick_row(
                    ui,
                    "Facing",
                    edited.facing,
                    &HazardFacing::ALL,
                    HazardFacing::label,
                );
                color_changed = color_row(ui, &mut edited);
                rows
            });
        if rows.inner.iter().any(|row| row.drag_started()) {
            state.history.begin_gesture(&editor.draft);
        }
        let drag_id = egui::Id::new("hazard color drag");
        if color_changed && held && !state.history.gesture_open() {
            state.history.begin_gesture(&editor.draft);
            ui.data_mut(|data| data.insert_temp(drag_id, true));
        }
        apply_fields(editor, &mut state.history, index, current, edited);
        if rows.inner.iter().any(|row| row.drag_stopped()) {
            state.history.end_gesture(&editor.draft);
        }
        // The color drag this section began ends when the button comes up.
        let color_drag = ui.data(|data| data.get_temp::<bool>(drag_id)) == Some(true);
        if color_drag && !ui.input(|input| input.pointer.any_down()) {
            state.history.end_gesture(&editor.draft);
            ui.data_mut(|data| data.insert_temp(drag_id, false));
        }
        panel_label(ui, "Thrown away from the side touched.");
    });
}

const KNOCKBACK_HINT: &str = "How hard the hit throws the player away from the side touched: \
     up from the top, sideways from a side, down from below.";
const LIFT_HINT: &str = "Extra upward speed when the player is thrown sideways.";
const TIME_HINT: &str = "How long the sideways push lasts.";

/// A dropdown row: the choices by name, `current` shown; returns the one now chosen.
fn pick_row<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    current: T,
    all: &[T],
    name: fn(T) -> &'static str,
) -> T {
    panel_label(ui, label);
    let mut picked = current;
    let combo = egui::ComboBox::from_id_salt(egui::Id::new(("hazard pick", label)))
        .width(ui.available_width())
        .truncate()
        .selected_text(name(current))
        .show_ui(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for &option in all {
                if choice(ui, name(option), option == current).clicked() {
                    picked = option;
                    ui.close_menu();
                }
            }
        });
    probe(ui, || {
        (egui::Id::new(("hazard pick", label)), combo.response.rect)
    });
    ui.end_row();
    picked
}

/// The color row: a swatch showing the color in use (the hazard's own, or its shape's) and a
/// Reset that goes back to the shape's. Returns whether the picker changed the color.
fn color_row(ui: &mut egui::Ui, edited: &mut HazardFields) -> bool {
    panel_label(ui, "Color");
    let shown = edited.color.unwrap_or_else(|| edited.style.default_color());
    let mut rgb = [shown.0, shown.1, shown.2];
    let mut changed = false;
    ui.horizontal(|ui| {
        let swatch = ui.color_edit_button_rgb(&mut rgb);
        probe(ui, || (egui::Id::new(("hazard color",)), swatch.rect));
        // The picker rounds the color it is given and reports that as a change as soon as it
        // opens; that must not turn into an override of the shape's color.
        if swatch.changed() && !same_color(rgb, [shown.0, shown.1, shown.2]) {
            edited.color = Some((rgb[0], rgb[1], rgb[2]));
            changed = true;
        }
        let reset = ui
            .add_enabled(edited.color.is_some(), egui::Button::new("Reset"))
            .on_hover_text("Back to the shape's color")
            .on_disabled_hover_text("This color is the shape's");
        probe(ui, || (egui::Id::new(("hazard color reset",)), reset.rect));
        if reset.clicked() {
            edited.color = None;
        }
    });
    ui.end_row();
    changed
}

fn float_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    suffix: &str,
    decimals: usize,
    hint: &str,
) -> egui::Response {
    panel_label(ui, label);
    let mut response = ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(if decimals == 0 { 1.0 } else { 0.01 })
            .fixed_decimals(decimals)
            .suffix(suffix),
    );
    if !hint.is_empty() {
        response = response.on_hover_text(hint);
    }
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
