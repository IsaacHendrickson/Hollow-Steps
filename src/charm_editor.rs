//! The charm list as the editor edits it: a draft of `charms.ron` (saved list, edited list,
//! dirty), a declarative field table, validation, reset, New charm, Delete and the staged save.
//! Pure rules, no window; the editor's Charms panel (`charm_panel`) draws this one draft.
//!
//! The field table is `FIELDS` (every field of a charm, with how it is drawn) plus the numeric
//! ranges: `MANA_COST` and `AMOUNTS`, one `FieldSpec` per effect kind. A new field is one
//! `FIELDS` line (and its member in `Charm`); a test serializes the default charm and fails when
//! a field written to the file has no `FIELDS` entry.
//!
//! Three levels of checking, each in its own place:
//! - a number: `set_mana_cost` and `set_effect_amount` refuse a value outside its range (or not a
//!   number) with the reason, so the draft never holds one;
//! - text and color: the draft holds whatever was typed, and `problems` names the field (a name
//!   of 0 or more than 32 characters, a description that is not one short line, a fully
//!   transparent color), because a text box cannot refuse a key press;
//! - several charms together: the id and the name are unique, and a charm costs at most the base
//!   maximum mana (`problems` takes it, from the saved `vitals.ron`).
//!
//! `problems` and `validate` block Save; an invalid draft writes nothing. `CharmCatalog::new`
//! agrees with them: a draft without problems always makes a valid catalog (a test checks).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::charms::{
    number_text, Charm, CharmCatalog, Effect, Rgba, FILE_NAME, MAX_DAMAGE_BONUS,
    MAX_DESCRIPTION_CHARS, MAX_HEALTH_BONUS, MAX_ID_CHARS, MAX_NAME_CHARS, MAX_SPEED_PERCENT,
    MIN_SPEED_PERCENT,
};
use crate::tuning::{write_staged, FieldSpec};

/// What a charm does, without the amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectKind {
    MaxHealth,
    MoveSpeedPercent,
    SwordDamage,
}

impl EffectKind {
    pub const ALL: [Self; 3] = [Self::MaxHealth, Self::MoveSpeedPercent, Self::SwordDamage];

    pub fn of(effect: &Effect) -> Self {
        match effect {
            Effect::MaxHealth(_) => Self::MaxHealth,
            Effect::MoveSpeedPercent(_) => Self::MoveSpeedPercent,
            Effect::SwordDamage(_) => Self::SwordDamage,
        }
    }

    /// How the dropdown names it.
    pub fn label(self) -> &'static str {
        match self {
            Self::MaxHealth => "Max health",
            Self::MoveSpeedPercent => "Move speed",
            Self::SwordDamage => "Sword damage",
        }
    }

    /// The range of this kind's amount (`AMOUNTS`).
    pub fn spec(self) -> &'static FieldSpec {
        &AMOUNTS[self as usize]
    }

    /// The effect of this kind with `amount` (whole kinds are rounded; the caller checked the
    /// range).
    pub fn effect(self, amount: f32) -> Effect {
        match self {
            Self::MaxHealth => Effect::MaxHealth(amount.round() as u16),
            Self::MoveSpeedPercent => Effect::MoveSpeedPercent(amount),
            Self::SwordDamage => Effect::SwordDamage(amount.round() as u16),
        }
    }
}

/// The amount inside `effect`.
pub fn amount(effect: &Effect) -> f32 {
    match *effect {
        Effect::MaxHealth(points) | Effect::SwordDamage(points) => f32::from(points),
        Effect::MoveSpeedPercent(percent) => percent,
    }
}

const fn number(
    key: &'static str,
    label: &'static str,
    unit: &'static str,
    (min, max, step): (f32, f32, f32),
    integer: bool,
) -> FieldSpec {
    FieldSpec {
        key,
        label,
        group: "Charms",
        unit,
        min,
        max,
        step,
        decimals: if integer { 0 } else { 1 },
        integer,
    }
}

/// The largest mana cost a charm may have at all; the base maximum mana is the real limit
/// (`problems`), and it is at most this (`vitals.max_mana`).
pub const MAX_MANA_COST: f32 = 10_000.0;

