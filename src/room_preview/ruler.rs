//! The ruler of the header: how big the player is and how far and high each kind of jump goes,
//! from the real reach code (`crate::reach`), in pixels and in cells of the picture.
use std::path::Path;

use macroquad::math::Vec2;

use crate::ability::Ability;
use crate::charms::CharmCatalog;
use crate::player::PlayerConfig;
use crate::reach::Interval;
use crate::tuning::TuningConfig;
use crate::upgrades::{max_totals, UpgradeCatalog};
use crate::vitals::VitalsConfig;

/// The player's size and the reach numbers the preview prints.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ruler {
    pub player: Vec2,
    /// `without` is the best jump with no ability and every buff at its maximum; `with` is base
    /// stats, which is also the jump with no buffs and no ability (`crate::reach`).
    pub interval: Interval,
}

impl Ruler {
    /// The ruler of a base config and the same config with every buff applied.
    pub fn from_configs(base: &PlayerConfig, buffed: &PlayerConfig) -> Self {
        Self {
            player: base.size,
            interval: Interval::from_configs(base, buffed),
        }
    }

    /// The ruler at the shipped tuning: `player.ron`, the vitals, every charm and every upgrade
    /// of `assets` (the same inputs as the editor's Reach toggle). A missing `player.ron` means
    /// the built-in defaults, as everywhere else.
    pub fn shipped(assets: &Path) -> Result<Self, String> {
        let tuning = TuningConfig::load(assets)?;
        let vitals = VitalsConfig::load(assets)?;
        let charms = CharmCatalog::load(assets, vitals.max_mana)?;
        let upgrades = UpgradeCatalog::load(assets)?;
        Ok(Self {
            player: tuning.player.size,
            interval: Interval::new(&tuning, &vitals, &charms, &max_totals(&upgrades)),
        })
    }

    /// The ruler lines: the player, the jump with no ability (no buffs, then every buff at its
    /// maximum) and each ability's. `cell` is pixels per character.
    pub fn lines(&self, cell: f32) -> Vec<String> {
        let reach = |far: f32, high: f32| {
            format!(
                "far {:.0} px ({:.1} cells), high {:.0} px ({:.1} cells)",
                far,
                far / cell,
                high,
                high / cell
            )
        };
        let mut rows = vec![
            (
                "jump, no ability, no buffs".to_owned(),
                reach(
                    self.interval.with.jump_distance,
                    self.interval.with.jump_height,
                ),
            ),
            (
                "jump, no ability, every buff at max".to_owned(),
                reach(
                    self.interval.without.jump_distance,
                    self.interval.without.jump_height,
                ),
            ),
        ];
        for ability in Ability::ALL {
            rows.push((
                format!("jump with {}", ability.name()),
                reach(
                    self.interval.gap(*ability).high,
                    self.interval.height(*ability).high,
                ),
            ));
        }
        let width = rows.iter().map(|(name, _)| name.len()).max().unwrap_or(0);
        let mut lines = vec![format!(
            "Ruler: player {:.0} x {:.0} px ({:.1} x {:.1} cells)",
            self.player.x,
            self.player.y,
            self.player.x / cell,
            self.player.y / cell
        )];
        lines.extend(
            rows.into_iter()
                .map(|(name, numbers)| format!("  {name:<width$}  {numbers}")),
        );
        lines
    }
}
