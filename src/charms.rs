//! Charms: equippable bonuses that cost mana capacity.
//!
//! Pure rules, no window. `data/charms.ron` lists the charms (an `assets/charms.ron` replaces it
//! whole); `CharmState` says which are owned and which are equipped; `effective` layers the
//! equipped charms on top of the base `player.ron` and `vitals.ron` numbers. The base numbers are
//! never changed or saved from here: `GameState` keeps them in its `PlayerDraft`, and applies the
//! result of `effective` to the player and the vitals (`game::charm_link`).
//!
//! The mana rule: every equipped charm takes its `mana_cost` off the maximum mana, and a charm
//! cannot be equipped when the equipped charms plus it would cost more than the *base* maximum
//! mana. Unequipping is always allowed.

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::assets::read_override;
use crate::tuning::TuningConfig;
use crate::vitals::VitalsConfig;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "charms.ron";

const DEFAULTS: &str = include_str!("../data/charms.ron");

/// Longest a charm's name, id and description may be, in characters.
pub const MAX_NAME_CHARS: usize = 32;
pub const MAX_ID_CHARS: usize = 48;
pub const MAX_DESCRIPTION_CHARS: usize = 200;
/// Largest bonus a single charm may give.
pub const MAX_HEALTH_BONUS: u16 = 1_000;
pub const MAX_DAMAGE_BONUS: u16 = 1_000;
pub const MIN_SPEED_PERCENT: f32 = 1.0;
pub const MAX_SPEED_PERCENT: f32 = 200.0;
/// A sum of mana costs this close to the budget still fits (float noise).
const MANA_EPSILON: f32 = 1e-4;

/// The start of the message for a mana cost outside 0 to the base max mana. `with_cost_hint`
/// recognizes an error by it.
const COST_RANGE: &str = "mana_cost must be between 0 and the base max mana";

/// What to do when the files disagree about mana, as the last line of the load error.
const COST_HINT: &str = "Hint: lower the charm's mana_cost in assets/charms.ron, or raise \
                         max_mana in assets/vitals.ron (or delete that file).";

/// `error` with `COST_HINT` on a line of its own when it is about a cost the max mana cannot
/// cover; any other error is unchanged.
fn with_cost_hint(error: String) -> String {
    if error.contains(COST_RANGE) {
        format!("{error}\n{COST_HINT}")
    } else {
        error
    }
}

/// Red, green, blue, alpha.
pub type Rgba = [u8; 4];

/// What a charm does. One effect per charm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// This many more points of maximum health.
    MaxHealth(u16),
    /// Walk this many percent faster than the base move speed.
    MoveSpeedPercent(f32),
    /// This many more points of damage per sword hit.
    SwordDamage(u16),
}

impl Effect {
    /// The amount inside its allowed range.
    pub fn validate(&self) -> Result<(), String> {
        match *self {
            Self::MaxHealth(points) if (1..=MAX_HEALTH_BONUS).contains(&points) => Ok(()),
            Self::MaxHealth(_) => Err(format!(
                "MaxHealth must be between 1 and {MAX_HEALTH_BONUS}"
            )),
            Self::SwordDamage(points) if (1..=MAX_DAMAGE_BONUS).contains(&points) => Ok(()),
            Self::SwordDamage(_) => Err(format!(
                "SwordDamage must be between 1 and {MAX_DAMAGE_BONUS}"
            )),
            Self::MoveSpeedPercent(percent)
                if percent.is_finite()
                    && (MIN_SPEED_PERCENT..=MAX_SPEED_PERCENT).contains(&percent) =>
            {
                Ok(())
            }
            Self::MoveSpeedPercent(_) => Err(format!(
                "MoveSpeedPercent must be between {MIN_SPEED_PERCENT} and {MAX_SPEED_PERCENT}"
            )),
        }
    }

