//! The F4 player-tuning window: a drag box for every number in `player.ron`, then one for every
//! number in `vitals.ron`.
//!
//! Presentation only. The ranges, validation and file writing live in `hollow_steps::tuning`,
//! `hollow_steps::vitals` and `GameState`; this module turns widget changes into
//! `GameState::set_tuning` / `set_vitals_config` calls, so a change is in effect on the next
//! tick, and shows why a typed value was refused.
use std::cell::RefCell;

use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::tuning::{Field, FieldSpec};
use hollow_steps::vitals::VitalsField;

/// What the footer says about the last Save or refused edit.
#[derive(Clone, Debug, Default, PartialEq)]
enum Status {
    #[default]
    Quiet,
    Saved(String),
    Error(String),
}

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
                        let mut group = "";
                        for field in Field::ALL {
                            if field.spec().group != group {
                                group = field.spec().group;
                                ui.add_space(4.0);
                                ui.strong(group);
                            }
                            field_row(ui, game, status, field);
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

/// One labelled drag box. Returns the new value if it was changed, and why a typed value was
/// refused (the box keeps its value then).
fn drag_row(ui: &mut egui::Ui, spec: &FieldSpec, mut value: f32) -> (Option<f32>, Option<String>) {
    let rejected = RefCell::new(None);
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(spec.label));
        let mut drag = egui::DragValue::new(&mut value)
            .range(spec.min..=spec.max)
            .speed(spec.step)
            .fixed_decimals(spec.decimals)
            // Judge a typed number when it is entered, not on every key press.
            .update_while_editing(false)
            .custom_parser(|text| parse_entry(spec, text, &rejected));
        if !spec.unit.is_empty() {
            drag = drag.suffix(format!(" {}", spec.unit));
        }
        let response = ui.add(drag).on_hover_text(format!(
            "{}: {} to {}{}",
            spec.key,
            spec.min,
            spec.max,
            if spec.unit.is_empty() {
                String::new()
            } else {
                format!(" {}", spec.unit)
            }
        ));
        #[cfg(test)]
        ui.data_mut(|data| data.insert_temp(egui::Id::new(("field", spec.key)), response.rect));
        changed = response.changed();
    });
    (changed.then_some(value), rejected.into_inner())
}

fn field_row(ui: &mut egui::Ui, game: &mut GameState, status: &mut Status, field: Field) {
    let (edited, rejected) = drag_row(ui, field.spec(), field.get(game.tuning()));
    if let Some(value) = edited {
        let applied = game
            .tuning()
            .with_field(field, value)
            .and_then(|changed| game.set_tuning(changed));
        *status = match applied {
            Ok(()) => Status::Quiet,
            Err(error) => Status::Error(error),
        };
    }
    if let Some(message) = rejected {
        *status = Status::Error(message);
    }
}

fn vitals_row(ui: &mut egui::Ui, game: &mut GameState, status: &mut Status, field: VitalsField) {
    let (edited, rejected) = drag_row(ui, field.spec(), field.get(game.vitals_config()));
    if let Some(value) = edited {
        let applied = game
            .vitals_config()
            .with_field(field, value)
            .and_then(|changed| game.set_vitals_config(changed));
        *status = match applied {
            Ok(()) => Status::Quiet,
            Err(error) => Status::Error(error),
        };
    }
    if let Some(message) = rejected {
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

/// Reads typed text for one field. Anything outside the field's range, or not a number, is
/// refused: the drag box keeps its value and `rejected` gets the reason for the footer.
fn parse_entry(spec: &FieldSpec, text: &str, rejected: &RefCell<Option<String>>) -> Option<f64> {
    let text = text.trim();
    let number = if spec.unit.is_empty() {
        text
    } else {
        text.strip_suffix(spec.unit).unwrap_or(text).trim()
    };
    let result = number
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| format!("{}: \"{text}\" is not a number", spec.key))
        .and_then(|value| spec.check(value as f32))
        .map(f64::from);
    match result {
        Ok(value) => Some(value),
        Err(message) => {
            *rejected.borrow_mut() = Some(message);
            None
        }
    }
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
