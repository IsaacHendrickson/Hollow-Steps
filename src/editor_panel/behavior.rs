//! Choosing, creating, renaming and deleting behaviors in the Entity properties panel.
use super::property::{BehaviorCreation, BehaviorSource, PropertyUi};
use super::widgets::{panel_button, panel_input, panel_label, searchable_list, section};
use super::EditorSession;
use crate::SearchList;
use egui_macroquad::egui;
use std::path::Path;

pub(super) fn draw_preset_picker(
    ui: &mut egui::Ui,
    editor: &EditorSession,
    p: &mut PropertyUi,
    can_assign: bool,
) {
    section(ui, "New behavior from preset", |ui| {
        let labels = editor
            .content
            .presets
            .iter()
            .map(|preset| format!("{}: {}", preset.name, preset.description))
            .collect::<Vec<_>>();
        let previous = p.presets.selected;
        searchable_list(
            ui,
            egui::Id::new("presets"),
            "Preset",
            &labels,
            &mut p.presets,
        );
        if p.presets.selected != previous {
            if let Some(index) = p.presets.selected {
                p.creation = Some(BehaviorCreation {
                    source: BehaviorSource::Preset(index),
                    name: String::new(),
                    assign: can_assign,
                });
            }
        }
        if labels.is_empty() {
            panel_label(ui, "No presets in assets/presets");
        }
    });
}

/// The prompt is shared by preset instantiation and copying a shared behavior.
pub(super) fn draw_behavior_creation(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut PropertyUi,
    path: Option<&Path>,
) -> bool {
    let Some(creation) = &mut p.creation else {
        return false;
    };
    let title = match &creation.source {
        BehaviorSource::Preset(index) => {
            format!("Create from {}", editor.content.presets[*index].name)
        }
        BehaviorSource::Copy(name) => format!("Save {name} as new behavior"),
    };
    panel_label(ui, &title);
    panel_input(
        ui,
        egui::Id::new("create behavior name"),
        "Behavior name (file stem)",
        &mut creation.name,
    );
    if path.is_some() {
        ui.checkbox(
            &mut creation.assign,
            "Assign to selected entity (all its instances)",
        );
    }
    if panel_button(ui, "Create behavior") {
        let assign = path.filter(|_| creation.assign);
        let result = match &creation.source {
            BehaviorSource::Preset(index) => {
                let preset = editor.content.presets[*index].clone();
                editor
                    .content
                    .create_behavior_from_preset(&preset, &creation.name, assign)
            }
            BehaviorSource::Copy(source) => {
                editor.content.copy_behavior(source, &creation.name, assign)
            }
        };
        match result {
            Ok(()) => {
                p.behavior_list.query.clear();
                p.behavior_list.selected = editor
                    .content
                    .behaviors
                    .keys()
                    .position(|name| name == creation.name.trim());
                if let Some(path) = path {
                    p.fields.behavior = editor.content.entities[path].behavior.clone();
                }
                p.loaded_state = None;
                p.key_values = None;
                p.shared_edit = None;
                p.presets.selected = None;
                p.creation = None;
                p.error = None;
                editor.message.clear();
                editor.dirty = true;
                if let Ok(definitions) = editor.content.placement_definitions() {
                    editor.definitions = definitions;
                }
            }
            Err(error) => {
                panel_label(ui, &error);
                p.error = Some(error);
            }
        }
        return true;
    }
    if panel_button(ui, "Cancel creation") {
        p.creation = None;
        p.presets.selected = None;
        p.error = None;
    }
    false
}

/// A change to the behavior library asked for in the Behavior section.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum BehaviorOp {
    New,
    Rename,
    Delete,
}

/// What the Behavior section decided this frame: the behavior now selected in its list and
/// the library change, if any, to run before anything else is drawn.
pub(super) struct BehaviorChoice {
    pub selected: Option<String>,
    pub operation: Option<BehaviorOp>,
}

