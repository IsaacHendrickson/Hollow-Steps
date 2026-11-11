//! The Key values section: the numeric and pose parameters of the selected behavior.
use super::property::PropertyUi;
use super::widgets::{panel_input, panel_label, pose_dropdown, section};
use super::EditorSession;
use egui_macroquad::egui;

pub(super) fn draw_key_values(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut PropertyUi,
    error: &mut Option<String>,
) {
    use hollow_steps::behavior_values::KeyValues;
    let pose_names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
    let Some(behavior) = editor.content.behaviors.get_mut(&p.behavior) else {
        return;
    };
    let (_, values) = p
        .key_values
        .get_or_insert_with(|| (p.behavior.clone(), KeyValues::from_behavior(behavior)));
    section(ui, "Key values", |ui| {
        if values.values.is_empty() {
            panel_label(ui, "This behavior has no numeric parameters.");
            panel_label(
                ui,
                "Enable Advanced: structured rows to add actions, states, and transitions, or create and assign a behavior from a preset.",
            );
        }
        let mut changed = false;
        for (index, entry) in values.values.iter_mut().enumerate() {
            let before = entry.value.clone();
            if entry.pose_choice {
                pose_dropdown(
                    ui,
                    egui::Id::new(("key value", &p.behavior, index)),
                    &entry.label,
                    &mut entry.value,
                    &pose_names,
                );
            } else {
                panel_input(
                    ui,
                    egui::Id::new(("key value", &p.behavior, index)),
                    &entry.label,
                    &mut entry.value,
                );
            }
            changed |= before != entry.value;
        }
        if let Err(e) = values.apply(&mut behavior.clone()) {
            ui.colored_label(egui::Color32::LIGHT_RED, &e);
            *error = Some(e);
        } else if changed {
            let before = behavior.clone();
            values.apply(behavior).expect("validated key values");
            editor.dirty |= before != *behavior;
            p.loaded_state = None;
            editor.message.clear();
        }
    });
}
