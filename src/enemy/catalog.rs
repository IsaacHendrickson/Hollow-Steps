//! The catalog of validated enemy definitions loaded from `assets/` (entities, behaviors, body
//! templates and poses), and the spawning of an `Enemy` from one of them.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use macroquad::math::Vec2;

use super::{Enemy, EnemyDefinition};
use crate::behavior::{load_behaviors, read_ron, ron_files, BehaviorDefinition};
use crate::body::{load_body_templates, BodyTemplates};
use crate::player::WorldBounds;

/// Validated asset definitions, shared by all room instances.
#[derive(Debug)]
pub struct EnemyDefinitions {
    entities: BTreeMap<String, EnemyDefinition>,
    behaviors: BTreeMap<String, Arc<BehaviorDefinition>>,
    bodies: BodyTemplates,
    poses: Arc<crate::pose::Poses>,
}

impl EnemyDefinitions {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let behaviors = load_behaviors(&assets.join("behaviors"))?;
        let bodies = load_body_templates(&assets.join("bodies"))?;
        let poses = Arc::new(crate::pose::load_poses(&assets.join("poses"))?);
        for (name, behavior) in &behaviors {
            crate::pose::validate_behavior(behavior, &poses).map_err(|e| {
                format!(
                    "{}: {e}",
                    assets
                        .join("behaviors")
                        .join(format!("{name}.ron"))
                        .display()
                )
            })?;
        }
        let mut entities = BTreeMap::new();
        for path in ron_files(&assets.join("entities"))? {
            let entity: EnemyDefinition = read_ron(&path)?;
            entity
                .validate()
                .map_err(|error| format!("{}: {error}", path.display()))?;
            if !behaviors.contains_key(&entity.behavior) {
                return Err(format!(
                    "{}: entity {:?}: missing behavior name {:?}",
                    path.display(),
                    entity.name,
                    entity.behavior
                ));
            }
            crate::pose::validate_entity(&entity, &behaviors[&entity.behavior], &poses)
                .map_err(|e| format!("{}: {e}", path.display()))?;
            if let Some(body) = &entity.body {
                body.validate_template(&bodies)
                    .map_err(|e| format!("{}: entity {:?}: {e}", path.display(), entity.name))?;
            }
            if entities.contains_key(&entity.name) {
                return Err(format!(
                    "{}: duplicate entity name {:?}",
                    path.display(),
                    entity.name
                ));
            }
            entities.insert(entity.name.clone(), entity);
        }
        Ok(Self {
            entities,
            behaviors,
            bodies,
            poses,
        })
    }

    pub fn from_definitions(
        entities: impl IntoIterator<Item = EnemyDefinition>,
        behaviors: BTreeMap<String, Arc<BehaviorDefinition>>,
        bodies: BodyTemplates,
    ) -> Result<Self, String> {
        Self::from_definitions_with_poses(entities, behaviors, bodies, crate::pose::Poses::new())
    }

    pub fn from_definitions_with_poses(
        entities: impl IntoIterator<Item = EnemyDefinition>,
        behaviors: BTreeMap<String, Arc<BehaviorDefinition>>,
        bodies: BodyTemplates,
        poses: crate::pose::Poses,
    ) -> Result<Self, String> {
        for (name, pose) in &poses {
            crate::pose::validate_name(name).map_err(|e| format!("pose {name:?}: {e}"))?;
            pose.validate().map_err(|e| format!("pose {name:?}: {e}"))?;
        }
        for (name, template) in &bodies {
            template
                .validate()
                .map_err(|e| format!("body template {name:?}: {e}"))?;
        }
        for behavior in behaviors.values() {
            behavior.validate()?;
            crate::pose::validate_behavior(behavior, &poses)?;
        }
        let mut result = BTreeMap::new();
        for entity in entities {
            entity.validate()?;
            if let Some(body) = &entity.body {
                body.validate_template(&bodies)
                    .map_err(|e| format!("entity {:?}: {e}", entity.name))?;
            }
            if !behaviors.contains_key(&entity.behavior) {
                return Err(format!("missing behavior {:?}", entity.behavior));
            }
            crate::pose::validate_entity(&entity, &behaviors[&entity.behavior], &poses)?;
            if result.insert(entity.name.clone(), entity).is_some() {
                return Err("duplicate entity name".into());
            }
        }
        Ok(Self {
            entities: result,
            behaviors,
            bodies,
            poses: Arc::new(poses),
        })
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entities.keys().map(String::as_str)
    }

    pub fn size(&self, name: &str) -> Result<Vec2, String> {
        let entity = self
            .entities
            .get(name)
            .ok_or_else(|| format!("missing enemy definition {name:?}"))?;
        Ok(Vec2::new(entity.size.0, entity.size.1))
    }

    pub fn spawn(
        &self,
        name: &str,
        left: f32,
        right: f32,
        bounds: WorldBounds,
    ) -> Result<Enemy, String> {
        let entity = self
            .entities
            .get(name)
            .ok_or_else(|| format!("missing enemy definition {name:?}"))?;
        let mut enemy = Enemy::from_definition_with_templates(
            entity,
            self.behaviors[&entity.behavior].clone(),
            &self.bodies,
            left,
            right,
            bounds,
        )?;
        enemy.poses = self.poses.clone();
        Ok(enemy)
    }
}
