//! The "Flip next run" and "Flip rooms next run" rows of the F4 window's "Run (debug)" section:
//! Auto, Never or Always, as radio buttons (`docs/variety.md`). The first is for the whole
//! dungeon, the second for the rooms marked flippable. Auto leaves it to `variety.ron` and the
//! run's seed; the other two override it. The choices are for the runs that start after them; a
//! run already going keeps its own. They are never written to `variety.ron` or the profile.
//!
//! The logic is `set_flip_choice` and `flip_label`, which only call `GameState`; the widgets just
//! feed them, like `set_tier`.
use super::RunDebug;
use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::variety::FlipChoice;

/// Sets what the next runs do about the flip. Never fails; it only changes the choice.
pub fn set_flip_choice(game: &mut GameState, choice: FlipChoice) -> Result<(), String> {
    game.set_flip_choice(choice);
    Ok(())
}

/// The read-only line under the radio buttons: what the next run will do, and whether the run in
/// progress (or on the result screen) is flipped.
pub fn flip_label(game: &GameState) -> String {
    let next = match game.flip_choice() {
        FlipChoice::Never => "the next run is not flipped".to_owned(),
        FlipChoice::Always => "the next run is flipped".to_owned(),
        FlipChoice::Auto => format!(
            "the next run is flipped by chance: {}% of seeds (variety.ron)",
            game.variety_rules().flip_percent
        ),
    };
    match (game.run_seed().is_some(), game.dungeon_flipped()) {
        (true, true) => format!("This run is flipped; {next}"),
        (true, false) => format!("This run is not flipped; {next}"),
        (false, _) => {
            let mut text = next;
            text[..1].make_ascii_uppercase();
            text
        }
    }
}

/// Sets what the next runs do about the rooms marked flippable. Never fails.
pub fn set_room_flip_choice(game: &mut GameState, choice: FlipChoice) -> Result<(), String> {
    game.set_room_flip_choice(choice);
    Ok(())
}

/// The read-only line under the "Flip rooms next run" buttons: what the next run does with the
/// rooms marked flippable, and how many of them the run in progress mirrors on their own.
pub fn room_flip_label(game: &GameState) -> String {
    let next = match game.room_flip_choice() {
        FlipChoice::Never => "no room marked flippable is mirrored next run".to_owned(),
        FlipChoice::Always => "every room marked flippable is mirrored next run".to_owned(),
        FlipChoice::Auto => format!(
            "each room marked flippable is mirrored by chance next run: {}% (variety.ron)",
            game.variety_rules().room_flip_percent
        ),
    };
    if game.run_seed().is_some() && game.run_flipped() {
        let count = game.flipped_room_count();
        format!("This run mirrors {count} flippable room(s) on their own; {next}")
    } else {
        let mut text = next;
        text[..1].make_ascii_uppercase();
        text
    }
}

/// Draws one row of Auto, Never and Always radio buttons named `label` and returns the choice
/// after any click. `id` names the buttons for the tests.
pub(super) fn choice_row(
    ui: &mut egui::Ui,
    label: &str,
    id: &str,
    before: FlipChoice,
) -> FlipChoice {
    let mut choice = before;
    ui.horizontal(|ui| {
        ui.add_sized([super::super::LABEL_WIDTH, 20.0], egui::Label::new(label));
        for option in FlipChoice::ALL {
            let radio = ui.radio_value(&mut choice, option, option.label());
            #[cfg(test)]
            ui.data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(format!("{id} {}", option.label())),
                    radio.rect,
                )
            });
            #[cfg(not(test))]
            let _ = (radio, id);
        }
    });
    choice
}

impl RunDebug {
    /// Draws the Flip row. A click on a radio button goes to `set_flip_choice` at once.
    pub(super) fn draw_flip(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        let before = game.flip_choice();
        let choice = choice_row(ui, "Flip next run", "flip", before);
        if choice != before {
            self.last = Some(set_flip_choice(game, choice).map(|()| "Flip choice set"));
        }
        ui.label(flip_label(game));
    }

    /// Draws the Flip rooms row, the same way, for the rooms marked flippable.
    pub(super) fn draw_room_flip(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        let before = game.room_flip_choice();
        let choice = choice_row(ui, "Flip rooms next run", "room flip", before);
        if choice != before {
            self.last = Some(set_room_flip_choice(game, choice).map(|()| "Flip rooms choice set"));
        }
        ui.label(room_flip_label(game));
    }
}

#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod tests;
