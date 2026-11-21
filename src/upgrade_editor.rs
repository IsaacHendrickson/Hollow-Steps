//! The upgrade list as the editor edits it: a draft of `upgrades.ron` (saved list, edited list,
//! dirty), a declarative field table, validation, reset, New upgrade, Delete and the staged save.
//! Pure rules, no window; the editor's Upgrades panel (`upgrade_panel`) draws this one draft.
//!
//! The field table is `FIELDS` (every field of an upgrade, with how it is drawn) plus the numeric
//! ranges: `COSTS`, one `FieldSpec` per level, and the effect amounts, which are the charms' own
//! (`charm_editor::EffectKind::spec`) so a charm and an upgrade of the same kind accept the same
//! numbers. A new field is one `FIELDS` line (and its member in `Upgrade`); a test serializes an
//! upgrade and fails when a field written to the file has no `FIELDS` entry.
//!
//! Three levels of checking, each in its own place:
//! - a number: `set_cost` and `set_effect_amount` refuse a value outside its range (or not a
//!   number) with the reason, so the draft never holds one;
//! - text: the draft holds whatever was typed, and `problems` names the field (a name of 0 or more
//!   than 32 characters, a description that is not one short line), because a text box cannot
//!   refuse a key press;
//! - several upgrades together: the id and the name are unique.
//!
//! `problems` and `validate` block Save; an invalid draft writes nothing. `UpgradeCatalog::new`
//! agrees with them: a draft without problems always makes a valid catalog (a test checks).
//!
//! Deleting an upgrade only removes it from the list. The levels players bought stay in
//! `profile.ron` and have no effect until an upgrade with that id is listed again.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::charm_editor::{amount, EffectKind};
use crate::charms::{is_valid_id, Effect, MAX_DESCRIPTION_CHARS, MAX_ID_CHARS, MAX_NAME_CHARS};
use crate::snapshot_history::Snapshots;
use crate::tuning::{write_staged, FieldSpec};
use crate::upgrades::{
    self, max_totals_of, MaxTotals, Upgrade, UpgradeCatalog, FILE_NAME, MAX_LEVELS,
};

/// The largest price a level may have: the catalog's own ceiling, as the drag box's end.
pub const MAX_COST: f32 = upgrades::MAX_COST as f32;

const fn cost(key: &'static str, label: &'static str) -> FieldSpec {
    FieldSpec {
        key,
        label,
        group: "Upgrades",
        unit: "coins",
        min: 1.0,
        max: MAX_COST,
        step: 1.0,
        decimals: 0,
        integer: true,
    }
}

/// The range of the price of each level, in level order (`COSTS[0]` is level 1).
pub const COSTS: [FieldSpec; MAX_LEVELS] = [
    cost("costs.1", "Level 1"),
    cost("costs.2", "Level 2"),
    cost("costs.3", "Level 3"),
    cost("costs.4", "Level 4"),
    cost("costs.5", "Level 5"),
    cost("costs.6", "Level 6"),
    cost("costs.7", "Level 7"),
    cost("costs.8", "Level 8"),
    cost("costs.9", "Level 9"),
    cost("costs.10", "Level 10"),
];

/// The range of one level's effect amount: the charms' range for that kind, called "Per level"
/// because an upgrade gives it once for every level bought.
pub fn amount_spec(kind: EffectKind) -> FieldSpec {
    FieldSpec {
        label: "Per level",
        ..*kind.spec()
    }
}

/// How the panel draws a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Shown, never edited: the id is a stable name.
    ReadOnly,
    /// A text box (the only ones in the panel); at most this many characters.
    Text(usize),
    /// The kind dropdown and the per-level amount drag box (`EffectKind`, `amount_spec`).
    Effect,
    /// One price drag box per level (`COSTS`), with Add level and Remove level.
    Costs,
}

/// One field of an upgrade as `upgrades.ron` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpgradeField {
    /// The name in the file.
    pub key: &'static str,
    pub label: &'static str,
    pub control: Control,
}

const fn field(key: &'static str, label: &'static str, control: Control) -> UpgradeField {
    UpgradeField {
        key,
        label,
        control,
    }
}

