use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use macroquad::math::Vec2;
use serde::Deserialize;

use crate::behavior::{
    load_behaviors, read_ron, ron_files, Action, Behavior, BehaviorContext, BehaviorDefinition,
    Condition,
};
use crate::player::WorldBounds;

pub const ENEMY_MAX_HEALTH: u8 = 3;
pub const KNOCKBACK_SPEED: f32 = 260.0;
pub const KNOCKBACK_SECONDS: f32 = 0.16;

pub const ENEMY_SIZE: Vec2 = Vec2::new(36.0, 36.0);

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub name: String,
    pub health: u8,
    pub size: (f32, f32),
    pub contact_damage: u8,
    pub behavior: String,
}

impl EnemyDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("entity name must not be empty".into());
        }
        if self.health == 0 {
            return Err(format!("entity {:?}: health must be positive", self.name));
        }
        if !self.size.0.is_finite()
            || !self.size.1.is_finite()
            || self.size.0 <= 0.0
            || self.size.1 <= 0.0
        {
            return Err(format!(
                "entity {:?}: size must be finite and positive",
                self.name
            ));
        }
        if self.behavior.trim().is_empty() {
            return Err(format!("entity {:?}: missing behavior name", self.name));
        }
        Ok(())
    }
}

/// Validated asset definitions, shared by all room instances.
#[derive(Debug)]
pub struct EnemyDefinitions {
    entities: BTreeMap<String, EnemyDefinition>,
    behaviors: BTreeMap<String, Arc<BehaviorDefinition>>,
}

impl EnemyDefinitions {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let behaviors = load_behaviors(&assets.join("behaviors"))?;
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
        })
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
        Enemy::from_definition(
            entity,
            self.behaviors[&entity.behavior].clone(),
            left,
            right,
            bounds,
        )
    }
}

/// A floor-bound enemy. Patrol endpoints and position are rectangle centers.
#[derive(Clone, Debug)]
pub struct Enemy {
    position: Vec2,
    patrol_left: f32,
    patrol_right: f32,
    name: String,
    size: Vec2,
    max_health: u8,
    contact_damage: u8,
    behavior: Behavior,
    health: u8,
    last_attack_id: Option<u64>,
    knockback_direction: f32,
    knockback_remaining: f32,
}

impl Enemy {
    pub fn new(
        left: f32,
        right: f32,
        speed: f32,
        detection_radius: f32,
        bounds: WorldBounds,
    ) -> Self {
        assert!(speed.is_finite() && speed >= 0.0);
        assert!(detection_radius.is_finite() && detection_radius >= 0.0);
        // Compatibility constructor for programmatic spawns. It uses the same
        // state machine as the file-backed chase definition, with custom tuning.
        let mut definition =
            BehaviorDefinition::from_ron(include_str!("../assets/behaviors/chase.ron"))
                .expect("built-in chase behavior");
        for state in &mut definition.states {
            for action in &mut state.actions {
                match action {
                    Action::Patrol { speed: value } | Action::MoveTowardPlayer { speed: value } => {
                        *value = speed
                    }
                    _ => {}
                }
            }
            for transition in &mut state.transitions {
                if let Condition::PlayerDistanceLessThan(value) = &mut transition.condition {
                    *value = detection_radius;
                }
            }
        }
        Self::from_definition(
            &EnemyDefinition {
                name: "chase".into(),
                health: ENEMY_MAX_HEALTH,
                size: (ENEMY_SIZE.x, ENEMY_SIZE.y),
                contact_damage: 1,
                behavior: "chase".into(),
            },
            Arc::new(definition),
            left,
            right,
            bounds,
        )
        .expect("valid programmatic enemy")
    }

    pub fn from_definition(
        definition: &EnemyDefinition,
        behavior: Arc<BehaviorDefinition>,
        left: f32,
        right: f32,
        bounds: WorldBounds,
    ) -> Result<Self, String> {
        definition.validate()?;
        let size = Vec2::new(definition.size.0, definition.size.1);
        if !left.is_finite()
            || !right.is_finite()
            || left > right
            || !bounds.left.is_finite()
            || !bounds.right.is_finite()
            || !bounds.floor_y.is_finite()
            || left < bounds.left + size.x / 2.0
            || right > bounds.right - size.x / 2.0
        {
            return Err(format!("entity {:?}: invalid patrol interval [{left}, {right}] for room bounds and entity size", definition.name));
        }
        Ok(Self {
            position: Vec2::new(left, bounds.floor_y - size.y / 2.0),
            patrol_left: left,
            patrol_right: right,
            name: definition.name.clone(),
            size,
            max_health: definition.health,
            contact_damage: definition.contact_damage,
            behavior: Behavior::new(behavior)?,
            health: definition.health,
            last_attack_id: None,
            knockback_direction: 0.0,
            knockback_remaining: 0.0,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn max_health(&self) -> u8 {
        self.max_health
    }

    pub fn contact_damage(&self) -> u8 {
        self.contact_damage
    }

    pub fn behavior(&self) -> &Behavior {
        &self.behavior
    }

    pub fn health(&self) -> u8 {
        self.health
    }

    pub fn is_dead(&self) -> bool {
        self.health == 0
    }

    pub fn hit(&mut self, attack_id: u64, direction: f32) -> bool {
        if self.is_dead()
            || self.last_attack_id == Some(attack_id)
            || !direction.is_finite()
            || direction == 0.0
        {
            return false;
        }
        self.last_attack_id = Some(attack_id);
        self.health -= 1;
        self.knockback_direction = direction.signum();
        self.knockback_remaining = KNOCKBACK_SECONDS;
        true
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn update(&mut self, player_position: Vec2, delta_seconds: f32, bounds: WorldBounds) {
        let dt = if delta_seconds.is_finite() {
            delta_seconds.max(0.0)
        } else {
            0.0
        };
        if self.is_dead() {
            return;
        }
        if self.knockback_remaining > 0.0 {
            let knockback_dt = dt.min(self.knockback_remaining);
            self.position.x = (self.position.x
                + self.knockback_direction * KNOCKBACK_SPEED * knockback_dt)
                .clamp(
                    bounds.left + self.size.x / 2.0,
                    bounds.right - self.size.x / 2.0,
                );
            self.knockback_remaining = (self.knockback_remaining - dt).max(0.0);
            self.position.y = bounds.floor_y - self.size.y / 2.0;
            return;
        }
        self.behavior.update(
            &mut self.position,
            BehaviorContext {
                player_position,
                health: self.health,
                size: self.size,
                bounds,
                patrol_left: self.patrol_left,
                patrol_right: self.patrol_right,
            },
            dt,
        );
    }

    pub fn touches(&self, position: Vec2, size: Vec2) -> bool {
        let separation = (self.position - position).abs();
        let reach = (self.size + size) / 2.0;
        !self.is_dead() && separation.x <= reach.x && separation.y <= reach.y
    }
}
