//! The Look section of the Player panel: the player's body as the editor edits it.
//!
//! The body panel the Entity properties view uses (`draw_body_visual`, `draw_pose_assets`)
//! draws the colors, scale, hurt feedback, template motion and poses; here it is handed the
//! player's visual as its target instead of an enemy's. What that panel does not cover is drawn
//! here: the template's parts, the pose each state plays, the run speed and the canvas preview
//! controls. The values and rules are `hollow_steps::player_look_editor`.
use std::sync::Arc;

use crate::editor_panel::body_visual::{draw_body_visual, draw_pose_assets};
use crate::editor_panel::property::PropertyUi;
use crate::editor_panel::widgets::{panel_button, panel_label, pose_dropdown, probe, section};
use egui_macroquad::egui;
use hollow_steps::body::{BodyPart, ColorSlot, Shape};
use hollow_steps::editor::EditorSession;
use hollow_steps::player_body::PlayerPose;
use hollow_steps::player_look_editor::PreviewSettings;

/// Part sizes and offsets are fractions of the player's box.
const PART_SIZE: std::ops::RangeInclusive<f32> = 0.01..=4.0;
const PART_OFFSET: std::ops::RangeInclusive<f32> = -2.0..=2.0;
const PART_ROTATION: std::ops::RangeInclusive<f32> = -360.0..=360.0;
/// Pixels per second; `PlayerLook::validate` only asks for a positive number.
const RUN_SPEED: std::ops::RangeInclusive<f32> = 1.0..=2_000.0;

/// What the Look section remembers between frames.
pub(crate) struct LookUi {
    /// The body panel's state; its `body` is the player's visual while the panel draws.
    property: PropertyUi,
    /// Which pose, facing and outline the canvas shows.
    pub(crate) preview: PreviewSettings,
    /// Draw the player at the spawn point on the canvas while the Player panel is open.
    pub(crate) show_on_canvas: bool,
}

impl Default for LookUi {
    fn default() -> Self {
        Self {
            property: PropertyUi::default(),
            preview: PreviewSettings::default(),
            show_on_canvas: true,
        }
    }
}

pub(super) fn draw(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut LookUi) {
    section(ui, "Look", |ui| {
        panel_label(
            ui,
            if editor.look.is_dirty() {
                "Colors or poses changed since the saved look."
            } else {
                "Matches the saved look."
            },
        );
        draw_preview_controls(ui, state);
        draw_visual(ui, editor, state);
        draw_state_poses(ui, editor);
        draw_parts(ui, editor);
        draw_pose_assets(ui, editor, &mut state.property);
        if let Some(error) = &state.property.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if let Some(problem) = editor
            .look
            .problem(&editor.content.bodies, &editor.content.poses)
        {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(format!("The player cannot use this look: {problem}"))
                        .color(egui::Color32::LIGHT_RED),
                )
                .wrap(),
            );
        }
        if panel_button(ui, "Reset look") {
            editor.look.reset();
        }
        ui.small(
            "Save player writes the colors and poses chosen here (player_body.ron). The body \
             template (parts, motion) and the pose files are saved with Save in the room tools.",
        );
    });
}

fn draw_preview_controls(ui: &mut egui::Ui, state: &mut LookUi) {
    let response = ui.checkbox(&mut state.show_on_canvas, "Show player on canvas");
    probe(ui, || (egui::Id::new("preview show"), response.rect));
    ui.horizontal(|ui| {
        for pose in PlayerPose::ALL {
            let response = ui.selectable_value(&mut state.preview.pose, pose, pose.label());
            probe(ui, || {
                (egui::Id::new(("preview pose", pose.label())), response.rect)
            });
        }
    });
    let response = ui.checkbox(&mut state.preview.face_left, "Face left");
    probe(ui, || (egui::Id::new("preview face left"), response.rect));
    let response = ui.checkbox(&mut state.preview.show_hitbox, "Show hitbox outline");
    probe(ui, || (egui::Id::new("preview hitbox"), response.rect));
}

