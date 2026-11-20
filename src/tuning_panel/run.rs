//! The F4 window's "Run (debug)" section: the difficulty tier and a Reset profile button, for
//! trying the hub-and-run loop without playing a dungeon to the end. Both change the saved
//! profile (`profile.ron`) at once.
//!
//! The logic is `set_tier` and `reset_profile`, which only call `GameState::set_profile`; the
//! widgets just feed them. Nothing here changes how a run plays.
use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::profile::Profile;

/// The highest tier the debug box allows.
pub const MAX_TIER: u32 = 99;

/// What the section last did, shown under its buttons.
#[derive(Debug, Default)]
pub struct RunDebug {
    last: Option<Result<&'static str, String>>,
}

/// Sets the difficulty tier (at most `MAX_TIER`) and saves the profile. Coins and clears stay.
/// A failed save keeps the new tier for this session and returns why.
pub fn set_tier(game: &mut GameState, tier: u32) -> Result<(), String> {
    let profile = Profile {
        tier: tier.min(MAX_TIER),
        ..*game.profile()
    };
    game.set_profile(profile)
}

/// Puts coins, tier and cleared back to zero and saves the profile.
pub fn reset_profile(game: &mut GameState) -> Result<(), String> {
    game.set_profile(Profile::default())
}

impl RunDebug {
    /// Draws the section. With no hub in `run.ron` there are no runs, so it only says so.
    pub fn draw(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        ui.add_space(4.0);
        ui.strong("Run (debug)");
        if game.run_phase().is_none() {
            ui.label("run.ron names no hub room, so there are no runs.");
            return;
        }
        let profile = *game.profile();
        ui.label(format!(
            "Banked coins {}  |  dungeons cleared {}",
            profile.coins, profile.cleared
        ));
        let mut tier = profile.tier;
        ui.horizontal(|ui| {
            ui.add_sized(
                [super::LABEL_WIDTH, 20.0],
                egui::Label::new("Difficulty tier"),
            );
            let drag = ui.add(
                egui::DragValue::new(&mut tier)
                    .range(0..=MAX_TIER)
                    .speed(0.1),
            );
            #[cfg(test)]
            ui.data_mut(|data| data.insert_temp(egui::Id::new("tier box"), drag.rect));
            if drag.changed() {
                self.last = Some(set_tier(game, tier).map(|()| "Tier saved"));
            }
        });
        let reset = ui.button("Reset profile");
        #[cfg(test)]
        ui.data_mut(|data| data.insert_temp(egui::Id::new("reset profile button"), reset.rect));
        if reset.clicked() {
            self.last = Some(reset_profile(game).map(|()| "Profile reset"));
        }
        match &self.last {
            Some(Ok(done)) => {
                ui.label(*done);
            }
            Some(Err(error)) => {
                ui.colored_label(egui::Color32::LIGHT_RED, error.as_str());
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod draw_tests;
#[cfg(test)]
mod tests;