/// The mana cost. Its `max` is the hard ceiling; the panel and `problems` hold a charm to the
/// base maximum mana.
pub const MANA_COST: FieldSpec = number(
    "mana_cost",
    "Mana cost",
    "mana",
    (0.0, MAX_MANA_COST, 0.5),
    false,
);

/// The range of the effect amount for each `EffectKind`, in `EffectKind::ALL` order.
pub const AMOUNTS: [FieldSpec; 3] = [
    number(
        "effect.MaxHealth",
        "Amount",
        "points",
        (1.0, MAX_HEALTH_BONUS as f32, 1.0),
        true,
    ),
    number(
        "effect.MoveSpeedPercent",
        "Amount",
        "%",
        (MIN_SPEED_PERCENT, MAX_SPEED_PERCENT, 0.5),
        false,
    ),
    number(
        "effect.SwordDamage",
        "Amount",
        "points",
        (1.0, MAX_DAMAGE_BONUS as f32, 1.0),
        true,
    ),
];

/// How the panel draws a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    /// Shown, never edited: the id is a stable name.
    ReadOnly,
    /// A text box (the only ones in the panel); at most this many characters.
    Text(usize),
    /// A drag box with a unit, from `MANA_COST`.
    Number,
    /// The kind dropdown and the amount drag box (`EffectKind`, `AMOUNTS`).
    Effect,
    Color,
    Toggle,
}

/// One field of a charm as `charms.ron` writes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CharmField {
    /// The name in the file.
    pub key: &'static str,
    pub label: &'static str,
    pub control: Control,
}

const fn field(key: &'static str, label: &'static str, control: Control) -> CharmField {
    CharmField {
        key,
        label,
        control,
    }
}

/// Every field of a charm, in the order the panel draws them.
pub const FIELDS: [CharmField; 7] = [
    field("id", "Id", Control::ReadOnly),
    field("name", "Name", Control::Text(MAX_NAME_CHARS)),
    field(
        "description",
        "Description",
        Control::Text(MAX_DESCRIPTION_CHARS),
    ),
    field("mana_cost", "Mana cost", Control::Number),
    field("effect", "Effect", Control::Effect),
    field("color", "Color", Control::Color),
    field("owned_at_start", "Owned at start", Control::Toggle),
];

/// Why a draft cannot be saved: which charm and field, and what to say next to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    /// Index into `CharmDraft::edited`.
    pub charm: usize,
    /// A `FIELDS` key.
    pub field: &'static str,
    pub message: String,
}

#[derive(Serialize)]
struct FileOut<'a> {
    charms: &'a [Charm],
}

/// The charms as saved and as edited.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CharmDraft {
    saved: Vec<Charm>,
    edited: Vec<Charm>,
}

impl CharmDraft {
    /// A clean draft: the edited list is the saved one.
    pub fn new(saved: Vec<Charm>) -> Self {
        Self {
            edited: saved.clone(),
            saved,
        }
    }

    pub fn from_catalog(catalog: &CharmCatalog) -> Self {
        Self::new(catalog.charms().to_vec())
    }

    pub fn saved(&self) -> &[Charm] {
        &self.saved
    }

    pub fn edited(&self) -> &[Charm] {
        &self.edited
    }

    /// True when the edited list differs from the saved one (a charm changed, added, deleted or
    /// moved).
    pub fn is_dirty(&self) -> bool {
        self.edited != self.saved
    }

    /// True when charm `index` differs from its saved version, or has none (a new charm).
    pub fn is_charm_dirty(&self, index: usize) -> bool {
        let Some(charm) = self.edited.get(index) else {
            return false;
        };
        self.saved.iter().find(|saved| saved.id == charm.id) != Some(charm)
    }

    /// Replaces the edited list (undo and redo). It was a draft's list once, so it is held as it
    /// is; `problems` still judges it.
    pub fn set_edited(&mut self, edited: Vec<Charm>) {
        self.edited = edited;
    }

    fn charm_mut(&mut self, index: usize) -> Result<&mut Charm, String> {
        self.edited
            .get_mut(index)
            .ok_or_else(|| "no such charm".into())
    }

    /// Sets the name as typed; `problems` says what is wrong with it.
    pub fn set_name(&mut self, index: usize, text: &str) -> Result<(), String> {
        self.charm_mut(index)?.name = text.to_owned();
        Ok(())
    }

