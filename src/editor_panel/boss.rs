use super::visual_size_field::{hitbox_side, visual_size_rows};
use super::widgets::{
    choice, panel_button, panel_combo, panel_input, panel_label, row_controls, searchable_list,
    section,
};
use super::EditorSession;
use crate::SearchList;
use egui_macroquad::egui;
use hollow_steps::behavior::Action;
use hollow_steps::boss_definition::AttackSelection;
use hollow_steps::boss_editor::BossInputs;
use std::path::PathBuf;

pub(crate) struct BossPropertyUi {
    pub path: PathBuf,
    pub inputs: BossInputs,
    pub attack: Option<usize>,
    attack_name: String,
    new_attack: Option<String>,
    add_action: SearchList,
    pub error: Option<String>,
    operation_error: Option<String>,
}

impl BossPropertyUi {
    pub fn new(editor: &EditorSession, path: PathBuf) -> Self {
        Self {
            inputs: BossInputs::from_definition(&editor.content.bosses[&path]),
            path,
            attack: None,
            attack_name: String::new(),
            new_attack: None,
            add_action: SearchList::default(),
            error: None,
            operation_error: None,
        }
    }

    fn select_attack(&mut self, index: usize) {
        self.attack = Some(index);
        self.attack_name = self.inputs.attacks[index].name.clone();
    }

    pub fn sync(&mut self, editor: &mut EditorSession) {
        self.error = self
            .inputs
            .build()
            .and_then(|boss| editor.update_boss(&self.path, boss))
            .err();
    }
}

pub(crate) fn draw_boss_panel(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut BossPropertyUi,
) {
    section(ui, "Boss properties", |ui| {
        panel_label(ui, "Shared boss definition (all rooms)");
        for (i, label) in [
            "Name",
            "Health (1..4294967295)",
            "Hitbox width (px)",
            "Hitbox height (px)",
            "Contact damage (0..65535)",
            "Pause between attacks (seconds)",
        ]
        .iter()
        .enumerate()
        {
            panel_input(
                ui,
                egui::Id::new(("boss field", &p.path, i)),
                label,
                &mut p.inputs.fields[i],
            );
        }
        visual_size_rows(
            ui,
            &mut p.inputs.visual_size,
            (
                hitbox_side(&p.inputs.fields[2]),
                hitbox_side(&p.inputs.fields[3]),
            ),
        );
        let mut mode = usize::from(p.inputs.selection == AttackSelection::Random);
        panel_combo(
            ui,
            egui::Id::new("boss selection"),
            "Attack selection mode",
            &["Cycle", "Random"],
            &mut mode,
        );
        p.inputs.selection = if mode == 0 {
            AttackSelection::Cycle
        } else {
            AttackSelection::Random
        };
    });
    section(ui, "Boss attacks", |ui| {
        let mut selected = None;
        let mut command = None;
        for (i, attack) in p.inputs.attacks.iter().enumerate() {
            ui.push_id(("boss attack", i), |ui| {
                if choice(
                    ui,
                    &format!("{}. {}", i + 1, attack.name),
                    p.attack == Some(i),
                )
                .clicked()
                {
                    selected = Some(i);
                }
                if let Some(op) = row_controls(ui) {
                    command = Some((i, op));
                }
            });
        }
        if let Some(i) = selected {
            p.select_attack(i);
        }
        if let Some((i, op)) = command {
            let result = if op == 2 {
                p.inputs.remove_attack(i)
            } else {
                p.inputs.move_attack(i, op == 0)
            };
            match result {
                Ok(()) => {
                    if op == 2 {
                        p.attack = p.attack.and_then(|s| {
                            if s == i {
                                None
                            } else {
                                Some(s - usize::from(s > i))
                            }
                        });
                    } else {
                        let target = if op == 0 { i - 1 } else { i + 1 };
                        p.attack = p.attack.map(|s| {
                            if s == i {
                                target
                            } else if s == target {
                                i
                            } else {
                                s
                            }
                        });
                    }
                    p.operation_error = None;
                }
                Err(error) => p.operation_error = Some(error),
            }
        }
        if panel_button(ui, "Add attack") {
            p.new_attack = Some(String::new());
        }
        if let Some(name) = &mut p.new_attack {
            panel_input(
                ui,
                egui::Id::new("new boss attack"),
                "New attack name",
                name,
            );
            if panel_button(ui, "Create attack") {
                match p.inputs.add_attack(name) {
                    Ok(i) => {
                        p.select_attack(i);
                        p.new_attack = None;
                        p.operation_error = None;
                    }
                    Err(error) => p.operation_error = Some(error),
                }
            }
            if panel_button(ui, "Cancel attack") {
                p.new_attack = None;
            }
        }
    });
    if let Some(index) = p.attack.filter(|i| *i < p.inputs.attacks.len()) {
        section(ui, "Attack editor", |ui| {
            panel_input(
                ui,
                egui::Id::new(("rename attack", index)),
                "Attack name",
                &mut p.attack_name,
            );
            if panel_button(ui, "Rename attack") {
                p.operation_error = p.inputs.rename_attack(index, &p.attack_name).err();
            }
            panel_label(ui, "Actions (in order)");
            let mut command = None;
            for (i, row) in p.inputs.attacks[index].actions.iter_mut().enumerate() {
                ui.push_id(("boss action", index, i), |ui| {
                    ui.separator();
                    panel_label(ui, &format!("{}. {}", i + 1, Action::ALL_TYPES[row.kind]));
                    let template = Action::all_defaults(row.kind).expect("catalog action");
                    for (j, label) in template.parameter_labels().iter().enumerate() {
                        panel_input(
                            ui,
                            egui::Id::new(("boss action value", &p.path, index, i, j)),
                            label,
                            &mut row.values[j],
                        );
                        if let Err(error) = template.clone().set_parameter(j, &row.values[j]) {
                            ui.colored_label(egui::Color32::LIGHT_RED, error);
                        }
                    }
                    if let Some(op) = row_controls(ui) {
                        command = Some((i, op));
                    }
                });
            }
            if let Some((i, op)) = command {
                p.operation_error = if op == 2 {
                    p.inputs.remove_action(index, i)
                } else {
                    p.inputs.move_action(index, i, op == 0)
                }
                .err();
            }
            let catalog = hollow_steps::boss_editor::BossActionInput::catalog();
            let names = catalog.iter().map(|(_, name)| *name).collect::<Vec<_>>();
            searchable_list(
                ui,
                egui::Id::new("boss action catalog"),
                "Action type",
                &names,
                &mut p.add_action,
            );
            if panel_button(ui, "Add action") {
                if let Some((kind, _)) = p.add_action.selected.and_then(|i| catalog.get(i)) {
                    p.operation_error = p.inputs.add_action(index, *kind).err();
                }
            }
        });
    }
    // Always validate every buffer, including collapsed sections and unselected attacks.
    p.sync(editor);
    if let Some(error) = &p.error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
    if let Some(error) = &p.operation_error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}
