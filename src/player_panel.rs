//! The editor's Player panel, and the field row the F4 tuning window shares with it.
//!
//! Presentation only. The values, ranges, validation and file writing live in
//! `hollow_steps::player_editor` and `hollow_steps::tuning`; `draw_field` is the one function
//! that draws a number, for this panel and for the F4 window, so a typed value is judged the same
//! way in both.
use std::cell::RefCell;

mod look;

use crate::editor_panel::widgets::{panel_button, panel_label, probe, section};
use egui_macroquad::egui;
use hollow_steps::editor::EditorSession;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::player_history::{PlayerHistory, PlayerSnapshot};
use hollow_steps::tuning::{Field, FieldSpec};

/// What a window says about the last Save or refused edit.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) enum Status {
    #[default]
    Quiet,
    Saved(String),
    Error(String),
}

/// One number to draw.
pub(crate) struct FieldRow<'a> {
    pub spec: &'a FieldSpec,
    pub value: f32,
    /// Width of the label column, so the drag boxes line up.
    pub label_width: f32,
    /// `Some(differs from the saved value)` adds a Reset button for this field.
    pub dirty: Option<bool>,
    /// Why the value is not acceptable, drawn under the row.
    pub problem: Option<&'a str>,
}

/// What happened to the row this frame.
#[derive(Debug, Default)]
pub(crate) struct FieldEdit {
    /// The new value, if the box was dragged or a valid number was typed.
    pub value: Option<f32>,
    /// Why a typed value was refused (the box keeps its value then).
    pub rejected: Option<String>,
    /// The Reset button was clicked.
    pub reset: bool,
}

/// A labelled drag box with the field's unit and clamped range.
pub(crate) fn draw_field(ui: &mut egui::Ui, row: FieldRow<'_>) -> FieldEdit {
    let FieldRow {
        spec,
        mut value,
        label_width,
        dirty,
        problem,
    } = row;
    let rejected = RefCell::new(None);
    let mut changed = false;
    let mut reset = false;
    ui.horizontal(|ui| {
        ui.add_sized([label_width, 20.0], egui::Label::new(spec.label));
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
        probe(ui, || (egui::Id::new(("field", spec.key)), response.rect));
        changed = response.changed();
        if let Some(dirty) = dirty {
            let button = ui
                .add_enabled(dirty, egui::Button::new("Reset").small())
                .on_hover_text("Back to the saved value")
                .on_disabled_hover_text("Same as the saved value");
            probe(ui, || (egui::Id::new(("reset", spec.key)), button.rect));
            reset = button.clicked();
        }
    });
    if let Some(problem) = problem {
        ui.add(
            egui::Label::new(egui::RichText::new(problem).color(egui::Color32::LIGHT_RED)).wrap(),
        );
    }
    FieldEdit {
        value: changed.then_some(value),
        rejected: rejected.into_inner(),
        reset,
    }
}