    /// Sets the description as typed; `problems` says what is wrong with it.
    pub fn set_description(&mut self, index: usize, text: &str) -> Result<(), String> {
        self.charm_mut(index)?.description = text.to_owned();
        Ok(())
    }

    /// Sets the mana cost. A value outside 0 to `MAX_MANA_COST`, or not a number, is refused
    /// with the reason and the draft is unchanged. (Above the base maximum mana is a problem for
    /// `problems`, not a refusal.)
    pub fn set_mana_cost(&mut self, index: usize, value: f32) -> Result<(), String> {
        let value = MANA_COST.check(value)?;
        self.charm_mut(index)?.mana_cost = value;
        Ok(())
    }

    /// Sets the amount of the effect, in the range of its kind (whole kinds are rounded). A value
    /// outside the range, or not a number, is refused and the draft is unchanged.
    pub fn set_effect_amount(&mut self, index: usize, value: f32) -> Result<(), String> {
        let charm = self.charm_mut(index)?;
        let kind = EffectKind::of(&charm.effect);
        let value = kind.spec().check(value)?;
        charm.effect = kind.effect(value);
        Ok(())
    }

    /// Changes the kind of the effect. The amount is kept, held inside the new kind's range.
    pub fn set_effect_kind(&mut self, index: usize, kind: EffectKind) -> Result<(), String> {
        let charm = self.charm_mut(index)?;
        let spec = kind.spec();
        let kept = amount(&charm.effect).clamp(spec.min, spec.max);
        charm.effect = kind.effect(kept);
        Ok(())
    }

    pub fn set_color(&mut self, index: usize, color: Rgba) -> Result<(), String> {
        self.charm_mut(index)?.color = color;
        Ok(())
    }

    pub fn set_owned_at_start(&mut self, index: usize, owned: bool) -> Result<(), String> {
        self.charm_mut(index)?.owned_at_start = owned;
        Ok(())
    }

    /// Charm `index` back to its saved version, in its place. A charm that was never saved is
    /// removed.
    pub fn reset_charm(&mut self, index: usize) {
        let Some(charm) = self.edited.get(index) else {
            return;
        };
        match self.saved.iter().find(|saved| saved.id == charm.id) {
            Some(saved) => self.edited[index] = saved.clone(),
            None => {
                self.edited.remove(index);
            }
        }
    }

    /// Every charm back to the saved list: edits undone, new charms removed, deleted ones back.
    pub fn reset_all(&mut self) {
        self.edited = self.saved.clone();
    }

    /// Adds a charm at the end and returns its index. Its id is `charm_N` and its name
    /// `Charm N`, for the lowest N from 1 where neither is taken (names ignoring case). It costs
    /// no mana, gives +5 max health, is grey and is not owned at the start (a room can hold it).
    pub fn new_charm(&mut self) -> usize {
        let taken = |number: u32| {
            let (id, name) = (format!("charm_{number}"), format!("Charm {number}"));
            self.edited
                .iter()
                .any(|charm| charm.id == id || charm.name.trim().eq_ignore_ascii_case(&name))
        };
        let number = (1..).find(|&number| !taken(number)).expect("a free number");
        self.edited.push(Charm {
            id: format!("charm_{number}"),
            name: format!("Charm {number}"),
            description: String::new(),
            mana_cost: 0.0,
            effect: Effect::MaxHealth(5),
            color: [160, 160, 160, 255],
            owned_at_start: false,
        });
        self.edited.len() - 1
    }

    /// Removes charm `index` from the edited list and returns it. The game unequips it and drops
    /// it from the owned list when the list is applied (`game::charm_editor_link`).
    pub fn delete(&mut self, index: usize) -> Result<Charm, String> {
        if index >= self.edited.len() {
            return Err("no such charm".into());
        }
        Ok(self.edited.remove(index))
    }

    /// The ids in the saved list that the edited list no longer has.
    pub fn deleted_ids(&self) -> Vec<&str> {
        self.saved
            .iter()
            .filter(|saved| !self.edited.iter().any(|charm| charm.id == saved.id))
            .map(|saved| saved.id.as_str())
            .collect()
    }

