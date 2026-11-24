//! The "Generate next run" row of the F4 window's "Run (debug)" section: Auto, Never or Always, as
//! radio buttons (`docs/dungeon-gen.md`). Auto leaves it to `variety.ron` (`generator.enabled`)
//! and the pool; Never plays the fixed dungeon; Always asks the generator even with the rules off,
//! so a pool that cannot build comes back with the generator's reason when the run does not
//! start. The choice is for the runs that start after it; a run already going keeps its own
//! dungeon. It is never written to `variety.ron` or the profile.
//!
//! The logic is `set_generate_choice` and `generate_label`, which only call `GameState`; the
//! widgets just feed them. The three-way choice is the flips' (`FlipChoice`), so the row is drawn
//! with the same buttons.
use super::flip::choice_row;
use super::RunDebug;
use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::variety::GenerateChoice;

/// Sets what the next runs do about generating. Never fails; it only changes the choice.
pub fn set_generate_choice(game: &mut GameState, choice: GenerateChoice) -> Result<(), String> {
    game.set_generate_choice(choice);
    Ok(())
}

/// The read-only line under the radio buttons: what the next run will do, and whether the run in
/// progress (or on the result screen) is generated.
pub fn generate_label(game: &GameState) -> String {
    let next = match game.generate_choice() {
        GenerateChoice::Never => "the next run plays the fixed dungeon".to_owned(),
        GenerateChoice::Always => {
            "the next run is generated, or does not start and says why".to_owned()
        }
        GenerateChoice::Auto if game.variety_rules().generator.enabled => {
            "the next run is generated when the pool has a Start and a Boss piece (variety.ron)"
                .to_owned()
        }
        GenerateChoice::Auto => {
            "the next run plays the fixed dungeon: generator.enabled is off in variety.ron"
                .to_owned()
        }
    };
    match (game.run_seed().is_some(), game.run_generated()) {
        (true, true) => format!("This run is generated; {next}"),
        (true, false) => format!("This run is the fixed dungeon; {next}"),
        (false, _) => {
            let mut text = next;
            text[..1].make_ascii_uppercase();
            text
        }
    }
}

impl RunDebug {
    /// Draws the Generate row. A click on a radio button goes to `set_generate_choice` at once.
    pub(super) fn draw_generate(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        let before = game.generate_choice();
        let choice = choice_row(ui, "Generate next run", "generate", before);
        if choice != before {
            self.last = Some(set_generate_choice(game, choice).map(|()| "Generate choice set"));
        }
        ui.label(generate_label(game));
    }
}

#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod tests;
