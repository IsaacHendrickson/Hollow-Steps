//! Validated boss content. Runtime instances never read files or wall-clock time.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::behavior::{nonnegative, ron_files, Action};
use crate::body::{load_body_templates, BodyTemplates, BodyVisual};
use crate::boss_special::BossSpecials;
use crate::pose::{load_poses, Poses};

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
    /// The hitbox: contact damage and being hit use this size.
    pub size: (f32, f32),
    /// The size the boss is drawn at, when it should differ from the hitbox. Absent means the
    /// hitbox size. See `crate::visual_box`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visual_size: Option<(f32, f32)>,
    pub contact_damage: u16,
    pub selection: AttackSelection,
    pub pause_seconds: f32,
    pub attacks: Vec<AttackDefinition>,
    /// The boss's look. `None` keeps the plain box; the hitbox is always `size`.
    #[serde(default)]
    pub body: Option<BodyVisual>,
}

impl BossDefinition {
    pub fn from_ron(source: &str) -> Result<Self, String> {
        let definition: Self = ron::from_str(source).map_err(|e: ron::error::SpannedError| {
            let line = source
                .lines()
                .nth(e.span.start.line.saturating_sub(1))
                .unwrap_or_default()
                .trim();
            format!("{e}; near {line}")
        })?;
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
            crate::visual_box::validate(self.visual_size)?;
            nonnegative(self.pause_seconds).map_err(|e| format!("pause_seconds: {e}"))?;
            if let Some(body) = &self.body {
                body.validate()?;
            }
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

pub(crate) fn validate_action(action: &Action) -> Result<(), String> {
    action.validate()?;
    match action {
        Action::JumpToPlayer { speed, .. } | Action::SlamDown { speed } => {
            crate::behavior::positive(*speed).map_err(|e| format!("speed {e}"))?;
        }
        _ => {}
    }
    Ok(())
}

/// Validated boss content plus the body templates and poses the bosses draw on.
#[derive(Debug, Default)]
pub struct BossDefinitions {
    bosses: BTreeMap<String, Arc<BossDefinition>>,
    bodies: BodyTemplates,
    poses: Arc<Poses>,
}

/// Checks a boss against the body templates and poses it names.
fn validate_assets(
    definition: &BossDefinition,
    bodies: &BodyTemplates,
    poses: &Poses,
) -> Result<(), String> {
    if let Some(body) = &definition.body {
        body.validate_template(bodies)
            .map_err(|e| format!("boss {:?}: {e}", definition.name))?;
    }
    crate::pose::validate_boss(definition, poses)
}

impl BossDefinitions {
    /// Bosses without bodies; a boss that names a body template is rejected here.
    pub fn from_definitions(
        definitions: impl IntoIterator<Item = BossDefinition>,
    ) -> Result<Self, String> {
        Self::from_definitions_with_assets(definitions, BodyTemplates::new(), Poses::new())
    }

    pub fn from_definitions_with_assets(
        definitions: impl IntoIterator<Item = BossDefinition>,
        bodies: BodyTemplates,
        poses: Poses,
    ) -> Result<Self, String> {
        let mut catalog = BTreeMap::new();
        for definition in definitions {
            definition.validate()?;
            validate_assets(&definition, &bodies, &poses)?;
            let name = definition.name.clone();
            if catalog.insert(name.clone(), Arc::new(definition)).is_some() {
                return Err(format!("duplicate boss name {name:?}"));
            }
        }
        Ok(Self {
            bosses: catalog,
            bodies,
            poses: Arc::new(poses),
        })
    }

    pub fn size(&self, name: &str) -> Result<Vec2, String> {
        self.bosses
            .get(name)
            .map(|b| b.size.into())
            .ok_or_else(|| format!("missing boss definition {name:?}"))
    }

    pub fn load(assets: &Path) -> Result<Self, String> {
        let directory = assets.join("bosses");
        // Older asset packs without boss placements remain valid.
        if !directory.exists() {
            return Ok(Self::default());
        }
        let bodies = load_body_templates(&assets.join("bodies"))?;
        let poses = load_poses(&assets.join("poses"))?;
        let mut definitions = BTreeMap::new();
        for path in ron_files(&directory)? {
            let source =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let definition = BossDefinition::from_ron(&source)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            validate_assets(&definition, &bodies, &poses)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            let name = definition.name.clone();
            if definitions
                .insert(name.clone(), Arc::new(definition))
                .is_some()
            {
                return Err(format!("{}: duplicate boss name {name:?}", path.display()));
            }
        }
        Ok(Self {
            bosses: definitions,
            bodies,
            poses: Arc::new(poses),
        })
    }

    pub fn spawn(&self, name: &str, position: Vec2) -> Result<BossSpecials, String> {
        let definition = self
            .bosses
            .get(name)
            .ok_or_else(|| format!("missing boss definition {name:?}"))?;
        BossSpecials::from_definition_with_body(
            definition.clone(),
            position,
            &self.bodies,
            self.poses.clone(),
        )
    }
}