/// Every field of an upgrade, in the order the panel draws them.
pub const FIELDS: [UpgradeField; 5] = [
    field("id", "Id", Control::ReadOnly),
    field("name", "Name", Control::Text(MAX_NAME_CHARS)),
    field(
        "description",
        "Description",
        Control::Text(MAX_DESCRIPTION_CHARS),
    ),
    field("effect", "Effect", Control::Effect),
    field("costs", "Prices", Control::Costs),
];

/// Why a draft cannot be saved: which upgrade and field, and what to say next to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    /// Index into `UpgradeDraft::edited`.
    pub upgrade: usize,
    /// A `FIELDS` key.
    pub field: &'static str,
    pub message: String,
}

#[derive(Serialize)]
struct FileOut<'a> {
    upgrades: &'a [Upgrade],
}

/// The upgrades as saved and as edited.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpgradeDraft {
    saved: Vec<Upgrade>,
    edited: Vec<Upgrade>,
}

impl UpgradeDraft {
    /// A clean draft: the edited list is the saved one.
    pub fn new(saved: Vec<Upgrade>) -> Self {
        Self {
            edited: saved.clone(),
            saved,
        }
    }

    pub fn from_catalog(catalog: &UpgradeCatalog) -> Self {
        Self::new(catalog.upgrades().to_vec())
    }

    pub fn saved(&self) -> &[Upgrade] {
        &self.saved
    }

    pub fn edited(&self) -> &[Upgrade] {
        &self.edited
    }

    /// True when the edited list differs from the saved one (an upgrade changed, added, deleted
    /// or moved).
    pub fn is_dirty(&self) -> bool {
        self.edited != self.saved
    }

    /// True when upgrade `index` differs from its saved version, or has none (a new upgrade).
    pub fn is_upgrade_dirty(&self, index: usize) -> bool {
        let Some(upgrade) = self.edited.get(index) else {
            return false;
        };
        self.saved.iter().find(|saved| saved.id == upgrade.id) != Some(upgrade)
    }

    /// Replaces the edited list (undo and redo). It was a draft's list once, so it is held as it
    /// is; `problems` still judges it.
    pub fn set_edited(&mut self, edited: Vec<Upgrade>) {
        self.edited = edited;
    }

    fn upgrade_mut(&mut self, index: usize) -> Result<&mut Upgrade, String> {
        self.edited
            .get_mut(index)
            .ok_or_else(|| "no such upgrade".into())
    }

    /// Sets the name as typed; `problems` says what is wrong with it.
    pub fn set_name(&mut self, index: usize, text: &str) -> Result<(), String> {
        self.upgrade_mut(index)?.name = text.to_owned();
        Ok(())
    }

    /// Sets the description as typed; `problems` says what is wrong with it.
    pub fn set_description(&mut self, index: usize, text: &str) -> Result<(), String> {
        self.upgrade_mut(index)?.description = text.to_owned();
        Ok(())
    }

    /// Sets the amount one level gives, in the range of its kind (whole kinds are rounded). A
    /// value outside the range, or not a number, is refused and the draft is unchanged.
    pub fn set_effect_amount(&mut self, index: usize, value: f32) -> Result<(), String> {
        let upgrade = self.upgrade_mut(index)?;
        let kind = EffectKind::of(&upgrade.effect);
        let value = kind.spec().check(value)?;
        upgrade.effect = kind.effect(value);
        Ok(())
    }

    /// Changes the kind of the effect. The amount is kept, held inside the new kind's range.
    pub fn set_effect_kind(&mut self, index: usize, kind: EffectKind) -> Result<(), String> {
        let upgrade = self.upgrade_mut(index)?;
        let spec = kind.spec();
        let kept = amount(&upgrade.effect).clamp(spec.min, spec.max);
        upgrade.effect = kind.effect(kept);
        Ok(())
    }

