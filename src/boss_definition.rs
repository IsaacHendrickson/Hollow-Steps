//! Validated boss content. Runtime instances never read files or wall-clock time.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::behavior::{nonnegative, read_ron, ron_files, Action};
use crate::boss_special::BossSpecials;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AttackSelection {
    Cycle,
    Random,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttackDefinition {
    pub name: String,
    pub actions: Vec<Action>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossDefinition {
    pub name: String,
    pub health: u32,
    pub size: (f32, f32),
    pub contact_damage: u8,
    pub selection: AttackSelection,
    pub pause_seconds: f32,
    pub attacks: Vec<AttackDefinition>,
}

impl BossDefinition {
    pub fn from_ron(source: &str) -> Result<Self, String> {
        let definition: Self = ron::from_str(source).map_err(|e| e.to_string())?;
        definition.validate()?;
        Ok(definition)
    }

    pub fn validate(&self) -> Result<(), String> {
        let check = || {
            if self.name.trim().is_empty() {
                return Err("name must not be empty".into());
            }
            if self.health == 0 {
                return Err("health must be positive".into());
            }
            if !Vec2::from(self.size).is_finite() || self.size.0 <= 0.0 || self.size.1 <= 0.0 {
                return Err("size must be finite and positive".into());
            }
            nonnegative(self.pause_seconds).map_err(|e| format!("pause_seconds: {e}"))?;
            if self.attacks.is_empty() {
                return Err("attacks must not be empty".into());
            }
            let mut names = std::collections::BTreeSet::new();
            for attack in &self.attacks {
                if attack.name.trim().is_empty() || !names.insert(&attack.name) {
                    return Err(format!("empty or duplicate attack name {:?}", attack.name));
                }
                if attack.actions.is_empty() {
                    return Err(format!(
                        "attack {:?}: actions must not be empty",
                        attack.name
                    ));
                }
                for (index, action) in attack.actions.iter().enumerate() {
                    validate_action(action)
                        .map_err(|e| format!("attack {:?}, action {index}: {e}", attack.name))?;
                }
            }
            Ok(())
        };
        check().map_err(|e: String| format!("boss {:?}: {e}", self.name))
    }
}

fn validate_action(action: &Action) -> Result<(), String> {
    for (name, value) in action.parameter_values() {
        nonnegative(value).map_err(|e| format!("{name}: {e}"))?;
        if name == "speed" && value == 0.0 {
            return Err("speed must be positive".into());
        }
        if name == "damage" && (value > 255.0 || value.fract() != 0.0) {
            return Err("damage must be an integer in 0..255".into());
        }
        if name == "count" && (!(1.0..=256.0).contains(&value) || value.fract() != 0.0) {
            return Err("count must be an integer in 1..256".into());
        }
    }
    match action {
        Action::MoveTowardPlayer { .. }
        | Action::MoveAwayFromPlayer { .. }
        | Action::Patrol { .. } => Err("enemy-only movement action".into()),
        _ => Ok(()),
    }
}

#[derive(Debug, Default)]
pub struct BossDefinitions(BTreeMap<String, Arc<BossDefinition>>);

impl BossDefinitions {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let directory = assets.join("bosses");
        // Older asset packs without boss placements remain valid.
        if !directory.exists() {
            return Ok(Self::default());
        }
        let mut definitions = BTreeMap::new();
        for path in ron_files(&directory)? {
            let definition: BossDefinition = read_ron(&path)?;
            definition
                .validate()
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let name = definition.name.clone();
            if definitions
                .insert(name.clone(), Arc::new(definition))
                .is_some()
            {
                return Err(format!("{}: duplicate boss name {name:?}", path.display()));
            }
        }
        Ok(Self(definitions))
    }

    pub fn spawn(&self, name: &str, position: Vec2) -> Result<BossSpecials, String> {
        let definition = self
            .0
            .get(name)
            .ok_or_else(|| format!("missing boss definition {name:?}"))?;
        BossSpecials::from_definition(definition.clone(), position)
    }
}