/// The body panel, with the player's visual as its target. It offers "Box" (no body), which a
/// player look cannot say, so a result of `None` is dropped and the visual stays.
fn draw_visual(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut LookUi) {
    let visual = editor.look.edited().visual.clone();
    state.property.body = Some(visual.clone());
    draw_body_visual(ui, editor, &mut state.property);
    if let Some(edited) = state.property.body.take() {
        if edited != visual {
            editor.look.set_visual(edited);
        }
    }
}

/// Which pose file plays for each state, and the speed above which the run pose plays.
fn draw_state_poses(ui: &mut egui::Ui, editor: &mut EditorSession) {
    section(ui, "Poses by state", |ui| {
        let names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
        for pose in PlayerPose::ALL {
            let before = editor.look.edited().pose_name(pose).to_owned();
            let mut name = before.clone();
            pose_dropdown(
                ui,
                egui::Id::new(("player pose", pose.label())),
                &format!("{} pose", pose.label()),
                &mut name,
                &names,
            );
            if name != before {
                editor.look.set_pose_name(pose, name);
            }
        }
        let mut run_speed = editor.look.edited().run_speed;
        ui.horizontal(|ui| {
            ui.label("Run pose above");
            let response = ui.add(
                egui::DragValue::new(&mut run_speed)
                    .range(RUN_SPEED)
                    .speed(1.0)
                    .suffix(" px/s"),
            );
            probe(ui, || (egui::Id::new("run speed"), response.rect));
        });
        if run_speed != editor.look.edited().run_speed {
            editor.look.set_run_speed(run_speed);
        }
    });
}

/// The parts of the player's body template, in drawing order. The template is shared with
/// everything else that uses it and is saved by the editor's own Save.
fn draw_parts(ui: &mut egui::Ui, editor: &mut EditorSession) {
    let name = editor.look.edited().visual.template.clone();
    let Some(template) = editor.content.bodies.get_mut(&name) else {
        return;
    };
    section(ui, "Parts (shared by every user of this template)", |ui| {
        let before = Arc::clone(template);
        let edited = Arc::make_mut(template);
        for part in &mut edited.parts {
            let title = part.name.clone();
            section(ui, &title, |ui| draw_part(ui, part));
        }
        if let Err(error) = edited.validate() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        // A part is never added or removed here, so equality is the whole change check.
        editor.dirty |= *before != *template.as_ref();
    });
}

fn draw_part(ui: &mut egui::Ui, part: &mut BodyPart) {
    let id = part.name.clone();
    let pair = |ui: &mut egui::Ui,
                label: &str,
                key: &'static str,
                value: &mut (f32, f32),
                range: std::ops::RangeInclusive<f32>| {
        ui.horizontal(|ui| {
            ui.label(label);
            for (axis, number) in [("x", &mut value.0), ("y", &mut value.1)] {
                let response = ui.add(
                    egui::DragValue::new(number)
                        .range(range.clone())
                        .speed(0.005)
                        .fixed_decimals(3),
                );
                probe(ui, || {
                    (egui::Id::new(("part", &id, key, axis)), response.rect)
                });
            }
        });
    };
    ui.label(match part.shape {
        Shape::Ellipse => "Ellipse",
        Shape::Triangle => "Triangle",
        Shape::Rect => "Rectangle",
        Shape::Line => "Line (width is the thickness)",
    });
    pair(ui, "Size (box fraction)", "size", &mut part.size, PART_SIZE);
    if let Some(offset) = &mut part.offset {
        pair(ui, "Offset (box fraction)", "offset", offset, PART_OFFSET);
    }
    ui.horizontal(|ui| {
        ui.label("Rotation");
        let response = ui.add(
            egui::DragValue::new(&mut part.rotation)
                .range(PART_ROTATION)
                .speed(1.0)
                .suffix(" deg"),
        );
        probe(ui, || {
            (egui::Id::new(("part", &id, "rotation")), response.rect)
        });
    });
    egui::ComboBox::from_id_salt(("part color", &id))
        .selected_text(format!("Color: {:?}", part.color_slot))
        .show_ui(ui, |ui| {
            for slot in [
                ColorSlot::Primary,
                ColorSlot::Secondary,
                ColorSlot::Accent,
                ColorSlot::Eye,
            ] {
                ui.selectable_value(&mut part.color_slot, slot, format!("{slot:?}"));
            }
        });
}
