//! The structured States section: state list, rename, initial state, raw RON and the
//! action and transition rows.
use super::property::PropertyUi;
use super::widgets::{
    panel_button, panel_combo, panel_input, panel_label, pose_dropdown, row_controls,
    searchable_list, section,
};
use super::EditorSession;
use crate::SearchList;
use egui_macroquad::egui;

/// The States section, shown when the author turned on Advanced: structured rows. Problems go
/// to `error`; a structural change drops the cached key values.
pub(super) fn draw_states_section(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut PropertyUi,
    error: &mut Option<String>,
) {
    use hollow_steps::definition_editor::{
        add_state, edit_state_lists, remove_state, rename_state, serialize,
    };
    let pose_names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
    if p.rows {
        let original = editor.content.behaviors.get(&p.behavior).cloned();
        section(ui, "States", |ui| {
            if let Some(behavior) = editor.content.behaviors.get_mut(&p.behavior) {
                let mut names = behavior
                    .states
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>();
                searchable_list(
                    ui,
                    egui::Id::new(("behavior states",)),
                    "State",
                    &names,
                    &mut p.states,
                );
                panel_input(
                    ui,
                    egui::Id::new(("new state",)),
                    "New state",
                    &mut p.new_state,
                );
                if panel_button(ui, "Add state") {
                    match add_state(behavior, p.new_state.clone()) {
                        Ok(()) => {
                            p.states.query.clear();
                            p.states.selected = Some(behavior.states.len() - 1);
                            p.loaded_state = None;
                            editor.dirty = true;
                            names = behavior.states.iter().map(|s| s.name.clone()).collect();
                        }
                        Err(e) => *error = Some(e),
                    }
                }
                let Some(index) = p.states.selected else {
                    panel_label(ui, "Select a state to edit");
                    return;
                };
                let key = (p.behavior.clone(), index);
                if p.loaded_state.as_ref() != Some(&key) {
                    let selected = &behavior.states[index];
                    p.state_name = selected.name.clone();
                    p.structured =
                        hollow_steps::definition_editor::StateInputs::from_state(selected);
                    p.conditions = p
                        .structured
                        .transitions
                        .iter()
                        .map(|row| SearchList {
                            selected: Some(row.kind),
                            ..SearchList::default()
                        })
                        .collect();
                    p.actions = serialize(&selected.actions).expect("serializable actions");
                    p.transitions =
                        serialize(&selected.transitions).expect("serializable transitions");
                    p.loaded_state = Some(key);
                }
                panel_input(
                    ui,
                    egui::Id::new(("state name",)),
                    "State name",
                    &mut p.state_name,
                );
                if panel_button(ui, "Rename state") {
                    match rename_state(behavior, index, p.state_name.clone()) {
                        Ok(()) => {
                            editor.dirty = true;
                            p.structured = hollow_steps::definition_editor::StateInputs::from_state(
                                &behavior.states[index],
                            );
                            names = behavior.states.iter().map(|s| s.name.clone()).collect();
                            p.transitions = serialize(&behavior.states[index].transitions)
                                .expect("serializable transitions");
                        }
                        Err(e) => *error = Some(e),
                    }
                }
                if panel_button(ui, "Make initial") {
                    hollow_steps::definition_editor::set_initial_state(behavior, index)
                        .expect("selected state");
                    editor.dirty = true;
                }
                panel_label(ui, &format!("Initial: {}", behavior.initial_state));
                let before = behavior.states[index].clone();
                if panel_button(ui, "Advanced: raw RON") {
                    p.advanced = !p.advanced;
                    // Switching views always starts from the last valid model.
                    p.actions = serialize(&before.actions).expect("actions");
                    p.transitions = serialize(&before.transitions).expect("transitions");
                    p.structured =
                        hollow_steps::definition_editor::StateInputs::from_state(&before);
                    p.conditions = p
                        .structured
                        .transitions
                        .iter()
                        .map(|row| SearchList {
                            selected: Some(row.kind),
                            ..SearchList::default()
                        })
                        .collect();
                }
                if p.advanced {
                    section(ui, "Actions", |ui| {
                        panel_label(ui, "Ordered actions (RON list)");
                        ui.add(
                            egui::TextEdit::multiline(&mut p.actions)
                                .id(egui::Id::new("actions"))
                                .desired_width(ui.available_width())
                                .desired_rows(6)
                                .code_editor(),
                        );
                    });
                    section(ui, "Transitions", |ui| {
                        panel_label(ui, "Transitions (RON list)");
                        ui.add(
                            egui::TextEdit::multiline(&mut p.transitions)
                                .id(egui::Id::new("transitions"))
                                .desired_width(ui.available_width())
                                .desired_rows(6)
                                .code_editor(),
                        );
                    });
                    if let Err(e) =
                        edit_state_lists(&mut behavior.states[index], &p.actions, &p.transitions)
                    {
                        *error = Some(e);
                    }
                } else {
                    draw_state_rows(ui, p, &names, index, &pose_names);
                    if let Err(e) = p.structured.apply(&mut behavior.states[index], &names) {
                        *error = Some(e);
                    }
                }
                editor.dirty |= before != behavior.states[index];
                if panel_button(ui, "Remove state") {
                    match remove_state(behavior, index) {
                        Ok(()) => {
                            p.states.selected = None;
                            p.loaded_state = None;
                            editor.dirty = true;
                        }
                        Err(e) => *error = Some(e),
                    }
                }
            } else {
                *error = Some(format!("Missing behavior file {:?}", p.behavior));
            }
        });
        if original.as_ref() != editor.content.behaviors.get(&p.behavior) {
            // Structural changes invalidate numeric addresses and labels.
            p.key_values = None;
        }
    }
}

