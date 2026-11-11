//! The Body visual and Pose assets sections of the Entity properties panel.
use super::property::PropertyUi;
use super::widgets::{panel_button, panel_input, pose_dropdown, probe, section};
use super::EditorSession;
use egui_macroquad::egui;

/// The Body visual section: template choice, damage pose, scale, colors and template motion.
pub(super) fn draw_body_visual(ui: &mut egui::Ui, editor: &mut EditorSession, p: &mut PropertyUi) {
    section(ui, "Body visual", |ui| {
        egui::ComboBox::from_id_salt("body template")
            .selected_text(p.body.as_ref().map_or("Box", |b| b.template.as_str()))
            .show_ui(ui, |ui| {
                if ui.selectable_label(p.body.is_none(), "Box").clicked() {
                    p.body = None;
                }
                for name in editor.content.bodies.keys() {
                    if ui
                        .selectable_label(
                            p.body.as_ref().is_some_and(|b| b.template == *name),
                            name,
                        )
                        .clicked()
                    {
                        if let Some(body) = &mut p.body {
                            body.template = name.clone();
                        } else {
                            p.body = Some(hollow_steps::body::BodyVisual::for_template(name));
                        }
                    }
                }
            });
        if let Some(body) = &mut p.body {
            section(ui, "Damage pose", |ui| {
                let names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
                let mut pose = body.hurt.pose.clone().unwrap_or_default();
                pose_dropdown(
                    ui,
                    egui::Id::new("hurt pose"),
                    "Hurt pose",
                    &mut pose,
                    &names,
                );
                body.hurt.pose = if pose.is_empty() { None } else { Some(pose) };
                for (label, value) in [
                    ("Hurt duration (seconds)", &mut body.hurt.duration_secs),
                    ("Hurt blend time (seconds)", &mut body.hurt.blend_secs),
                    ("White flash duration (seconds)", &mut body.hurt.flash_secs),
                    ("Flash tint strength (0..1)", &mut body.hurt.tint_strength),
                    ("Flash red (0..1)", &mut body.hurt.tint.0),
                    ("Flash green (0..1)", &mut body.hurt.tint.1),
                    ("Flash blue (0..1)", &mut body.hurt.tint.2),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value));
                    });
                }
            });
            ui.horizontal(|ui| {
                ui.label("Visual scale");
                ui.add(egui::DragValue::new(&mut body.scale));
            });
            for (label, color) in [
                ("Primary", &mut body.primary),
                ("Secondary", &mut body.secondary),
                ("Accent", &mut body.accent),
                ("Eye", &mut body.eye),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    let mut rgba = [color.0, color.1, color.2, color.3];
                    if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                        *color = (rgba[0], rgba[1], rgba[2], rgba[3]);
                    }
                });
            }
            if let Err(e) = body.validate_template(&editor.content.bodies) {
                ui.colored_label(egui::Color32::LIGHT_RED, &e);
            }
            if let Some(template) = editor.content.bodies.get_mut(&body.template) {
                let before = template.clone();
                draw_body_motion(ui, std::sync::Arc::make_mut(template));
                editor.dirty |= before != *template;
            }
        }
    });
}

