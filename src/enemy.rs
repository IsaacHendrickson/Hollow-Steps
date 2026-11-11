use std::sync::Arc;

use macroquad::math::Vec2;

use crate::behavior::{
    Action, Behavior, BehaviorContext, BehaviorDefinition, Condition, State, Transition,
};
use crate::body::{BodyInstance, BodyTemplates};
use crate::player::WorldBounds;
use crate::terrain::Terrain;
use crate::tuning::EnemyTuning;

/// The shipped values, kept so existing tests and callers still compile. The game reads
/// `EnemyTuning` (from `data/player.ron`); a test keeps these equal to that file.
pub const ENEMY_MAX_HEALTH: u16 = 30;
pub const KNOCKBACK_SPEED: f32 = 260.0;
pub const KNOCKBACK_SECONDS: f32 = 0.16;

pub const ENEMY_SIZE: Vec2 = Vec2::new(36.0, 36.0);

mod catalog;
mod death;
mod definition;
mod flap;
mod movement;

pub use catalog::EnemyDefinitions;
pub use death::DeathTimer;
pub use definition::{EnemyDefinition, DEFAULT_DEATH_SECS, MAX_DEATH_SECS};

/// A behavior-driven enemy. Patrol endpoints and position are rectangle centers.
#[derive(Clone, Debug)]
pub struct Enemy {
    position: Vec2,
    patrol_left: f32,
    patrol_right: f32,
    name: String,
    /// The hitbox size.
    size: Vec2,
    /// The size the body is drawn at (the hitbox size unless the definition sets one).
    visual_size: Vec2,
    body: Option<BodyInstance>,
    poses: Arc<crate::pose::Poses>,
    facing_left: bool,
    max_health: u16,
    contact_damage: u16,
    behavior: Behavior,
    health: u16,
    last_attack_id: Option<u64>,
    knockback_direction: f32,
    knockback_remaining: f32,
    tuning: EnemyTuning,
    /// Runs only once the enemy's health has run out.
    death: DeathTimer,
    /// Ground (floor-pinned) or flying at a hover height; set by the definition.
    movement: Movement,
    /// Seconds this enemy has been flying, which drives the bob. Stands still while it is dead.
    flight_secs: f64,
    /// Where the wings are in their beat (a flier with a `flap` only).
    flap_clock: FlapClock,
    /// Which flap pose the body was last given (`true` = up), `None` when none is in place.
    wing_pose: Option<bool>,
    /// A pose from the behavior (a wind-up, a recoil) is playing, so the wings leave the body
    /// alone until the behavior clears it.
    behavior_pose_held: bool,
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
                health: EnemyTuning::default().default_health,
                size: (ENEMY_SIZE.x, ENEMY_SIZE.y),
                visual_size: None,
                contact_damage: 10,
                behavior: "chase".into(),
                body: None,
                // Code-made enemies (summons, spawns) have no body to fade: gone at once.
                death_secs: 0.0,
                movement: Default::default(),
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
        // A flier starts on its hover line; everything else on the floor.
        let start_y = definition
            .movement
            .altitude(size, bounds, 0.0)
            .unwrap_or(bounds.floor_y - size.y / 2.0);
        let mut behavior = Behavior::new(behavior)?;
        behavior.set_flying(definition.movement.is_flying());
        Ok(Self {
            position: Vec2::new(left, start_y),
            patrol_left: left,
            patrol_right: right,
            name: definition.name.clone(),
            size,
            visual_size: crate::visual_box::size(size, definition.visual_size),
            body,
            poses: Arc::default(),
            facing_left: false,
            max_health: definition.health,
            contact_damage: definition.contact_damage,
            behavior,
            health: definition.health,
            last_attack_id: None,
            knockback_direction: 0.0,
            knockback_remaining: 0.0,
            tuning: EnemyTuning::default(),
            death: DeathTimer::new(definition.death_secs),
            movement: definition.movement.clone(),
            flight_secs: 0.0,
            flap_clock: FlapClock::default(),
            wing_pose: None,
            behavior_pose_held: false,
        })
    }

    pub fn movement(&self) -> &Movement {
        &self.movement
    }

    /// For a flier with a `flap`: beats the wings at the rate its horizontal speed asks for and
    /// puts the matching pose on the body when the stroke changes. A pose the behavior is
    /// playing (wind-up, recoil) is never overridden, and the hurt pose still wins while it
    /// lasts because the body remembers this one for afterwards.
    fn beat_wings(&mut self, horizontal_speed: f32, dt: f32) {
        let Movement::Flying {
            flap: Some(flap), ..
        } = &self.movement
        else {
            return;
        };
        self.flap_clock.advance(flap, horizontal_speed, dt);
        let up = self.flap_clock.wings_up();
        if self.behavior_pose_held || self.wing_pose == Some(up) {
            return;
        }
        if let Some(body) = &mut self.body {
            let name = if up { &flap.up_pose } else { &flap.down_pose };
            body.set_pose(Some(name), &self.poses);
            self.wing_pose = Some(up);
        }
    }

    /// For a flier: advance its flight clock and put it back on its hover line, inside the room.
    /// Ground enemies are left exactly as the behavior and terrain left them.
    fn hold_altitude(&mut self, dt: f32, bounds: WorldBounds) {
        if !self.movement.is_flying() {
            return;
        }
        self.flight_secs += f64::from(dt);
        self.position.x = Movement::clamp_x(self.position.x, self.size, bounds);
        if let Some(y) = self.movement.altitude(self.size, bounds, self.flight_secs) {
            self.position.y = y;
        }
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

    pub fn max_health(&self) -> u16 {
        self.max_health
    }

    pub fn contact_damage(&self) -> u16 {
        self.contact_damage
    }

    pub(crate) fn take_spawn_positions(&mut self) -> Vec<Vec2> {
        self.behavior.take_spawn_positions()
    }

    pub fn behavior(&self) -> &Behavior {
        &self.behavior
    }

    pub fn health(&self) -> u16 {
        self.health
    }

    /// Health has run out. A dead enemy is inert: it does not act, deal damage, or get hit.
    /// It stays in the room, fading, until `is_removable`.
    pub fn is_dead(&self) -> bool {
        self.health == 0
    }

    /// Dead but still fading out.
    pub fn is_dying(&self) -> bool {
        self.is_dead() && !self.death.finished()
    }

    /// Dead and done fading: time for the room to drop it.
    pub fn is_removable(&self) -> bool {
        self.is_dead() && self.death.finished()
    }

    /// Drawing opacity: `1` while alive, fading to `0` as the death plays out.
    pub fn opacity(&self) -> f32 {
        if self.is_dead() {
            self.death.opacity()
        } else {
            1.0
        }
    }

    pub fn tuning(&self) -> EnemyTuning {
        self.tuning
    }

    /// Applies the current knockback tunables. Safe to call every tick; a knockback already
    /// under way keeps its remaining time.
    pub fn set_tuning(&mut self, tuning: EnemyTuning) {
        self.tuning = tuning;
    }

    /// A freshly spawned code-made enemy (summon, spawn action) with the tunable default
    /// health and knockback. Enemies built from definitions keep their authored health.
    pub fn spawned_with(mut self, tuning: EnemyTuning) -> Self {
        self.tuning = tuning;
        self.health = tuning.default_health;
        self.max_health = tuning.default_health;
        self
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
        self.health = self.health.saturating_sub(crate::combat::SWORD_DAMAGE);
        if let Some(body) = &mut self.body {
            body.hurt(&self.poses);
        }
        self.knockback_direction = direction.signum();
        self.knockback_remaining = self.tuning.knockback_seconds;
        true
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    /// The hitbox size: collision, contact damage and attack reach.
    pub fn size(&self) -> Vec2 {
        self.size
    }

    /// The size the enemy is drawn at; the hitbox size unless its definition sets one.
    pub fn visual_size(&self) -> Vec2 {
        self.visual_size
    }

    /// The center of the drawn box: the hitbox's horizontal center and bottom edge.
    pub fn visual_center(&self) -> Vec2 {
        crate::visual_box::center(self.position, self.size, self.visual_size)
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
        self.update_with_terrain(
            player_position,
            player_size,
            delta_seconds,
            bounds,
            &Terrain::default(),
        );
    }

    /// Like `update_with_player_size`, but whatever movement the behavior
    /// produced (patrol, chase, lunge, jump, knockback, gravity) is swept
    /// against `terrain`, so enemies cannot pass through platforms or walls.
    pub fn update_with_terrain(
        &mut self,
        player_position: Vec2,
        player_size: Vec2,
        delta_seconds: f32,
        bounds: WorldBounds,
        terrain: &Terrain,
    ) {
        if self.is_dead() {
            self.step_dying(delta_seconds);
            return;
        }
        let previous = self.position;
        self.step_behavior(player_position, player_size, delta_seconds, bounds);
        // An enemy already inside a solid (authored overlap) is left alone
        // rather than being pushed out unpredictably.
        if terrain.is_empty()
            || self.movement.is_flying()
            || self.position == previous
            || terrain.overlaps_body(previous, self.size)
        {
            return;
        }
        let mut position = previous;
        terrain.move_body(
            &mut position,
            self.size,
            self.position - previous,
            crate::terrain::Limits {
                left: bounds.left,
                right: bounds.right,
                floor_y: bounds.floor_y,
            },
        );
        self.position = position;
    }

    /// A defeated enemy runs no behavior and does not move or animate: only its death clock
    /// advances, so the body keeps the pose and flash of the killing blow while it fades.
    fn step_dying(&mut self, delta_seconds: f32) {
        self.death.advance(delta_seconds);
    }

    fn step_behavior(
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
                + self.knockback_direction * self.tuning.knockback_speed * knockback_dt)
                .clamp(
                    bounds.left + self.size.x / 2.0,
                    bounds.right - self.size.x / 2.0,
                );
            self.knockback_remaining = (self.knockback_remaining - dt).max(0.0);
            self.hold_altitude(dt, bounds);
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
                    self.behavior_pose_held = name.is_some();
                    self.wing_pose = None;
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
                    self.behavior_pose_held = false;
                    self.wing_pose = None;
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
        self.hold_altitude(dt, bounds);
        if dt > 0.0 {
            self.beat_wings((self.position.x - previous_x).abs() / dt, dt);
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

    pub fn action_damage(&self, position: Vec2, size: Vec2) -> u16 {
        if self.is_dead() {
            0
        } else {
            self.behavior.blast_damage(position, size).max(
                self.attack_hit(position, size)
                    .map_or(0, |hit| hit.damage.clamp(0, i32::from(u16::MAX)) as u16),
            )
        }
    }
}
