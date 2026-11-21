//! The editor's Upgrades panel.
//!
//! Presentation only. The values, ranges, validation and file writing live in
//! `hollow_steps::upgrade_editor`; a number is drawn by the Player panel's `draw_field`, so a typed
//! value is judged the same way here as there. Every field of an upgrade is an
//! `upgrade_editor::FIELDS` entry, and the panel draws them in that order, each as its `Control`
//! says.
use egui_macroquad::egui;
use hollow_steps::charm_editor::{amount, EffectKind};
use hollow_steps::editor::EditorSession;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::game::GameState;
use hollow_steps::snapshot_history::SnapshotHistory;
use hollow_steps::tuning::FieldSpec;
use hollow_steps::upgrade_editor::{
    amount_spec, Control, UpgradeDraft, UpgradeField, COSTS, FIELDS,
};
use hollow_steps::upgrades::Upgrade;

use crate::editor_panel::widgets::{panel_button, panel_label, probe};
use crate::player_panel::{draw_field, FieldEdit, FieldRow, Status};

/// Label column of the panel's rows; the editor's side panel is narrow.
const LABEL_WIDTH: f32 = 110.0;

/// What the Upgrades panel remembers between frames.
#[derive(Default)]
pub(crate) struct UpgradeUi {
    /// The Upgrades panel is shown instead of the room tools.
    pub(crate) open: bool,
    /// Save was clicked this frame; `finish_frame` saves through the game and records the result.
    pub(crate) save_requested: bool,
    pub(crate) status: Status,
    /// Why a typed number or a level change was refused: the upgrade's id, the field (a `COSTS`
    /// key for a price) and the reason, drawn under the field until the next edit.
    refused: Option<(String, &'static str, String)>,
    /// Undo and redo for what this panel edits, apart from the room's.
    history: SnapshotHistory<Upgrade>,
    /// The last mouse press was inside this panel: Ctrl+Z and Ctrl+Y are for its history.
    focused: bool,
    /// An Undo or Redo button was clicked this frame; applied after the frame is observed.
    history_request: Option<HistoryKey>,
}

impl UpgradeUi {
    /// Ctrl+Z / Ctrl+Y: goes through the Upgrades panel's own history when the panel is open and
    /// has focus, and returns true (the key is taken even if there is nothing to undo). False
    /// means the keys are someone else's.
    pub(crate) fn apply_history(&mut self, editor: &mut EditorSession, key: HistoryKey) -> bool {
        if !(self.open && self.focused) {
            return false;
        }
        self.step_history(editor, key);
        true
    }

    fn step_history(&mut self, editor: &mut EditorSession, key: HistoryKey) {
        match key {
            HistoryKey::Undo => self.history.undo(&mut editor.upgrades),
            HistoryKey::Redo => self.history.redo(&mut editor.upgrades),
        };
        self.refused = None;
    }