    /// Everything wrong with the edited list, charm by charm, in a fixed order.
    /// `base_max_mana` is the maximum mana without charms, from the saved `vitals.ron`.
    pub fn problems(&self, base_max_mana: f32) -> Vec<Problem> {
        let mut problems = Vec::new();
        let (mut ids, mut names) = (HashSet::new(), HashSet::new());
        for (index, charm) in self.edited.iter().enumerate() {
            let mut add = |field: &'static str, message: String| {
                problems.push(Problem {
                    charm: index,
                    field,
                    message,
                });
            };
            if charm.id.is_empty()
                || charm.id.chars().count() > MAX_ID_CHARS
                || !charm
                    .id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                add(
                    "id",
                    format!("the id must be 1 to {MAX_ID_CHARS} letters, digits, `_` or `-`"),
                );
            } else if !ids.insert(charm.id.as_str()) {
                add("id", format!("the id \"{}\" is used twice", charm.id));
            }
            let length = charm.name.chars().count();
            if charm.name.trim().is_empty() || length > MAX_NAME_CHARS {
                add(
                    "name",
                    format!("the name must be 1 to {MAX_NAME_CHARS} characters"),
                );
            } else if charm.name != charm.name.trim() {
                add(
                    "name",
                    "the name must not start or end with a space".to_owned(),
                );
            } else if !names.insert(charm.name.to_lowercase()) {
                add(
                    "name",
                    format!(
                        "another charm is already called \"{}\" (names are compared ignoring case)",
                        charm.name
                    ),
                );
            }
            if charm.description.chars().count() > MAX_DESCRIPTION_CHARS
                || charm.description.contains('\n')
            {
                add(
                    "description",
                    format!(
                        "the description must be one line of at most {MAX_DESCRIPTION_CHARS} \
                         characters"
                    ),
                );
            }
            if let Err(message) = MANA_COST.check(charm.mana_cost) {
                add("mana_cost", message);
            } else if charm.mana_cost > base_max_mana {
                add(
                    "mana_cost",
                    format!(
                        "mana_cost must not be above the base max mana ({})",
                        number_text(base_max_mana)
                    ),
                );
            }
            if let Err(message) = charm.effect.validate() {
                add("effect", message);
            }
            if charm.color[3] == 0 {
                add(
                    "color",
                    "the color must not be fully transparent (alpha 0)".to_owned(),
                );
            }
        }
        problems
    }

    /// The first problem with `field` of charm `index`, for the message next to it.
    pub fn problem_for(&self, index: usize, field: &str, base_max_mana: f32) -> Option<String> {
        self.problems(base_max_mana)
            .into_iter()
            .find(|problem| problem.charm == index && problem.field == field)
            .map(|problem| problem.message)
    }

    /// `Ok` when the edited list may be saved or played; otherwise the first problem, naming
    /// the charm.
    pub fn validate(&self, base_max_mana: f32) -> Result<(), String> {
        match self.problems(base_max_mana).into_iter().next() {
            Some(problem) => Err(format!(
                "charm \"{}\": {}",
                self.edited[problem.charm].name, problem.message
            )),
            None => Ok(()),
        }
    }

    /// The edited list as a catalog, when it has no problems.
    pub fn catalog(&self, base_max_mana: f32) -> Result<CharmCatalog, String> {
        self.validate(base_max_mana)?;
        CharmCatalog::new(self.edited.clone(), base_max_mana)
    }

    /// The file text, in the pretty format the editors use.
    pub fn to_ron(&self, base_max_mana: f32) -> Result<String, String> {
        self.validate(base_max_mana)?;
        crate::definition_editor::serialize(&FileOut {
            charms: &self.edited,
        })
    }

    /// The edited list is now the saved one (it was just written).
    pub fn mark_saved(&mut self) {
        self.saved = self.edited.clone();
    }

    /// Writes the edited list to `charms.ron` in `assets`, staged so a failed write leaves the
    /// old file, and marks it saved. An invalid draft writes nothing.
    pub fn save(&mut self, assets: &Path, base_max_mana: f32) -> Result<PathBuf, String> {
        let text = self.to_ron(base_max_mana)?;
        let path = assets.join(FILE_NAME);
        write_staged(&path, &text)?;
        self.mark_saved();
        Ok(path)
    }
}

#[cfg(test)]
mod tests;
