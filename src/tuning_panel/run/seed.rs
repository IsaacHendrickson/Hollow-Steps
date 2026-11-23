//! The Seed row of the F4 window's "Run (debug)" section: a "Fixed seed" checkbox, a text box
//! for the seed, and a read-only label with the seed of the run in progress or of the last run.
//! While the box is checked every run starts with the seed in the text box (`docs/variety.md`).
//!
//! The logic is `set_fixed_seed` and `seed_label`, which only call `GameState`; the widgets just
//! feed them, like `set_tier`. A bad text is refused with the reason and the seed in effect stays.
//!
//! AGENTS.md says text boxes are for names only and numbers get drag boxes. A seed is an
//! identifier, not a tuned number: it is copied from the result screen or shared as text, it
//! has no range to drag over, and 16 hex digits do not fit a drag box. So a text box is right
//! here, validated by `seed::parse_seed`.
use super::RunDebug;
use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::seed::{format_seed, parse_seed};

const CHECKBOX_HINT: &str = "While checked, every run starts with the seed in the box (hex, \
                             1 to 16 digits). A run already going keeps its own seed.";

/// Turns the fixed seed on with the seed in `text` (hex, 1 to 16 digits, either case, an optional
/// `0x`, spaces around it ignored), or off. Off never reads `text`. A text that is not a seed
/// is refused with the reason and nothing changes: a seed already fixed stays fixed, and a seed
/// that was not fixed does not become fixed.
pub fn set_fixed_seed(game: &mut GameState, on: bool, text: &str) -> Result<(), String> {
    if !on {
        game.set_fixed_seed(None);
        return Ok(());
    }
    let seed = parse_seed(text.trim())?;
    game.set_fixed_seed(Some(seed));
    Ok(())
}

/// The read-only label under the checkbox: the seed of the run in progress (or whose result is
/// showing), otherwise the seed of the last run, otherwise that no run has started yet.
pub fn seed_label(game: &GameState) -> String {
    match (game.run_seed(), game.last_run_seed()) {
        (Some(seed), _) => format!("Seed of this run: {}", format_seed(seed)),
        (None, Some(seed)) => format!("Seed of the last run: {}", format_seed(seed)),
        (None, None) => "No run has started yet, so there is no seed".to_owned(),
    }
}

impl RunDebug {
    /// Draws the Seed row. A change goes to `set_fixed_seed` at once and its outcome shows under
    /// the section's buttons.
    pub(super) fn draw_seed(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        let fixed = game.fixed_seed().is_some();
        let mut on = fixed;
        ui.horizontal(|ui| {
            let check = ui
                .checkbox(&mut on, "Fixed seed")
                .on_hover_text(CHECKBOX_HINT);
            #[cfg(test)]
            ui.data_mut(|data| data.insert_temp(egui::Id::new("fixed seed checkbox"), check.rect));
            let edit = ui.add(
                egui::TextEdit::singleline(&mut self.seed_text)
                    .hint_text("hex digits, such as 1f3a")
                    .desired_width(150.0),
            );
            #[cfg(test)]
            ui.data_mut(|data| data.insert_temp(egui::Id::new("seed text box"), edit.rect));
            if check.changed() {
                self.last = Some(set_fixed_seed(game, on, &self.seed_text).map(|()| {
                    if on {
                        "Fixed seed on"
                    } else {
                        "Fixed seed off"
                    }
                }));
            } else if edit.changed() && fixed {
                // Typing while the seed is fixed changes it; a bad text keeps the old seed.
                self.last = Some(
                    set_fixed_seed(game, true, &self.seed_text).map(|()| "Fixed seed changed"),
                );
            }
        });
        ui.label(seed_label(game));
    }
}

#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod tests;