    /// The end of an editor frame: the game takes the edited list into effect, and a Save asked
    /// for this frame goes through the game.
    pub(crate) fn finish_frame(&mut self, game: &mut GameState) {
        game.sync_upgrades_from_editor();
        if std::mem::take(&mut self.save_requested) {
            self.status = match game.save_upgrades() {
                Ok(path) => Status::Saved(path.display().to_string()),
                Err(error) => Status::Error(error),
            };
        }
    }
}

/// Something an upgrade's own buttons asked for this frame.
enum Request {
    Reset(usize),
    Delete(usize),
}

/// Draws the Upgrades button and, while it is open, the panel. Returns true when the panel is
/// shown, which means the caller draws nothing else in the editor's side panel. `charms_open` and
/// `player_open` are the other two panels' flags: only one of the three is open at a time, and
/// the one opened last wins.
pub(crate) fn draw(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
    charms_open: &mut bool,
    player_open: &mut bool,
) -> bool {
    if *charms_open || *player_open {
        state.open = false;
    }
    if panel_button(
        ui,
        if state.open {
            "Close upgrades"
        } else {
            "Upgrades"
        },
    ) {
        state.open = !state.open;
        if state.open {
            *charms_open = false;
            *player_open = false;
        }
    }
    if !state.open {
        state.focused = false;
        return false;
    }
    let before = editor.upgrades.edited().to_vec();
    draw_list(ui, editor, state);
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
    // A stretch of typing in a text box is one step, like a drag.
    let typing = ui.ctx().wants_keyboard_input();
    state
        .history
        .observe(before, editor.upgrades.edited(), down || typing);
    if let Some(key) = state.history_request.take() {
        state.step_history(editor, key);
    }
    true
}

fn draw_list(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut UpgradeUi) {
    ui.heading("Upgrades");
    panel_label(
        ui,
        if editor.upgrades.is_dirty() {
            "Unsaved changes."
        } else {
            "Matches the saved upgrades."
        },
    );
    panel_label(ui, &editor.upgrades.totals().text());
    if panel_button(ui, "New upgrade") {
        editor.upgrades.new_upgrade();
        state.refused = None;
    }
    let mut request = None;
    for index in 0..editor.upgrades.edited().len() {
        draw_upgrade(ui, editor, state, index, &mut request);
    }
    match request {
        Some(Request::Reset(index)) => editor.upgrades.reset_upgrade(index),
        Some(Request::Delete(index)) => {
            let _ = editor.upgrades.delete(index);
        }
        None => {}
    }
    if request.is_some() {
        state.refused = None;
    }
    ui.separator();
    if panel_button(ui, "Save upgrades") {
        state.refused = None;
        state.save_requested = true;
    }
    if panel_button(ui, "Undo upgrade edit") {
        state.history_request = Some(HistoryKey::Undo);
    }
    if panel_button(ui, "Redo upgrade edit") {
        state.history_request = Some(HistoryKey::Redo);
    }
    if panel_button(ui, "Reset all") {
        editor.upgrades.reset_all();
        state.refused = None;
        state.status = Status::Quiet;
    }
    match &state.status {
        Status::Quiet => {}
        Status::Saved(text) => panel_label(ui, &format!("Saved to {text}")),
        Status::Error(message) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(message.as_str()).color(egui::Color32::LIGHT_RED),
                )
                .wrap(),
            );
        }
    }
    ui.small(
        "Changes apply in play at once. Save writes upgrades.ron in the assets folder. Deleting \
         an upgrade keeps the levels players bought in profile.ron; they get no effect.",
    );
}

fn draw_upgrade(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
    index: usize,
    request: &mut Option<Request>,
) {
    let upgrade = editor.upgrades.edited()[index].clone();
    let title = if upgrade.name.trim().is_empty() {
        upgrade.id.clone()
    } else {
        upgrade.name.clone()
    };
    let dirty = editor.upgrades.is_upgrade_dirty(index);
    // The id, not the name, keys the section: renaming an upgrade must not rebuild its widgets.
    let section = egui::CollapsingHeader::new(format!("{title}{}", if dirty { " *" } else { "" }))
        .id_salt(("upgrade", &upgrade.id))
        .default_open(true);
    section.show(ui, |ui| {
        for field in FIELDS {
            let problem = editor.upgrades.problem_for(index, field.key);
            draw_field_row(ui, editor, state, index, field, problem);
        }
        ui.horizontal(|ui| {
            let reset = ui
                .add_enabled(dirty, egui::Button::new("Reset upgrade"))
                .on_hover_text("Back to the saved upgrade (a new upgrade is removed)")
                .on_disabled_hover_text("Same as the saved upgrade");
            probe(ui, || {
                (egui::Id::new(("upgrade reset", &upgrade.id)), reset.rect)
            });
            if reset.clicked() {
                *request = Some(Request::Reset(index));
            }
            let delete = ui.button("Delete upgrade").on_hover_text(
                "Removed from the list; players who bought it keep the levels but get no effect",
            );
            probe(ui, || {
                (egui::Id::new(("upgrade delete", &upgrade.id)), delete.rect)
            });
            if delete.clicked() {
                *request = Some(Request::Delete(index));
            }
        });
    });
}