/// The Pose assets section: every pose's offsets, and creating a new pose.
pub(super) fn draw_pose_assets(ui: &mut egui::Ui, editor: &mut EditorSession, p: &mut PropertyUi) {
    section(ui, "Pose assets", |ui| {
        for (name, pose) in &mut editor.content.poses {
            let before = pose.clone();
            section(ui, name, |ui| {
                let pose = std::sync::Arc::make_mut(pose);
                for semantic in hollow_steps::pose::SEMANTIC_POINTS {
                    ui.push_id((name, semantic), |ui| {
                        let mut enabled = pose.offsets.contains_key(*semantic);
                        if ui.checkbox(&mut enabled, *semantic).changed() {
                            if enabled {
                                pose.offsets.insert((*semantic).into(), (0.0, 0.0));
                            } else {
                                pose.offsets.remove(*semantic);
                            }
                        }
                        if let Some(offset) = pose.offsets.get_mut(*semantic) {
                            ui.horizontal(|ui| {
                                ui.label("Offset X (box fraction)");
                                ui.add(egui::DragValue::new(&mut offset.0));
                                ui.label("Y");
                                ui.add(egui::DragValue::new(&mut offset.1));
                            });
                        }
                    });
                }
                if let Err(error) = pose.validate() {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
            });
            editor.dirty |= before != *pose;
        }
        panel_input(
            ui,
            egui::Id::new("new pose name"),
            "New pose name",
            &mut p.new_pose_name,
        );
        if panel_button(ui, "Create pose") {
            match editor.content.create_pose(&p.new_pose_name) {
                Ok(()) => {
                    editor.dirty = true;
                    p.new_pose_name.clear();
                    p.error = None;
                }
                Err(error) => p.error = Some(error),
            }
        }
    });
}

fn draw_body_motion(ui: &mut egui::Ui, template: &mut hollow_steps::body::BodyTemplate) {
    use hollow_steps::body::{Follow, IdleMotion};
    fn follow_fields(ui: &mut egui::Ui, follow: &mut Follow) {
        ui.horizontal(|ui| {
            ui.label("Stiffness");
            ui.add(egui::DragValue::new(&mut follow.stiffness));
        });
        ui.horizontal(|ui| {
            ui.label("Damping");
            ui.add(egui::DragValue::new(&mut follow.damping));
        });
    }
    section(ui, "Pose following", |ui| {
        for (label, value) in [
            (
                "Pose settling factor",
                &mut template.pose_motion.settle_factor,
            ),
            (
                "Minimum pose blend time (seconds)",
                &mut template.pose_motion.min_blend_secs,
            ),
            (
                "Instant strike impulse time (seconds)",
                &mut template.pose_motion.instant_strike_secs,
            ),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(egui::DragValue::new(value));
            });
        }
        for semantic in hollow_steps::pose::SEMANTIC_POINTS {
            let mut name = template
                .pose_map
                .get(*semantic)
                .cloned()
                .unwrap_or_default();
            let names = template.points.keys().cloned().collect::<Vec<_>>();
            pose_dropdown(
                ui,
                egui::Id::new(("pose map", semantic)),
                semantic,
                &mut name,
                &names,
            );
            if name.is_empty() {
                template.pose_map.remove(*semantic);
            } else {
                template.pose_map.insert((*semantic).into(), name);
            }
        }
    });
    ui.label("Template motion (shared by all users)");
    follow_fields(ui, &mut template.follow);
    section(ui, "Point follow overrides", |ui| {
        for point in template.points.keys() {
            ui.push_id(point, |ui| {
                let mut enabled = template.follow_overrides.contains_key(point);
                let response = ui.checkbox(&mut enabled, point);
                probe(ui, || {
                    (egui::Id::new(("follow override", point)), response.rect)
                });
                if response.changed() {
                    if enabled {
                        template
                            .follow_overrides
                            .insert(point.clone(), template.follow);
                    } else {
                        template.follow_overrides.remove(point);
                    }
                }
                if let Some(follow) = template.follow_overrides.get_mut(point) {
                    follow_fields(ui, follow);
                }
            });
        }
    });
    section(ui, "Idle motion", |ui| {
        let mut remove = None;
        for (index, idle) in template.idle.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                egui::ComboBox::from_id_salt("idle point")
                    .selected_text(&idle.point)
                    .show_ui(ui, |ui| {
                        for name in template.points.keys() {
                            ui.selectable_value(&mut idle.point, name.clone(), name);
                        }
                    });
                for (label, value) in [
                    ("Amplitude X (box fraction)", &mut idle.amplitude.0),
                    ("Amplitude Y (box fraction)", &mut idle.amplitude.1),
                    ("Period (seconds)", &mut idle.period_secs),
                    ("Phase offset (seconds)", &mut idle.phase_offset_secs),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value));
                    });
                }
                if panel_button(ui, "Remove idle entry") {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            template.idle.remove(index);
        }
        if let Some(point) = template.points.keys().next() {
            if panel_button(ui, "Add idle entry") {
                template.idle.push(IdleMotion::for_point(point));
            }
        }
    });
    if let Err(error) = template.validate() {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}
