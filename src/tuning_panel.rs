//! The F4 player-tuning window: a drag box for every number in `player.ron`, then one for every
//! number in `vitals.ron`.
//!
//! Presentation only. The ranges, validation and file writing live in `hollow_steps::tuning`,
//! `hollow_steps::vitals` and `GameState`; this module turns widget changes into
//! `GameState::set_tuning` / `set_vitals_config` calls, so a change is in effect on the next
//! tick, and shows why a typed value was refused.
use crate::player_panel::{draw_field, FieldRow, Status};
use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::player_editor::SpawnCheck;
use hollow_steps::tuning::Field;
use hollow_steps::vitals::VitalsField;

// The field row and its text parser moved to `player_panel`; the tests below still call them.
#[cfg(test)]
use crate::player_panel::parse_entry;
#[cfg(test)]
use std::cell::RefCell;

/// State of the F4 window; the values themselves live in `GameState`.
#[derive(Debug, Default)]
pub struct TuningPanel {
    open: bool,
    status: Status,
}

/// Pixels of label column, so the drag boxes line up.
const LABEL_WIDTH: f32 = 190.0;

impl TuningPanel {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Draws the window while it is open and applies any edit or Save to `game`.
    pub fn draw(&mut self, ctx: &egui::Context, game: &mut GameState) {
        if !self.open {
            return;
        }
        let Self { open, status } = self;
        let list_height = (ctx.screen_rect().height() - 190.0).max(120.0);
        egui::Window::new("Player tuning (F4)")
            .open(open)
            .default_pos([16.0, 16.0])
            .resizable(false)
            .show(ctx, |ui| {
                ui.spacing_mut().interact_size.y = 20.0;
                egui::ScrollArea::vertical()
                    .id_salt("tuning fields")
                    .max_height(list_height)
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        let spawns = game.spawn_checks();
                        let mut group = "";
                        for field in Field::ALL {
                            if field.spec().group != group {
                                group = field.spec().group;
                                ui.add_space(4.0);
                                ui.strong(group);
                            }
                            field_row(ui, game, status, field, &spawns);
                        }
                        group = "";
                        for field in VitalsField::ALL {
                            if field.spec().group != group {
                                group = field.spec().group;
                                ui.add_space(4.0);
                                ui.strong(format!("Vitals: {group}"));
                            }
                            vitals_row(ui, game, status, field);
                        }
                    });
                ui.separator();
                footer(ui, game, status);
            });
    }
}

/// A player number, drawn by the row the editor's Player panel uses too. A value that breaks a
/// rule across fields or against a room's spawn is applied (and shown under the row); Save refuses it.
fn field_row(
    ui: &mut egui::Ui,
    game: &mut GameState,
    status: &mut Status,
    field: Field,
    spawns: &[SpawnCheck],
) {
    let problem = game.player_draft().problem_for(field, spawns);
    let edit = draw_field(
        ui,
        FieldRow {
            spec: field.spec(),
            value: field.get(game.tuning()),
            label_width: LABEL_WIDTH,
            dirty: None,
            problem: problem.as_deref(),
        },
    );
    if let Some(value) = edit.value {
        *status = match game.set_tuning_field(field, value) {
            Ok(()) => Status::Quiet,
            Err(error) => Status::Error(error),
        };
    }
    if let Some(message) = edit.rejected {
        *status = Status::Error(message);
    }
}

fn vitals_row(ui: &mut egui::Ui, game: &mut GameState, status: &mut Status, field: VitalsField) {
    let edit = draw_field(
        ui,
        FieldRow {
            spec: field.spec(),
            value: field.get(game.vitals_config()),
            label_width: LABEL_WIDTH,
            dirty: None,
            problem: None,
        },
    );
    if let Some(value) = edit.value {
        let applied = game
            .vitals_config()
            .with_field(field, value)
            .and_then(|changed| game.set_vitals_config(changed));
        *status = match applied {
            Ok(()) => Status::Quiet,
            Err(error) => Status::Error(error),
        };
    }
    if let Some(message) = edit.rejected {
        *status = Status::Error(message);
    }
}

/// Saves `player.ron`, and `vitals.ron` too when its values differ from the saved ones.
fn save_all(game: &mut GameState) -> Status {
    let mut saved = Vec::new();
    match game.save_tuning() {
        Ok(path) => saved.push(path.display().to_string()),
        Err(error) => return Status::Error(error),
    }
    if game.vitals_unsaved() {
        match game.save_vitals() {
            Ok(path) => saved.push(path.display().to_string()),
            Err(error) => return Status::Error(error),
        }
    }
    Status::Saved(saved.join(" and "))
}

fn footer(ui: &mut egui::Ui, game: &mut GameState, status: &mut Status) {
    let save = ui.button("Save");
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(egui::Id::new("save button"), save.rect));
    if save.clicked() {
        *status = save_all(game);
    }
    ui.label(if game.tuning_unsaved() || game.vitals_unsaved() {
        "Unsaved changes. F5 reloads the saved values."
    } else {
        "Matches the saved values."
    });
    match status {
        Status::Quiet => {}
        Status::Saved(path) => {
            ui.label(format!("Saved to {path}"));
        }
        Status::Error(message) => {
            ui.colored_label(egui::Color32::LIGHT_RED, message.as_str());
        }
    }
    ui.small(
        "Changes apply at once. Save writes player.ron, and vitals.ron if its values changed, \
         in the assets folder.",
    );
}

/// egui gets the wheel delta at 28 points per step, the same as the room editor, since the
/// integration's own scaling is too small to scroll the field list.
pub fn scroll_from_wheel(ctx: &egui::Context, wheel: (f32, f32)) {
    let delta = egui::vec2(wheel.0.clamp(-3.0, 3.0), wheel.1.clamp(-3.0, 3.0)) * 28.0;
    ctx.input_mut(|input| {
        input.raw_scroll_delta = delta;
        input.smooth_scroll_delta = delta;
    });
}

#[cfg(test)]
mod tests;