    /// Sets the price of `level` (0 is level 1): a whole number from 1 to `MAX_COST`. A value
    /// outside that, or not a number, is refused with the reason and the draft is unchanged.
    pub fn set_cost(&mut self, index: usize, level: usize, value: f32) -> Result<(), String> {
        let upgrade = self.upgrade_mut(index)?;
        let spec = COSTS.get(level).ok_or("no such level")?;
        let slot = upgrade.costs.get_mut(level).ok_or("no such level")?;
        *slot = spec.check(value)? as u32;
        Ok(())
    }

    /// Adds a level at the end, priced like the one before it. Refused past `MAX_LEVELS`.
    pub fn add_level(&mut self, index: usize) -> Result<(), String> {
        let upgrade = self.upgrade_mut(index)?;
        if upgrade.costs.len() >= MAX_LEVELS {
            return Err(format!("an upgrade has at most {MAX_LEVELS} levels"));
        }
        let price = upgrade.costs.last().copied().unwrap_or(10);
        upgrade.costs.push(price);
        Ok(())
    }

    /// Removes the last level. An upgrade keeps at least one.
    pub fn remove_level(&mut self, index: usize) -> Result<(), String> {
        let upgrade = self.upgrade_mut(index)?;
        if upgrade.costs.len() <= 1 {
            return Err("an upgrade needs at least one level".into());
        }
        upgrade.costs.pop();
        Ok(())
    }

    /// Upgrade `index` back to its saved version, in its place. An upgrade that was never saved
    /// is removed.
    pub fn reset_upgrade(&mut self, index: usize) {
        let Some(upgrade) = self.edited.get(index) else {
            return;
        };
        match self.saved.iter().find(|saved| saved.id == upgrade.id) {
            Some(saved) => self.edited[index] = saved.clone(),
            None => {
                self.edited.remove(index);
            }
        }
    }

    /// Every upgrade back to the saved list: edits undone, new upgrades removed, deleted ones
    /// back.
    pub fn reset_all(&mut self) {
        self.edited = self.saved.clone();
    }

    /// Adds an upgrade at the end and returns its index. Its id is `upgrade_N` and its name
    /// `Upgrade N`, for the lowest N from 1 where neither is taken (names ignoring case). It gives
    /// +5 max health per level and has one level that costs 10 coins.
    pub fn new_upgrade(&mut self) -> usize {
        let taken = |number: u32| {
            let (id, name) = (format!("upgrade_{number}"), format!("Upgrade {number}"));
            self.edited
                .iter()
                .any(|upgrade| upgrade.id == id || upgrade.name.trim().eq_ignore_ascii_case(&name))
        };
        let number = (1..).find(|&number| !taken(number)).expect("a free number");
        self.edited.push(Upgrade {
            id: format!("upgrade_{number}"),
            name: format!("Upgrade {number}"),
            description: String::new(),
            effect: Effect::MaxHealth(5),
            costs: vec![10],
        });
        self.edited.len() - 1
    }

    /// Removes upgrade `index` from the edited list and returns it. The levels in the profile are
    /// not touched: they stay, with no effect.
    pub fn delete(&mut self, index: usize) -> Result<Upgrade, String> {
        if index >= self.edited.len() {
            return Err("no such upgrade".into());
        }
        Ok(self.edited.remove(index))
    }

    /// The ids in the saved list that the edited list no longer has.
    pub fn deleted_ids(&self) -> Vec<&str> {
        self.saved
            .iter()
            .filter(|saved| !self.edited.iter().any(|upgrade| upgrade.id == saved.id))
            .map(|saved| saved.id.as_str())
            .collect()
    }

    /// What every upgrade gives at its last level, for the line under the list. Counts the
    /// edited list as it is, valid or not.
    pub fn totals(&self) -> MaxTotals {
        max_totals_of(&self.edited)
    }