/// One field of the upgrade, drawn as its `Control` says, with `problem` (if any) under it.
fn draw_field_row(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
    index: usize,
    field: UpgradeField,
    problem: Option<String>,
) {
    let upgrade = editor.upgrades.edited()[index].clone();
    let refused = state
        .refused
        .as_ref()
        .filter(|(id, key, _)| *id == upgrade.id && key.starts_with(field.key))
        .map(|(_, _, message)| message.clone());
    let shown = refused.or(problem);
    match field.control {
        Control::ReadOnly => {
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                ui.label(&upgrade.id)
                    .on_hover_text("A stable name; the profile remembers levels by it");
            });
        }
        Control::Text(max_chars) => {
            let mut text = if field.key == "name" {
                upgrade.name.clone()
            } else {
                upgrade.description.clone()
            };
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .id_salt(("upgrade text", &upgrade.id, field.key))
                        .char_limit(max_chars)
                        .desired_width(f32::INFINITY),
                );
                probe(ui, || {
                    (
                        egui::Id::new(("upgrade text", &upgrade.id, field.key)),
                        response.rect,
                    )
                });
                if response.changed() {
                    let _ = if field.key == "name" {
                        editor.upgrades.set_name(index, &text)
                    } else {
                        editor.upgrades.set_description(index, &text)
                    };
                }
            });
        }
        Control::Effect => {
            let kind = EffectKind::of(&upgrade.effect);
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                let combo = egui::ComboBox::from_id_salt(("upgrade effect", &upgrade.id))
                    .selected_text(kind.label())
                    .show_ui(ui, |ui| {
                        for option in EffectKind::ALL {
                            if ui
                                .selectable_label(option == kind, option.label())
                                .clicked()
                            {
                                let _ = editor.upgrades.set_effect_kind(index, option);
                            }
                        }
                    });
                probe(ui, || {
                    (
                        egui::Id::new(("upgrade effect", &upgrade.id)),
                        combo.response.rect,
                    )
                });
            });
            let spec = amount_spec(kind);
            let edit = number_row(ui, &spec, amount(&upgrade.effect), shown.as_deref());
            apply_number(
                editor,
                state,
                &upgrade.id,
                spec.key,
                edit,
                |draft, value| draft.set_effect_amount(index, value),
            );
        }
        Control::Costs => draw_costs(ui, editor, state, index, shown.clone()),
    }
    if matches!(field.control, Control::ReadOnly | Control::Text(_)) {
        if let Some(message) = shown {
            ui.add(
                egui::Label::new(egui::RichText::new(message).color(egui::Color32::LIGHT_RED))
                    .wrap(),
            );
        }
    }
}

/// One price drag box per level, then Add level and Remove level. `shown` is the message for the
/// prices as a whole (a refused number or a problem with them).
fn draw_costs(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
    index: usize,
    shown: Option<String>,
) {
    let upgrade = editor.upgrades.edited()[index].clone();
    ui.label(FIELDS[4].label);
    for (level, price) in upgrade.costs.iter().enumerate() {
        // A list that is longer than the table allows has no range for the extra levels: the
        // problem under the rows says so, and Remove level takes them off.
        let Some(spec) = COSTS.get(level) else { break };
        let edit = number_row(ui, spec, *price as f32, None);
        apply_number(
            editor,
            state,
            &upgrade.id,
            spec.key,
            edit,
            |draft, value| draft.set_cost(index, level, value),
        );
    }
    ui.horizontal(|ui| {
        let add = ui
            .add_enabled(
                upgrade.costs.len() < COSTS.len(),
                egui::Button::new("Add level"),
            )
            .on_hover_text("A new last level, priced like the one before it")
            .on_disabled_hover_text("An upgrade has at most 10 levels");
        probe(ui, || {
            (egui::Id::new(("upgrade add level", &upgrade.id)), add.rect)
        });
        if add.clicked() {
            state.refused = editor
                .upgrades
                .add_level(index)
                .err()
                .map(|message| (upgrade.id.clone(), "costs", message));
        }
        let remove = ui
            .add_enabled(upgrade.costs.len() > 1, egui::Button::new("Remove level"))
            .on_hover_text("Takes off the last level")
            .on_disabled_hover_text("An upgrade needs at least one level");
        probe(ui, || {
            (
                egui::Id::new(("upgrade remove level", &upgrade.id)),
                remove.rect,
            )
        });
        if remove.clicked() {
            state.refused = editor
                .upgrades
                .remove_level(index)
                .err()
                .map(|message| (upgrade.id.clone(), "costs", message));
        }
    });
    if let Some(message) = shown {
        ui.add(
            egui::Label::new(egui::RichText::new(message).color(egui::Color32::LIGHT_RED)).wrap(),
        );
    }
}

/// A drag box with the field's unit and range (the Player panel's row).
fn number_row(ui: &mut egui::Ui, spec: &FieldSpec, value: f32, problem: Option<&str>) -> FieldEdit {
    draw_field(
        ui,
        FieldRow {
            spec,
            value,
            label_width: LABEL_WIDTH,
            dirty: None,
            problem,
        },
    )
}

/// Puts a dragged or typed number into the draft, or remembers why it was refused.
fn apply_number(
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
    id: &str,
    key: &'static str,
    edit: FieldEdit,
    set: impl FnOnce(&mut UpgradeDraft, f32) -> Result<(), String>,
) {
    if let Some(value) = edit.value {
        state.refused = set(&mut editor.upgrades, value)
            .err()
            .map(|message| (id.to_owned(), key, message));
    }
    if let Some(message) = edit.rejected {
        state.refused = Some((id.to_owned(), key, message));
    }
}

#[cfg(test)]
mod tests;
