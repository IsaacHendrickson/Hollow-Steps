//! The F4 window's "Difficulty" section: the six numbers of `difficulty.ron` (what each tier adds
//! to enemy and boss health and damage and to a run's coins, and the tier the numbers stop
//! growing at), what they come to at the debug tier of the Run (debug) section, and a Save button
//! that writes `assets/difficulty.ron`.
//!
//! The logic is `set_percent`, `set_cap`, `readout` and `save`, which only call
//! `GameState::set_difficulty`, `save_difficulty` and `Difficulty::scale`; the widgets just feed
//! them. A value that fails the rules is refused with the reason and the box snaps back, so
//! nothing invalid is ever in effect or written. An edit is for the next run: a run in progress
//! keeps the numbers it began with, and F5 does not re-read the file.
use egui_macroquad::egui;
use hollow_steps::difficulty::{Difficulty, MAX_EFFECTIVE_TIER, MAX_PERCENT, MIN_EFFECTIVE_TIER};
use hollow_steps::game::GameState;

/// One of the five percents of `difficulty.ron`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Percent {
    EnemyHealth,
    EnemyDamage,
    BossHealth,
    BossDamage,
    Coins,
}

impl Percent {
    pub const ALL: [Self; 5] = [
        Self::EnemyHealth,
        Self::EnemyDamage,
        Self::BossHealth,
        Self::BossDamage,
        Self::Coins,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::EnemyHealth => "Enemy health per tier",
            Self::EnemyDamage => "Enemy damage per tier",
            Self::BossHealth => "Boss health per tier",
            Self::BossDamage => "Boss damage per tier",
            Self::Coins => "Coins per tier",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            Self::EnemyHealth => "Percent added to every enemy's health for each tier above 0.",
            Self::EnemyDamage => {
                "Percent added to the damage enemies deal for each tier above 0. Hazards are not scaled."
            }
            Self::BossHealth => "Percent added to a boss's health for each tier above 0.",
            Self::BossDamage => "Percent added to the damage bosses deal for each tier above 0.",
            Self::Coins => "Percent added to the coins a run earns for each tier above 0.",
        }
    }

    pub fn get(self, rules: &Difficulty) -> f32 {
        match self {
            Self::EnemyHealth => rules.enemy_health_percent,
            Self::EnemyDamage => rules.enemy_damage_percent,
            Self::BossHealth => rules.boss_health_percent,
            Self::BossDamage => rules.boss_damage_percent,
            Self::Coins => rules.coin_percent,
        }
    }

    /// The rules with this percent set to `value` (not checked here).
    pub fn with(self, rules: &Difficulty, value: f32) -> Difficulty {
        let mut changed = *rules;
        match self {
            Self::EnemyHealth => changed.enemy_health_percent = value,
            Self::EnemyDamage => changed.enemy_damage_percent = value,
            Self::BossHealth => changed.boss_health_percent = value,
            Self::BossDamage => changed.boss_damage_percent = value,
            Self::Coins => changed.coin_percent = value,
        }
        changed
    }
}

/// Sets one percent for the next run. A value outside 0 to 100, or not a number, is refused
/// with the reason and nothing changes.
pub fn set_percent(game: &mut GameState, field: Percent, value: f32) -> Result<(), String> {
    game.set_difficulty(field.with(game.difficulty(), value))
}

/// Sets the tier the numbers stop growing at (1 to 99). Out of range is refused with the reason.
pub fn set_cap(game: &mut GameState, tier: u32) -> Result<(), String> {
    game.set_difficulty(Difficulty {
        max_effective_tier: tier,
        ..*game.difficulty()
    })
}

/// Writes `assets/difficulty.ron` and returns where. A failed write keeps the rules in effect
/// and unsaved, and says which file failed.
pub fn save(game: &mut GameState) -> Result<String, String> {
    game.save_difficulty()
        .map(|path| path.display().to_string())
}

/// What the rules come to at `tier`, one short line each, for the debug tier of the Run
/// section: the tier that counts (the cap, when the tier is above it) and the multipliers.
pub fn readout(rules: &Difficulty, tier: u32) -> Vec<String> {
    let scale = rules.scale(tier);
    let counted = rules.effective_tier(tier);
    let head = if tier == 0 {
        "Tier 0: every number as authored".to_owned()
    } else if counted != tier {
        format!("Tier {tier} plays as tier {counted}, the cap")
    } else {
        format!("Tier {tier}")
    };
    vec![
        head,
        format!(
            "Enemies: health x{:.2}, damage x{:.2}",
            scale.enemy_health, scale.enemy_damage
        ),
        format!(
            "Bosses: health x{:.2}, damage x{:.2}",
            scale.boss_health, scale.boss_damage
        ),
        format!("Coins: x{:.2}", scale.coin),
    ]
}

/// What the section last did, shown under its Save button.
#[derive(Debug, Default)]
pub struct DifficultySection {
    last: Option<Result<String, String>>,
}

impl DifficultySection {
    pub fn draw(&mut self, ui: &mut egui::Ui, game: &mut GameState) {
        ui.add_space(4.0);
        ui.strong("Difficulty");
        ui.small("Percent added per tier above 0. A run uses these from its next start.");
        let rules = *game.difficulty();
        for field in Percent::ALL {
            let mut value = field.get(&rules);
            ui.horizontal(|ui| {
                ui.add_sized([super::LABEL_WIDTH, 20.0], egui::Label::new(field.label()));
                let drag = ui
                    .add(
                        egui::DragValue::new(&mut value)
                            .range(0.0..=MAX_PERCENT)
                            .speed(0.1)
                            .fixed_decimals(1)
                            .suffix(" %"),
                    )
                    .on_hover_text(field.hint());
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new(("difficulty box", field.label())), drag.rect)
                });
                if drag.changed() {
                    self.last = set_percent(game, field, value).err().map(Err);
                }
            });
        }
        let mut cap = rules.max_effective_tier;
        ui.horizontal(|ui| {
            ui.add_sized([super::LABEL_WIDTH, 20.0], egui::Label::new("Tier cap"));
            let drag = ui
                .add(
                    egui::DragValue::new(&mut cap)
                        .range(MIN_EFFECTIVE_TIER..=MAX_EFFECTIVE_TIER)
                        .speed(0.1),
                )
                .on_hover_text("A tier above this plays as this one.");
            #[cfg(test)]
            ui.data_mut(|data| data.insert_temp(egui::Id::new("difficulty cap box"), drag.rect));
            if drag.changed() {
                self.last = set_cap(game, cap).err().map(Err);
            }
        });
        let tier = game.profile().tier;
        ui.label("At the tier set above (Difficulty tier):");
        for line in readout(game.difficulty(), tier) {
            ui.label(line);
        }
        let save_button = ui.button("Save difficulty");
        #[cfg(test)]
        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new("save difficulty button"), save_button.rect)
        });
        if save_button.clicked() {
            self.last = Some(save(game));
        }
        ui.label(if game.difficulty_unsaved() {
            "Unsaved changes. F5 does not reload difficulty.ron."
        } else {
            "Matches the saved rules."
        });
        match &self.last {
            Some(Ok(path)) => {
                ui.label(format!("Saved to {path}"));
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