    /// Everything wrong with the edited list, upgrade by upgrade, in a fixed order.
    pub fn problems(&self) -> Vec<Problem> {
        let mut problems = Vec::new();
        let (mut ids, mut names) = (HashSet::new(), HashSet::new());
        for (index, upgrade) in self.edited.iter().enumerate() {
            let mut add = |field: &'static str, message: String| {
                problems.push(Problem {
                    upgrade: index,
                    field,
                    message,
                });
            };
            if !is_valid_id(&upgrade.id) {
                add(
                    "id",
                    format!("the id must be 1 to {MAX_ID_CHARS} letters, digits, `_` or `-`"),
                );
            } else if !ids.insert(upgrade.id.as_str()) {
                add("id", format!("the id \"{}\" is used twice", upgrade.id));
            }
            let length = upgrade.name.chars().count();
            if upgrade.name.trim().is_empty() || length > MAX_NAME_CHARS {
                add(
                    "name",
                    format!("the name must be 1 to {MAX_NAME_CHARS} characters"),
                );
            } else if upgrade.name != upgrade.name.trim() {
                add(
                    "name",
                    "the name must not start or end with a space".to_owned(),
                );
            } else if !names.insert(upgrade.name.to_lowercase()) {
                add(
                    "name",
                    format!(
                        "another upgrade is already called \"{}\" (names are compared ignoring \
                         case)",
                        upgrade.name
                    ),
                );
            }
            if upgrade.description.chars().count() > MAX_DESCRIPTION_CHARS
                || upgrade.description.contains('\n')
            {
                add(
                    "description",
                    format!(
                        "the description must be one line of at most {MAX_DESCRIPTION_CHARS} \
                         characters"
                    ),
                );
            }
            if let Err(message) = upgrade.effect.validate() {
                add("effect", message);
            }
            if upgrade.costs.is_empty() || upgrade.costs.len() > MAX_LEVELS {
                add(
                    "costs",
                    format!(
                        "an upgrade needs 1 to {MAX_LEVELS} levels, not {}",
                        upgrade.costs.len()
                    ),
                );
            }
            for (level, price) in upgrade.costs.iter().enumerate() {
                if *price == 0 {
                    add(
                        "costs",
                        format!("the price of level {} must be above 0", level + 1),
                    );
                } else if *price > upgrades::MAX_COST {
                    add(
                        "costs",
                        format!(
                            "the price of level {} must be at most {}",
                            level + 1,
                            upgrades::MAX_COST
                        ),
                    );
                }
            }
        }
        problems
    }

    /// The first problem with `field` of upgrade `index`, for the message next to it.
    pub fn problem_for(&self, index: usize, field: &str) -> Option<String> {
        self.problems()
            .into_iter()
            .find(|problem| problem.upgrade == index && problem.field == field)
            .map(|problem| problem.message)
    }

    /// `Ok` when the edited list may be saved or played; otherwise the first problem, naming
    /// the upgrade.
    pub fn validate(&self) -> Result<(), String> {
        match self.problems().into_iter().next() {
            Some(problem) => Err(format!(
                "upgrade \"{}\": {}",
                self.edited[problem.upgrade].name, problem.message
            )),
            None => Ok(()),
        }
    }

    /// The edited list as a catalog, when it has no problems.
    pub fn catalog(&self) -> Result<UpgradeCatalog, String> {
        self.validate()?;
        UpgradeCatalog::new(self.edited.clone())
    }

    /// The file text, in the pretty format the editors use.
    pub fn to_ron(&self) -> Result<String, String> {
        self.validate()?;
        crate::definition_editor::serialize(&FileOut {
            upgrades: &self.edited,
        })
    }

    /// The edited list is now the saved one (it was just written).
    pub fn mark_saved(&mut self) {
        self.saved = self.edited.clone();
    }

    /// Writes the edited list to `upgrades.ron` in `assets`, staged so a failed write leaves the
    /// old file, and marks it saved. An invalid draft writes nothing.
    pub fn save(&mut self, assets: &Path) -> Result<PathBuf, String> {
        let text = self.to_ron()?;
        let path = assets.join(FILE_NAME);
        write_staged(&path, &text)?;
        self.mark_saved();
        Ok(path)
    }
}

impl Snapshots<Upgrade> for UpgradeDraft {
    fn snapshot(&self) -> &[Upgrade] {
        self.edited()
    }

    fn restore(&mut self, list: Vec<Upgrade>) {
        self.set_edited(list);
    }
}

#[cfg(test)]
mod tests;
