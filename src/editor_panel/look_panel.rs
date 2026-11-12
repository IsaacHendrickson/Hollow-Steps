//! The Room look section of the Room tools panel: the terrain preset, the four colors (each
//! with a reset to the preset's), moss, the boundary switch and the seed. The rules live in
//! `hollow_steps::editor_look`; this is presentation and wiring only.
//!
//! A discrete change (preset, checkbox, reroll, a typed value) is one undo step. Dragging in
//! the color picker or on the seed field changes the look every frame; the whole drag, from the
//! first change to the release, is one step.
use super::widgets::{choice, panel_button, panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor::RoomDraft;
use hollow_steps::editor_history::{tracked, History};
use hollow_steps::editor_look::{LookColor, LookPreview};
use hollow_steps::terrain_styles::TerrainStyles;

/// What the Room look section remembers between frames.
#[derive(Default)]
pub(crate) struct LookUi {
    /// A drag this section began is open in the history.
    dragging: bool,
    /// The canvas preview's draw list, kept until the look or the terrain changes.
    pub(crate) preview: LookPreview,
}

pub(super) fn draw_look_section(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    section(ui, "Room look", |ui| {
        let style = editor.draft.layout.terrain_style.clone();
        let look = editor.look();
        let names: Vec<String> = editor
            .styles
            .names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        // Held at the start of the frame: a color or seed change made while it is down is a drag.
        let held = ui.input(|input| input.pointer.any_down());

        if !names.contains(&style.preset) {
            panel_label(
                ui,
                &format!(
                    "Preset {:?} is not in terrain_styles.ron; the canvas shows the default look.",
                    style.preset
                ),
            );
        }
        if let Some(name) = preset_picker(ui, &style.preset, &names) {
            apply(editor, state, false, |d, s| d.set_look_preset(s, &name));
        }

        for color in LookColor::ALL {
            let (red, green, blue) = color.of_look(&look);
            let shown = [red, green, blue];
            let mut rgb = shown;
            let overridden = color.overridden(&style).is_some();
            ui.horizontal(|ui| {
                ui.label(color.label());
                let swatch = ui.color_edit_button_rgb(&mut rgb);
                probe(ui, || {
                    (egui::Id::new(("look color", color.label())), swatch.rect)
                });
                if swatch.changed() && !same_color(rgb, shown) {
                    let picked = (rgb[0], rgb[1], rgb[2]);
                    apply(editor, state, held, |d, s| {
                        d.set_look_color(s, color, picked)
                    });
                }
                let reset = ui
                    .add_enabled(overridden, egui::Button::new("Reset"))
                    .on_hover_text("Back to the preset's color")
                    .on_disabled_hover_text("This color is the preset's");
                probe(ui, || {
                    (egui::Id::new(("look reset", color.label())), reset.rect)
                });
                if reset.clicked() {
                    apply(editor, state, false, |d, _| {
                        d.reset_look_color(color);
                        Ok(())
                    });
                }
            });
        }

        let mut moss = look.palette.moss;
        let response = ui.checkbox(&mut moss, "Moss");
        probe(ui, || {
            (egui::Id::new(("look checkbox", "Moss")), response.rect)
        });
        if response.changed() {
            apply(editor, state, false, |d, s| d.set_look_moss(s, moss));
        }
        let mut boundary = look.draw_boundary;
        let response = ui
            .checkbox(&mut boundary, "Draw boundary")
            .on_hover_text("The slabs around the room. They are only a picture, not collision.");
        probe(ui, || {
            (
                egui::Id::new(("look checkbox", "Draw boundary")),
                response.rect,
            )
        });
        if response.changed() {
            apply(editor, state, false, |d, _| {
                d.set_draw_boundary(boundary);
                Ok(())
            });
        }

        ui.horizontal(|ui| {
            ui.label("Seed offset");
            let mut seed = style.seed_offset;
            let field = ui.add(egui::DragValue::new(&mut seed).speed(1.0).fixed_decimals(0));
            probe(ui, || (egui::Id::new(("look seed",)), field.rect));
            if field.changed() {
                apply(editor, state, held, |d, _| {
                    d.set_seed_offset(seed);
                    Ok(())
                });
            }
        });
        if panel_button(ui, "Reroll") {
            apply(editor, state, false, |d, _| {
                d.reroll_seed();
                Ok(())
            });
        }

        // The drag ends when the button comes up; the canvas's own release check would also
        // end it, but this keeps the section self-contained.
        if state.look.dragging && !ui.input(|input| input.pointer.any_down()) {
            state.history.end_gesture(&editor.draft);
            state.look.dragging = false;
        }
    });
}

/// Colors closer than this in every channel are the same color. The picker converts the color
/// it is given to hue, saturation and value and back, and reports the rounding as a change as
/// soon as it opens; that must not turn into an override of the preset.
const SAME_COLOR: f32 = 0.0005;

fn same_color(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(a, b)| (a - b).abs() <= SAME_COLOR)
}

/// The preset dropdown; `Some(name)` when the user chose a preset other than the current one.
fn preset_picker(ui: &mut egui::Ui, current: &str, names: &[String]) -> Option<String> {
    panel_label(ui, "Preset");
    let shown = if names.iter().any(|name| name == current) {
        current.to_owned()
    } else {
        format!("{current} (unknown)")
    };
    let mut picked = None;
    let combo = egui::ComboBox::from_id_salt(egui::Id::new(("look preset",)))
        .width(ui.available_width())
        .truncate()
        .selected_text(shown)
        .show_ui(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for name in names {
                if choice(ui, name, name == current).clicked() && name != current {
                    picked = Some(name.clone());
                    ui.close_menu();
                }
            }
        });
    probe(ui, || {
        (egui::Id::new(("look preset",)), combo.response.rect)
    });
    picked
}

/// Applies a look edit made this frame and marks the session unsaved if it changed anything.
/// `held` says a drag is in progress: the first change opens one undo step that the release
/// closes. Otherwise each accepted edit is a step of its own (and a drag already open, from
/// the canvas, takes it in). A refused edit leaves the draft as it was and says why.
fn apply(
    editor: &mut EditorSession,
    state: &mut EditorUi,
    held: bool,
    edit: impl FnOnce(&mut RoomDraft, &TerrainStyles) -> Result<(), String>,
) {
    let history: &mut History = &mut state.history;
    if held && !history.gesture_open() {
        history.begin_gesture(&editor.draft);
        state.look.dragging = true;
    }
    let result = if history.gesture_open() {
        editor.edit_look(edit)
    } else {
        tracked(history, editor, |e| e.edit_look(edit))
    };
    match result {
        Ok(()) => editor.message.clear(),
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests;
