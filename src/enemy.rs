use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::behavior::{
    load_behaviors, read_ron, ron_files, Action, Behavior, BehaviorContext, BehaviorDefinition,
    Condition, State, Transition,
};
use crate::body::{load_body_templates, BodyInstance, BodyTemplates, BodyVisual};
use crate::player::WorldBounds;

pub const ENEMY_MAX_HEALTH: u8 = 3;
pub const KNOCKBACK_SPEED: f32 = 260.0;
pub const KNOCKBACK_SECONDS: f32 = 0.16;

pub const ENEMY_SIZE: Vec2 = Vec2::new(36.0, 36.0);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub name: String,
    pub health: u8,
    pub size: (f32, f32),
    pub contact_damage: u8,
    pub behavior: String,
    #[serde(default)]
    pub body: Option<BodyVisual>,
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
        if let Some(body) = &self.body {
            body.validate()
                .map_err(|e| format!("entity {:?}: {e}", self.name))?;
        }
        Ok(())
    }
}

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

/// A behavior-driven enemy. Patrol endpoints and position are rectangle centers.
#[derive(Clone, Debug)]
pub struct Enemy {
    position: Vec2,
    patrol_left: f32,
    patrol_right: f32,
    name: String,
    size: Vec2,
    body: Option<BodyInstance>,
    poses: Arc<crate::pose::Poses>,
    facing_left: bool,
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
        // Programmatic spawns have fixed rules; editable file-backed enemies
        // still use EnemyDefinitions. Keep this constructor independent of assets.
        let definition = BehaviorDefinition {
            initial_state: "patrol".into(),
            states: vec![
                State {
                    name: "patrol".into(),
                    actions: vec![Action::Patrol { speed }],
                    transitions: vec![Transition {
                        target: "chase".into(),
                        condition: Condition::PlayerDistanceLessThan(detection_radius),
                        invert: false,
                    }],
                },
                State {
                    name: "chase".into(),
                    actions: vec![Action::MoveTowardPlayer { speed }],
                    transitions: vec![
                        Transition {
                            target: "patrol".into(),
                            condition: Condition::PlayerDistanceLessThan(detection_radius),
                            invert: true,
                        },
                        Transition {
                            target: "chase".into(),
                            condition: Condition::ActionFinished,
                            invert: false,
                        },
                    ],
                },
            ],
        };
        Self::from_definition(
            &EnemyDefinition {
                name: "chase".into(),
                health: ENEMY_MAX_HEALTH,
                size: (ENEMY_SIZE.x, ENEMY_SIZE.y),
                contact_damage: 1,
                behavior: "chase".into(),
                body: None,
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
        Self::from_definition_with_templates(
            definition,
            behavior,
            &BodyTemplates::new(),
            left,
            right,
            bounds,
        )
    }

    fn from_definition_with_templates(
        definition: &EnemyDefinition,
        behavior: Arc<BehaviorDefinition>,
        templates: &BodyTemplates,
        left: f32,
        right: f32,
        bounds: WorldBounds,
    ) -> Result<Self, String> {
        definition.validate()?;
        let body = definition
            .body
            .clone()
            .map(|visual| BodyInstance::new(visual, templates))
            .transpose()?;
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
            body,
            poses: Arc::default(),
            facing_left: false,
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

    pub fn body(&self) -> Option<&BodyInstance> {
        self.body.as_ref()
    }

    pub fn body_mut(&mut self) -> Option<&mut BodyInstance> {
        self.body.as_mut()
    }

    pub(crate) fn placement_key(&self) -> (&str, f32, f32) {
        (&self.name, self.patrol_left, self.patrol_right)
    }

    pub(crate) fn preserve_body_motion_from(&mut self, previous: &Self) {
        if let (Some(body), Some(old_body)) = (&mut self.body, &previous.body) {
            body.preserve_motion_from(old_body);
        }
    }

    pub fn facing_left(&self) -> bool {
        self.facing_left
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
        if let Some(body) = &mut self.body {
            body.hurt(&self.poses);
        }
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
        self.update_with_player_size(player_position, Vec2::ZERO, delta_seconds, bounds);
    }

    pub fn update_with_player_size(
        &mut self,
        player_position: Vec2,
        player_size: Vec2,
        delta_seconds: f32,
        bounds: WorldBounds,
    ) {
        self.behavior.clear_blast_hits();
        let dt = if delta_seconds.is_finite() {
            delta_seconds.max(0.0)
        } else {
            0.0
        };
        if self.is_dead() {
            return;
        }
        if self.knockback_remaining > 0.0 {
            if let Some(body) = &mut self.body {
                body.step(dt);
            }
            self.behavior.pause_attack();
            self.behavior
                .advance_projectiles(player_position, player_size, bounds, dt);
            let knockback_dt = dt.min(self.knockback_remaining);
            self.position.x = (self.position.x
                + self.knockback_direction * KNOCKBACK_SPEED * knockback_dt)
                .clamp(
                    bounds.left + self.size.x / 2.0,
                    bounds.right - self.size.x / 2.0,
                );
            self.knockback_remaining = (self.knockback_remaining - dt).max(0.0);
            return;
        }
        let previous_x = self.position.x;
        self.behavior.update_with_player_size(
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
            player_size,
        );
        for step in self.behavior.take_pose_steps() {
            match step {
                crate::pose::PoseStep::Face(left) => self.facing_left = left,
                crate::pose::PoseStep::Apply {
                    name,
                    seconds,
                    blend_secs,
                    stiffness_multiplier,
                    instant,
                } => {
                    if let Some(body) = &mut self.body {
                        body.set_pose(name.as_deref(), &self.poses);
                        body.configure_pose_spring(blend_secs, stiffness_multiplier);
                        if instant {
                            body.instant_pose_impulse();
                        }
                        body.step(seconds);
                    }
                }
                crate::pose::PoseStep::Clear => {
                    if let Some(body) = &mut self.body {
                        body.clear_pose();
                    }
                }
                crate::pose::PoseStep::Idle(seconds) => {
                    if let Some(body) = &mut self.body {
                        body.step(seconds);
                    }
                }
            }
        }
        if self.position.x != previous_x {
            self.facing_left = self.position.x < previous_x;
        }
    }

    pub fn touches(&self, position: Vec2, size: Vec2) -> bool {
        let separation = (self.position - position).abs();
        let reach = (self.size + size) / 2.0;
        !self.is_dead() && separation.x <= reach.x && separation.y <= reach.y
    }

    pub fn attack_hit(&self, position: Vec2, size: Vec2) -> Option<crate::enemy_attack::DamageHit> {
        if self.is_dead() {
            None
        } else {
            self.behavior.attack_hit(self.position, position, size)
        }
    }

    pub fn action_damage(&self, position: Vec2, size: Vec2) -> u8 {
        if self.is_dead() {
            0
        } else {
            self.behavior.blast_damage(position, size).max(
                self.attack_hit(position, size)
                    .map_or(0, |hit| hit.damage.clamp(0, i32::from(u8::MAX)) as u8),
            )
        }
    }
}