fn draw_state_rows(
    ui: &mut egui::Ui,
    p: &mut PropertyUi,
    names: &[String],
    state_index: usize,
    pose_names: &[String],
) {
    use hollow_steps::behavior::{Action, Condition};
    use hollow_steps::definition_editor::{move_row, remove_row};
    let action_names = Action::TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    let condition_names = Condition::TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    section(ui, "Actions", |ui| {
        panel_label(ui, "Actions (in order)");
        let mut command = None;
        for (i, row) in p.structured.actions.iter_mut().enumerate() {
            ui.separator();
            panel_label(ui, &format!("{}. {}", i + 1, Action::TYPES[row.kind]));
            let template = Action::defaults(row.kind).expect("catalog action");
            for (j, label) in template.parameter_labels().iter().enumerate() {
                if hollow_steps::pose::is_pose_parameter(template.parameters()[j].0) {
                    pose_dropdown(
                        ui,
                        egui::Id::new(("action value", i, j)),
                        label,
                        &mut row.values[j],
                        pose_names,
                    );
                } else {
                    panel_input(
                        ui,
                        egui::Id::new(("action value", i, j)),
                        label,
                        &mut row.values[j],
                    );
                }
                let mut check = template.clone();
                if let Err(error) = check.set_parameter(j, &row.values[j]) {
                    ui.colored_label(egui::Color32::LIGHT_RED, &error);
                }
            }
            if let Some(op) = row_controls(ui) {
                command = Some((i, op));
            }
        }
        if let Some((i, op)) = command {
            if op == 2 {
                let _ = remove_row(&mut p.structured.actions, i);
            } else {
                let _ = move_row(&mut p.structured.actions, i, op == 0);
            }
        }
        searchable_list(
            ui,
            egui::Id::new(("add action kind",)),
            "Action type",
            &action_names,
            &mut p.add_action,
        );
        if panel_button(ui, "Add action") {
            if let Some(kind) = p.add_action.selected {
                p.structured.add_action(kind).expect("catalog");
            }
        }
    });
    section(ui, "Transitions", |ui| {
        panel_label(ui, "Transitions (first match wins)");
        let mut command = None;
        for (i, row) in p.structured.transitions.iter_mut().enumerate() {
            ui.separator();
            panel_label(ui, &format!("Transition {}", i + 1));
            let mut target = names.iter().position(|n| *n == row.target).unwrap_or(0);
            panel_combo(
                ui,
                egui::Id::new(("transition target", i)),
                "Target",
                names,
                &mut target,
            );
            row.target = names[target].clone();
            let filter = &mut p.conditions[i];
            searchable_list(
                ui,
                egui::Id::new(("condition kind", i)),
                "Condition",
                &condition_names,
                filter,
            );
            if let Some(kind) = filter.selected {
                if kind != row.kind {
                    row.change_kind(kind).expect("catalog");
                }
            }
            panel_label(ui, &format!("Current: {}", Condition::TYPES[row.kind]));
            if let Some((label, _)) = Condition::defaults(row.kind).expect("catalog").parameter() {
                panel_input(
                    ui,
                    egui::Id::new(("condition value", i)),
                    label,
                    &mut row.value,
                );
                if let Err(error) = row.build(names) {
                    ui.colored_label(egui::Color32::LIGHT_RED, &error);
                }
            }
            ui.checkbox(&mut row.invert, "Invert");
            if let Some(op) = row_controls(ui) {
                command = Some((i, op));
            }
        }
        if let Some((i, op)) = command {
            if op == 2 {
                let _ = remove_row(&mut p.structured.transitions, i);
                let _ = remove_row(&mut p.conditions, i);
            } else {
                let _ = move_row(&mut p.structured.transitions, i, op == 0);
                let _ = move_row(&mut p.conditions, i, op == 0);
            }
        }
        searchable_list(
            ui,
            egui::Id::new(("add condition kind",)),
            "New condition",
            &condition_names,
            &mut p.add_condition,
        );
        if panel_button(ui, "Add transition") {
            if let Some(kind) = p.add_condition.selected {
                p.structured
                    .add_transition(names[state_index].clone(), kind)
                    .expect("catalog");
                p.conditions.push(SearchList {
                    selected: Some(kind),
                    ..SearchList::default()
                });
            }
        }
    });
}
