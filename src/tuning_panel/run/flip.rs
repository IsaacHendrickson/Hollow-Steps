//! The "Flip next run" row of the F4 window's "Run (debug)" section: Auto, Never or Always, as
//! radio buttons (`docs/variety.md`). Auto leaves it to `variety.ron` and the run's seed; the
//! other two override it. The choice is for the runs that start after it; a run already going
//! keeps its own. It is never written to `variety.ron` or the profile.
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
    match (game.run_seed().is_some(), game.run_flipped()) {
        (true, true) => format!("This run is flipped; {next}"),
        (true, false) => format!("This run is not flipped; {next}"),
        (false, _) => {
            let mut text = next;
            text[..1].make_ascii_uppercase();
            text
        }
    }
}

impl RunDebug {
    /// Draws the Flip row. A click on a radio button goes to `set_flip_choice` at once.
    pub(super) fn draw_flip(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        let before = game.flip_choice();
        let mut choice = before;
        ui.horizontal(|ui| {
            ui.add_sized(
                [super::super::LABEL_WIDTH, 20.0],
                egui::Label::new("Flip next run"),
            );
            for option in FlipChoice::ALL {
                let radio = ui.radio_value(&mut choice, option, option.label());
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(format!("flip {}", option.label())),
                        radio.rect,
                    )
                });
                #[cfg(not(test))]
                let _ = radio;
            }
        });
        if choice != before {
            self.last = Some(set_flip_choice(game, choice).map(|()| "Flip choice set"));
        }
        ui.label(flip_label(game));
    }
}

#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod tests;