    /// The effect as the inventory shows it, for example `+10 max health`.
    pub fn text(&self) -> String {
        match *self {
            Self::MaxHealth(points) => format!("+{points} max health"),
            Self::MoveSpeedPercent(percent) => format!("+{}% move speed", number_text(percent)),
            Self::SwordDamage(points) => format!("+{points} sword damage"),
        }
    }
}

/// `value` for display: at most one decimal, and no `.0` on a whole number (`15`, `7.5`).
pub fn number_text(value: f32) -> String {
    format!("{}", (value * 10.0).round() / 10.0)
}

/// One charm as `data/charms.ron` lists it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Charm {
    /// Stable name for remembering ownership; unique, never shown.
    pub id: String,
    /// Shown in the inventory; 1 to 32 characters, unique ignoring case.
    pub name: String,
    pub description: String,
    /// Mana capacity taken while equipped, from 0 up to the base maximum mana.
    pub mana_cost: f32,
    pub effect: Effect,
    /// The swatch color.
    pub color: Rgba,
    /// Owned from the start of a run.
    pub owned_at_start: bool,
}

impl Charm {
    /// Every field inside its range. `base_max_mana` is the maximum mana without charms.
    pub fn validate(&self, base_max_mana: f32) -> Result<(), String> {
        let label = format!("charm \"{}\"", self.id);
        let error = |what: String| Err(format!("{label}: {what}"));
        if self.id.is_empty()
            || self.id.chars().count() > MAX_ID_CHARS
            || !self
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            return Err(format!(
                "charm id \"{}\" must be 1 to {MAX_ID_CHARS} letters, digits, `_` or `-`",
                self.id
            ));
        }
        let name_chars = self.name.chars().count();
        if self.name.trim().is_empty() || name_chars > MAX_NAME_CHARS {
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
        if !self.mana_cost.is_finite() || !(0.0..=base_max_mana).contains(&self.mana_cost) {
            return error(format!("{COST_RANGE} ({})", number_text(base_max_mana)));
        }
        if let Err(problem) = self.effect.validate() {
            return error(problem);
        }
        if self.color[3] == 0 {
            return error("color must not be fully transparent (alpha 0)".into());
        }
        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CharmFile {
    charms: Vec<Charm>,
}

/// Every charm there is, in the order the file lists them.
#[derive(Clone, Debug, PartialEq)]
pub struct CharmCatalog {
    charms: Vec<Charm>,
}

impl Default for CharmCatalog {
    /// The embedded `data/charms.ron`, checked against the shipped `max_mana`.
    fn default() -> Self {
        Self::from_ron(DEFAULTS, VitalsConfig::default().max_mana).expect("valid charm defaults")
    }
}

impl CharmCatalog {
    /// A catalog of `charms`, checked against `base_max_mana` like a file's would be.
    pub fn new(charms: Vec<Charm>, base_max_mana: f32) -> Result<Self, String> {
        let catalog = Self { charms };
        catalog.validate(base_max_mana)?;
        Ok(catalog)
    }

    /// Parses and validates a file's text against `base_max_mana`.
    pub fn from_ron(text: &str, base_max_mana: f32) -> Result<Self, String> {
        let file: CharmFile = ron::from_str(text).map_err(|error| format!("charms: {error}"))?;
        let catalog = Self {
            charms: file.charms,
        };
        catalog.validate(base_max_mana)?;
        Ok(catalog)
    }

    /// The `charms.ron` in `assets` if there is one (it replaces the defaults whole), otherwise
    /// the embedded `data/charms.ron`. A file that exists but cannot be read or is invalid is an
    /// error naming it, never a fallback.
    ///
    /// A cost the max mana cannot cover (a hand edit of either file) ends the error with one hint
    /// line saying which file to change.
    pub fn load(assets: &Path, base_max_mana: f32) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text, base_max_mana)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Self::from_ron(DEFAULTS, base_max_mana)
                .map_err(|error| format!("data/charms.ron: {error}")),
        }
        .map_err(with_cost_hint)
    }

    /// Each charm valid, ids unique, names unique ignoring case.
    pub fn validate(&self, base_max_mana: f32) -> Result<(), String> {
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for charm in &self.charms {
            charm.validate(base_max_mana)?;
            if !ids.insert(charm.id.as_str()) {
                return Err(format!("charm id \"{}\" is used twice", charm.id));
            }
            if !names.insert(charm.name.to_lowercase()) {
                return Err(format!(
                    "charm name \"{}\" is used twice (names are compared ignoring case)",
                    charm.name
                ));
            }
        }
        Ok(())
    }

    pub fn charms(&self) -> &[Charm] {
        &self.charms
    }

    pub fn get(&self, id: &str) -> Option<&Charm> {
        self.charms.iter().find(|charm| charm.id == id)
    }
}