/// Reads typed text for one field. Anything outside the field's range, or not a number, is
/// refused: the drag box keeps its value and `rejected` gets the reason.
pub(crate) fn parse_entry(
    spec: &FieldSpec,
    text: &str,
    rejected: &RefCell<Option<String>>,
) -> Option<f64> {
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

/// The panel's sections, in the order they are drawn. Every `Field` is in exactly one (a test
/// checks), so a new number has to be given a place. The file has no player health, so
/// "Health" and "Knockback" are the enemy defaults it holds.
pub(crate) const SECTIONS: [(&str, &[Field]); 7] = [
    ("Movement", &[Field::MoveSpeed]),
    (
        "Jump",
        &[
            Field::JumpSpeed,
            Field::Gravity,
            Field::MaxFallSpeed,
            Field::HeldJumpGravityMultiplier,
            Field::CoyoteSeconds,
            Field::JumpBufferSeconds,
        ],
    ),
    (
        "Dash",
        &[Field::DashSpeed, Field::DashDuration, Field::DashCooldown],
    ),
    (
        "Attack",
        &[
            Field::StrikeForwardWidth,
            Field::StrikeForwardHeight,
            Field::StrikeUpWidth,
            Field::StrikeUpHeight,
            Field::StrikeDownWidth,
            Field::StrikeDownHeight,
            Field::PogoSpeed,
        ],
    ),
    ("Health (enemy default)", &[Field::EnemyDefaultHealth]),
    (
        "Knockback (enemies)",
        &[Field::EnemyKnockbackSpeed, Field::EnemyKnockbackSeconds],
    ),
    ("Size", &[Field::Width, Field::Height]),
];

/// Label column of the panel's rows; the editor's side panel is narrower than the F4 window.
const LABEL_WIDTH: f32 = 150.0;

/// What the Player panel remembers between frames.
#[derive(Default)]
pub(crate) struct PlayerUi {
    /// The Player panel is shown instead of the room tools.
    pub(crate) open: bool,
    /// Save was clicked this frame; `draw_editor` saves through the game and records the result.
    pub(crate) save_requested: bool,
    pub(crate) status: Status,
    /// Why a typed value was refused, drawn under its field until the next edit.
    refused: Option<(Field, String)>,
    /// The Look section's state, including what the canvas previews.
    pub(crate) look: look::LookUi,
    /// Undo and redo for what this panel edits, apart from the room's.
    history: PlayerHistory,
    /// The last mouse press was inside this panel: Ctrl+Z and Ctrl+Y are for its history.
    focused: bool,
    /// An Undo or Redo button was clicked this frame; applied after the frame is observed.
    history_request: Option<HistoryKey>,
}

impl PlayerUi {
    /// Ctrl+Z / Ctrl+Y: goes through the Player panel's own history when the panel is open and
    /// has focus, and returns true (the key is taken even if there is nothing to undo). False
    /// means the keys are the room's.
    pub(crate) fn apply_history(&mut self, editor: &mut EditorSession, key: HistoryKey) -> bool {
        if !(self.open && self.focused) {
            return false;
        }
        self.step_history(editor, key);
        true
    }

    fn step_history(&mut self, editor: &mut EditorSession, key: HistoryKey) {
        match key {
            HistoryKey::Undo => self.history.undo(editor),
            HistoryKey::Redo => self.history.redo(editor),
        };
        self.refused = None;
    }

    /// What the editor canvas draws of the player: only while the panel is open and its Show
    /// player on canvas box is ticked.
    pub(crate) fn canvas_preview(
        &self,
    ) -> Option<hollow_steps::player_look_editor::PreviewSettings> {
        (self.open && self.look.show_on_canvas).then_some(self.look.preview)
    }
}

/// Draws the Player button and, while it is open, the panel. Returns true when the panel is
/// shown, which means the caller draws nothing else in the editor's side panel.
pub(crate) fn draw(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut PlayerUi) -> bool {
    if panel_button(ui, if state.open { "Close player" } else { "Player" }) {
        state.open = !state.open;
    }
    if !state.open {
        state.focused = false;
        return false;
    }
    let before = PlayerSnapshot::capture(editor);
    draw_stats(ui, editor, state);
    // A press inside the panel gives it the undo keys; a press anywhere else takes them away.
    let (down, pressed_at) = ui.input(|input| {
        (
            input.pointer.any_down(),
            input
                .pointer
                .any_pressed()
                .then(|| input.pointer.interact_pos())
                .flatten(),
        )
    });
    if let Some(at) = pressed_at {
        state.focused = ui.clip_rect().contains(at);
    }
    state
        .history
        .observe(before, &PlayerSnapshot::capture(editor), down);
    if let Some(key) = state.history_request.take() {
        state.step_history(editor, key);
    }
    true
}

fn draw_stats(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut PlayerUi) {
    ui.heading("Player");
    panel_label(
        ui,
        if editor.player.is_dirty() {
            "Unsaved changes."
        } else {
            "Matches the saved values."
        },
    );
    let problems = editor.player.problems(&editor.spawn_checks());
    for (name, fields) in SECTIONS {
        section(ui, name, |ui| {
            for &field in fields {
                let refused = state
                    .refused
                    .as_ref()
                    .filter(|(refused, _)| *refused == field)
                    .map(|(_, message)| message.as_str());
                let problem = refused.or_else(|| {
                    problems
                        .iter()
                        .find(|problem| problem.fields.contains(&field))
                        .map(|problem| problem.message.as_str())
                });
                let edit = draw_field(
                    ui,
                    FieldRow {
                        spec: field.spec(),
                        value: field.get(editor.player.edited()),
                        label_width: LABEL_WIDTH,
                        dirty: Some(editor.player.is_field_dirty(field)),
                        problem,
                    },
                );
                if let Some(value) = edit.value {
                    state.refused = None;
                    state.status = match editor.player.set_field(field, value) {
                        Ok(()) => Status::Quiet,
                        Err(error) => Status::Error(error),
                    };
                }
                if let Some(message) = edit.rejected {
                    state.refused = Some((field, message));
                }
                if edit.reset {
                    editor.player.reset_field(field);
                    state.refused = None;
                }
            }
        });
    }
    look::draw(ui, editor, &mut state.look);
    ui.separator();
    if panel_button(ui, "Save player") {
        state.refused = None;
        state.save_requested = true;
    }
    if panel_button(ui, "Undo player edit") {
        state.history_request = Some(HistoryKey::Undo);
    }
    if panel_button(ui, "Redo player edit") {
        state.history_request = Some(HistoryKey::Redo);
    }
    if panel_button(ui, "Reset all") {
        editor.player.reset_all();
        state.refused = None;
        state.status = Status::Quiet;
    }
    match &state.status {
        Status::Quiet => {}
        Status::Saved(path) => panel_label(ui, &format!("Saved to {path}")),
        Status::Error(message) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(message.as_str()).color(egui::Color32::LIGHT_RED),
                )
                .wrap(),
            );
        }
    }
    ui.small("Stat changes apply in play at once. Save writes player.ron in the assets folder.");
}

#[cfg(test)]
mod tests;