/// The Behavior section: the behavior list, "Use selected behavior" and the new / rename /
/// delete buttons. A collapsed section keeps the selection and asks for nothing.
pub(super) fn draw_behavior_picker(
    ui: &mut egui::Ui,
    editor: &EditorSession,
    p: &mut PropertyUi,
    has_entity: bool,
) -> BehaviorChoice {
    let behavior_names = editor.content.behaviors.keys().cloned().collect::<Vec<_>>();
    let mut selected = p
        .behavior_list
        .selected
        .and_then(|i| behavior_names.get(i))
        .cloned();
    let mut operation = None;
    section(ui, "Behavior", |ui| {
        searchable_list(
            ui,
            egui::Id::new(("behaviors",)),
            "Behavior",
            &behavior_names,
            &mut p.behavior_list,
        );
        selected = p
            .behavior_list
            .selected
            .and_then(|i| behavior_names.get(i))
            .cloned();
        if let Some(name) = &selected {
            if has_entity && panel_button(ui, "Use selected behavior") {
                p.fields.behavior = name.clone();
            }
        }
        panel_input(
            ui,
            egui::Id::new(("behavior name",)),
            "New / renamed behavior name",
            &mut p.behavior_name,
        );
        operation = if panel_button(ui, "New blank behavior") {
            Some(BehaviorOp::New)
        } else if ui
            .add_enabled(
                selected.as_ref().is_some_and(|name| {
                    !editor
                        .content
                        .behavior_needs_warning(name, p.shared_edit.as_deref())
                }),
                egui::Button::new("Rename behavior"),
            )
            .clicked()
        {
            Some(BehaviorOp::Rename)
        } else if panel_button(ui, "Delete behavior") {
            Some(BehaviorOp::Delete)
        } else {
            None
        };
    });
    BehaviorChoice {
        selected,
        operation,
    }
}

/// Runs the library change chosen in the Behavior section and brings the panel state in line
/// with the result; a refusal is shown and kept as the panel's error.
pub(super) fn run_operation(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut PropertyUi,
    path: Option<&Path>,
    operation: BehaviorOp,
    selected: Option<&str>,
) {
    let result = match operation {
        BehaviorOp::New => editor.content.create_blank_behavior(&p.behavior_name, path),
        BehaviorOp::Rename | BehaviorOp::Delete => selected
            .ok_or_else(|| "Select a behavior first".to_owned())
            .and_then(|name| {
                if operation == BehaviorOp::Rename {
                    editor.content.rename_behavior(name, &p.behavior_name)
                } else {
                    editor.content.delete_behavior(name)
                }
            }),
    };
    match result {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
            p.error = None;
            p.behavior_list.query.clear();
            p.behavior_list.selected = if operation == BehaviorOp::Delete {
                None
            } else {
                editor
                    .content
                    .behaviors
                    .keys()
                    .position(|name| name == p.behavior_name.trim())
            };
            if let Some(path) = path {
                p.fields.behavior = editor.content.entities[path].behavior.clone();
            }
            p.loaded_state = None;
            p.key_values = None;
            if operation == BehaviorOp::New {
                p.rows = true;
            }
        }
        Err(e) => {
            editor.message = e.clone();
            p.error = Some(e.clone());
            ui.colored_label(egui::Color32::LIGHT_RED, &e);
        }
    }
}

/// Makes `selected` the behavior being edited (a new one resets the state editor) and shows
/// who uses it. Returns true when the behavior is shared and the author has not yet chosen
/// to copy it or edit it in place, in which case nothing below it should be drawn.
pub(super) fn select_and_guard(
    ui: &mut egui::Ui,
    editor: &EditorSession,
    p: &mut PropertyUi,
    selected: String,
    has_entity: bool,
) -> bool {
    if p.behavior != selected {
        p.behavior = selected;
        p.loaded_state = None;
        p.key_values = None;
        p.shared_edit = None;
        p.states = SearchList {
            selected: Some(0),
            ..SearchList::default()
        };
    }
    ui.separator();
    let users = editor.content.behavior_users(&p.behavior);
    panel_label(ui, &format!("used by {users} entities"));
    let warning = editor
        .content
        .behavior_needs_warning(&p.behavior, p.shared_edit.as_deref());
    if warning {
        panel_label(
            ui,
            "Editing this behavior affects every entity that uses it.",
        );
        if panel_button(ui, "Save as new behavior") {
            p.creation = Some(BehaviorCreation {
                source: BehaviorSource::Copy(p.behavior.clone()),
                name: String::new(),
                assign: has_entity,
            });
        }
        if panel_button(ui, "Edit shared behavior") {
            p.shared_edit = Some(p.behavior.clone());
        }
    }
    warning
}