/// What `CharmState::toggle` did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Toggle {
    Equipped,
    Unequipped,
    /// Nothing changed; the text says why.
    Refused(String),
}

/// Which charms are owned and which are equipped, by id. Both lists keep their order and hold
/// no duplicates; every equipped charm is owned.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharmState {
    owned: Vec<String>,
    equipped: Vec<String>,
}

impl CharmState {
    /// The charms marked `owned_at_start` are owned, nothing is equipped.
    pub fn starting(catalog: &CharmCatalog) -> Self {
        Self {
            owned: catalog
                .charms()
                .iter()
                .filter(|charm| charm.owned_at_start)
                .map(|charm| charm.id.clone())
                .collect(),
            equipped: Vec::new(),
        }
    }

    pub fn owned(&self) -> &[String] {
        &self.owned
    }

    pub fn equipped(&self) -> &[String] {
        &self.equipped
    }

    pub fn is_owned(&self, id: &str) -> bool {
        self.owned.iter().any(|owned| owned == id)
    }

    pub fn is_equipped(&self, id: &str) -> bool {
        self.equipped.iter().any(|equipped| equipped == id)
    }

    /// The equipped charms, in the order they were equipped.
    pub fn equipped_charms<'a>(&self, catalog: &'a CharmCatalog) -> Vec<&'a Charm> {
        self.equipped
            .iter()
            .filter_map(|id| catalog.get(id))
            .collect()
    }

    /// The mana the equipped charms cost together.
    pub fn mana_used(&self, catalog: &CharmCatalog) -> f32 {
        self.equipped_charms(catalog)
            .iter()
            .map(|charm| charm.mana_cost)
            .sum()
    }

    /// Maximum mana with the equipped charms: the base maximum minus what they cost, never
    /// below zero.
    pub fn max_mana(&self, catalog: &CharmCatalog, base_max_mana: f32) -> f32 {
        (base_max_mana - self.mana_used(catalog)).max(0.0)
    }

    /// Why `id` cannot be equipped, or `Ok`. The charm must exist, be owned and not be equipped
    /// already, and the equipped charms plus it must not cost more than `base_max_mana`.
    pub fn can_equip(
        &self,
        catalog: &CharmCatalog,
        id: &str,
        base_max_mana: f32,
    ) -> Result<(), String> {
        let charm = catalog.get(id).ok_or("There is no such charm")?;
        if !self.is_owned(id) {
            return Err(format!("You do not own the {}", charm.name));
        }
        if self.is_equipped(id) {
            return Err(format!("The {} is already equipped", charm.name));
        }
        let used = self.mana_used(catalog);
        if used + charm.mana_cost > base_max_mana + MANA_EPSILON {
            let missing = used + charm.mana_cost - base_max_mana;
            let free = (base_max_mana - used).max(0.0);
            return Err(format!(
                "Not enough mana: the {} needs {} more (it costs {}, {} free)",
                charm.name,
                number_text(missing),
                number_text(charm.mana_cost),
                number_text(free),
            ));
        }
        Ok(())
    }

    /// Equips `id` at the end of the equipped list if `can_equip` allows it.
    pub fn equip(
        &mut self,
        catalog: &CharmCatalog,
        id: &str,
        base_max_mana: f32,
    ) -> Result<(), String> {
        self.can_equip(catalog, id, base_max_mana)?;
        self.equipped.push(id.to_owned());
        Ok(())
    }

    /// Takes `id` off. Always allowed; returns whether it was equipped.
    pub fn unequip(&mut self, id: &str) -> bool {
        let before = self.equipped.len();
        self.equipped.retain(|equipped| equipped != id);
        self.equipped.len() != before
    }

    /// Unequips an equipped charm, equips an unequipped one if the rules allow.
    pub fn toggle(&mut self, catalog: &CharmCatalog, id: &str, base_max_mana: f32) -> Toggle {
        if self.unequip(id) {
            return Toggle::Unequipped;
        }
        match self.equip(catalog, id, base_max_mana) {
            Ok(()) => Toggle::Equipped,
            Err(message) => Toggle::Refused(message),
        }
    }

    /// Makes `id` owned. False if it already was.
    pub fn grant(&mut self, id: &str) -> bool {
        if self.is_owned(id) {
            return false;
        }
        self.owned.push(id.to_owned());
        true
    }

    /// Owns every charm of `catalog` marked `owned_at_start`.
    pub fn grant_starting(&mut self, catalog: &CharmCatalog) {
        for charm in catalog.charms().iter().filter(|charm| charm.owned_at_start) {
            self.grant(&charm.id);
        }
    }

    /// Takes off charms from the last equipped backwards until the rest cost no more than
    /// `base_max_mana`. Returns the ids taken off, in that order.
    pub fn trim_to_budget(&mut self, catalog: &CharmCatalog, base_max_mana: f32) -> Vec<String> {
        let mut taken_off = Vec::new();
        while self.mana_used(catalog) > base_max_mana + MANA_EPSILON {
            let Some(id) = self.equipped.pop() else { break };
            taken_off.push(id);
        }
        taken_off
    }

    /// Forgets every charm `catalog` no longer lists (after a reload).
    pub fn retain_known(&mut self, catalog: &CharmCatalog) {
        self.owned.retain(|id| catalog.get(id).is_some());
        self.equipped.retain(|id| catalog.get(id).is_some());
    }
}

