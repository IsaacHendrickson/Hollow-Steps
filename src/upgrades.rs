//! Permanent upgrades: bonuses bought with banked coins at the hub shop and kept between runs.
//!
//! Pure rules, no window. `data/upgrades.ron` lists the upgrades (an `assets/upgrades.ron`
//! replaces it whole); the levels bought live in the `Profile` (`profile.ron`, by upgrade id);
//! `permanent_effects` turns them into the effects `charms::effective_with` layers on top of the
//! base numbers and the equipped charms. The base `player.ron` and `vitals.ron` are never
//! changed or saved from here, and `GameState` puts the result into effect (`game::charm_link`).
//!
//! An upgrade has one effect, the amount gained *per level*, and a price for each level: the
//! number of prices is the number of levels. An id the catalog no longer lists stays in the
//! profile and has no effect; a saved level above the catalog's last one counts as the last.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::read_override;
use crate::charms::{is_valid_id, Effect, MAX_DESCRIPTION_CHARS, MAX_ID_CHARS, MAX_NAME_CHARS};
use crate::profile::Profile;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "upgrades.ron";

const DEFAULTS: &str = include_str!("../data/upgrades.ron");

/// The most levels one upgrade may have (and so the most prices it lists).
pub const MAX_LEVELS: usize = 10;

/// One upgrade as `data/upgrades.ron` lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Upgrade {
    /// Stable name the profile remembers levels by; unique, never shown.
    pub id: String,
    /// Shown in the shop; 1 to 32 characters, unique ignoring case.
    pub name: String,
    pub description: String,
    /// What one level gives, checked with `Effect::validate`.
    pub effect: Effect,
    /// The price of level 1, level 2, and so on: 1 to 10 prices, each above 0.
    pub costs: Vec<u32>,
}

impl Upgrade {
    /// How many levels it has: one for each price.
    pub fn max_level(&self) -> u32 {
        u32::try_from(self.costs.len()).unwrap_or(u32::MAX)
    }

    /// Every field inside its range.
    pub fn validate(&self) -> Result<(), String> {
        if !is_valid_id(&self.id) {
            return Err(format!(
                "upgrade id \"{}\" must be 1 to {MAX_ID_CHARS} letters, digits, `_` or `-`",
                self.id
            ));
        }
        let label = format!("upgrade \"{}\"", self.id);
        let error = |what: String| Err(format!("{label}: {what}"));
        if self.name.trim().is_empty() || self.name.chars().count() > MAX_NAME_CHARS {
            return error(format!("name must be 1 to {MAX_NAME_CHARS} characters"));
        }
        if self.name != self.name.trim() {
            return error("name must not start or end with a space".into());
        }
        if self.description.chars().count() > MAX_DESCRIPTION_CHARS
            || self.description.contains('\n')
        {
            return error(format!(
                "description must be one line of at most {MAX_DESCRIPTION_CHARS} characters"
            ));
        }
        if let Err(problem) = self.effect.validate() {
            return error(problem);
        }
        if self.costs.is_empty() || self.costs.len() > MAX_LEVELS {
            return error(format!(
                "costs must list 1 to {MAX_LEVELS} prices (one per level), not {}",
                self.costs.len()
            ));
        }
        if let Some(level) = self.costs.iter().position(|cost| *cost == 0) {
            return error(format!("the price of level {} must be above 0", level + 1));
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UpgradeFile {
    upgrades: Vec<Upgrade>,
}

/// Every upgrade there is, in the order the file lists them (the order the shop shows them).
#[derive(Clone, Debug, PartialEq)]
pub struct UpgradeCatalog {
    upgrades: Vec<Upgrade>,
}

impl Default for UpgradeCatalog {
    /// The embedded `data/upgrades.ron`.
    fn default() -> Self {
        Self::from_ron(DEFAULTS).expect("valid upgrade defaults")
    }
}

impl UpgradeCatalog {
    /// Parses and validates a file's text.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let file: UpgradeFile =
            ron::from_str(text).map_err(|error| format!("upgrades: {error}"))?;
        let catalog = Self {
            upgrades: file.upgrades,
        };
        catalog.validate()?;
        Ok(catalog)
    }

    /// The `upgrades.ron` in `assets` if there is one (it replaces the defaults whole),
    /// otherwise the embedded `data/upgrades.ron`. A file that exists but cannot be read or is
    /// invalid is an error naming it, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Self::from_ron(DEFAULTS).map_err(|error| format!("data/upgrades.ron: {error}")),
        }
    }

    /// Each upgrade valid, ids unique, names unique ignoring case.
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for upgrade in &self.upgrades {
            upgrade.validate()?;
            if !ids.insert(upgrade.id.as_str()) {
                return Err(format!("upgrade id \"{}\" is used twice", upgrade.id));
            }
            if !names.insert(upgrade.name.to_lowercase()) {
                return Err(format!(
                    "upgrade name \"{}\" is used twice (names are compared ignoring case)",
                    upgrade.name
                ));
            }
        }
        Ok(())
    }

    pub fn upgrades(&self) -> &[Upgrade] {
        &self.upgrades
    }

    pub fn get(&self, id: &str) -> Option<&Upgrade> {
        self.upgrades.iter().find(|upgrade| upgrade.id == id)
    }
}

/// The level of upgrade `id` the profile has bought; 0 when it has none.
pub fn level(profile: &Profile, id: &str) -> u32 {
    profile.upgrades.get(id).copied().unwrap_or(0)
}

/// The price of the next level of `id`; none when the upgrade is unknown or already at its
/// last level.
pub fn next_cost(profile: &Profile, catalog: &UpgradeCatalog, id: &str) -> Option<u32> {
    let upgrade = catalog.get(id)?;
    let next = usize::try_from(level(profile, id)).ok()?;
    upgrade.costs.get(next).copied()
}

/// What `buy` did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Purchase {
    /// The coins were paid and the level went up by one.
    Bought,
    /// Nothing changed; the text says why.
    Refused(String),
}

/// Buys the next level of `id`: takes its price off the profile's coins and raises the level by
/// one. Refused (and nothing changes) for an id the catalog does not list, at the last level,
/// and when the coins fall short, saying by how many.
pub fn buy(profile: &mut Profile, catalog: &UpgradeCatalog, id: &str) -> Purchase {
    let Some(upgrade) = catalog.get(id) else {
        return Purchase::Refused("There is no such upgrade".to_owned());
    };
    let Some(cost) = next_cost(profile, catalog, id) else {
        return Purchase::Refused(format!(
            "The {} is already at its maximum level ({})",
            upgrade.name,
            upgrade.max_level()
        ));
    };
    if profile.coins < cost {
        return Purchase::Refused(format!(
            "Not enough coins: the {} costs {cost}, you need {} more",
            upgrade.name,
            cost - profile.coins
        ));
    }
    profile.coins -= cost;
    *profile.upgrades.entry(id.to_owned()).or_insert(0) += 1;
    Purchase::Bought
}

/// The effect of every level the profile owns: an upgrade at level 2 gives its effect twice.
/// Upgrades the catalog no longer lists give nothing, and a level above the catalog's last one
/// counts as the last.
pub fn permanent_effects(profile: &Profile, catalog: &UpgradeCatalog) -> Vec<Effect> {
    catalog
        .upgrades()
        .iter()
        .flat_map(|upgrade| {
            let owned = level(profile, &upgrade.id).min(upgrade.max_level());
            std::iter::repeat_n(upgrade.effect, owned as usize)
        })
        .collect()
}

#[cfg(test)]
mod tests;