/// The player and vitals numbers with `equipped` layered on top of the base ones.
///
/// Bonuses add up: max health and sword damage by points, move speed by percent of the *base*
/// speed. The costs of the equipped charms come off `max_mana` (never below zero, and the
/// starting mana is held to the new cap). The base values are only read.
pub fn effective(
    base: &TuningConfig,
    vitals: &VitalsConfig,
    equipped: &[&Charm],
) -> (TuningConfig, VitalsConfig) {
    let (mut health, mut damage, mut percent, mut cost) = (0u32, 0u32, 0.0f32, 0.0f32);
    for charm in equipped {
        if charm.mana_cost.is_finite() {
            cost += charm.mana_cost;
        }
        match charm.effect {
            Effect::MaxHealth(points) => health = health.saturating_add(u32::from(points)),
            Effect::SwordDamage(points) => damage = damage.saturating_add(u32::from(points)),
            Effect::MoveSpeedPercent(more) if more.is_finite() => percent += more,
            Effect::MoveSpeedPercent(_) => {}
        }
    }
    let cap = |base: u16, more: u32| (u32::from(base) + more).min(u32::from(u16::MAX)) as u16;
    let mut tuning = *base;
    tuning.player.max_health = cap(base.player.max_health, health);
    tuning.player.melee.damage = cap(base.player.melee.damage, damage);
    tuning.player.move_speed = base.player.move_speed * (1.0 + percent / 100.0);
    let mut vitals = *vitals;
    vitals.max_mana = (vitals.max_mana - cost).max(0.0);
    vitals.mana_start = vitals.mana_start.min(vitals.max_mana);
    (tuning, vitals)
}

#[cfg(test)]
mod tests;
