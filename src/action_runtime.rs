//! One action executor, independent of sequence transitions and encounter scheduling.
//!
//! This file holds the shared types and the dispatcher; each action's own behavior lives in a
//! sibling module: `attacks`, `movement`, `blasts` and `instant`, with `poses` keeping the
//! pose layer informed.
mod attacks;
mod blasts;
mod instant;
mod movement;
mod poses;

use crate::behavior::{Action, BehaviorContext, Blast};
use crate::enemy_attack::{AttackBox, AttackRuntime, DamageHit, Projectile};
use crate::player::WorldBounds;
use macroquad::math::Vec2;

/// Physical state exposed by an entity adapter; action geometry never uses render bodies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActorMotion {
    pub facing: f32,
    pub velocity: (f64, f64),
    pub airborne: bool,
}

/// Effects leave the executor through the owning entity's sink.
#[derive(Clone, Debug, PartialEq)]
pub enum ActionOutput {
    Damage(u32),
    DamageArea { origin: Vec2, area: AttackBox },
    Projectile(Projectile),
    Spawn(Vec2),
    Hold(Vec2),
    Glide { target: Vec2, speed: f32 },
}

/// Window-free view of either an enemy or a boss.
pub trait ActionActor {
    fn position(&self) -> Vec2;
    fn set_position(&mut self, position: Vec2);
    fn context(&self) -> BehaviorContext;
    fn player_size(&self) -> Vec2;
    fn motion(&self) -> ActorMotion;
    fn set_motion(&mut self, motion: ActorMotion);
    fn random_arena_position(&mut self) -> Vec2;
    fn emit(&mut self, output: ActionOutput);
    /// Legacy boss actions sample the attack's locked target and suspend ground gravity.
    fn ground_physics(&self) -> bool {
        true
    }
    fn legacy_target(&self) -> Option<Vec2> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct ActionRuntime {
    pub(crate) action_elapsed: f64,
    pub(crate) direction: f32,
    pub(crate) action_started: bool,
    pub(crate) jump_target: Option<Vec2>,
    pub(crate) airborne: bool,
    pub(crate) vertical_velocity: f64,
    pub(crate) blasts: Vec<Blast>,
    pub(crate) rings: Vec<Blast>,
    pub(crate) attack_runtime: Option<AttackRuntime>,
    pub(crate) projectiles: Vec<Projectile>,
    pub(crate) projectile_hits: Vec<DamageHit>,
    pub(crate) pose_steps: Vec<crate::pose::PoseStep>,
}

impl Default for ActionRuntime {
    fn default() -> Self {
        Self {
            action_elapsed: 0.0,
            direction: 1.0,
            action_started: false,
            jump_target: None,
            airborne: false,
            vertical_velocity: 0.0,
            blasts: Vec::new(),
            rings: Vec::new(),
            attack_runtime: None,
            projectiles: Vec::new(),
            projectile_hits: Vec::new(),
            pose_steps: Vec::new(),
        }
    }
}

impl ActionRuntime {
    pub fn advance_glide(position: &mut Vec2, target: Vec2, speed: f32, dt: f64) {
        let offset = target.as_dvec2() - position.as_dvec2();
        let distance = offset.length();
        if distance > 0.0 {
            *position = (position.as_dvec2()
                + offset * (f64::from(speed) * dt / distance).min(1.0))
            .as_vec2();
        }
    }

    pub fn advance_projectiles(
        &mut self,
        player: Vec2,
        player_size: Vec2,
        bounds: WorldBounds,
        dt: f32,
    ) {
        self.projectile_hits.clear();
        self.projectiles.retain_mut(|projectile| {
            let (alive, hit) = projectile.step(f64::from(dt), bounds, player, player_size);
            self.projectile_hits.extend(hit);
            alive
        });
    }

    pub fn attack_hit(&self, origin: Vec2, position: Vec2, size: Vec2) -> Option<DamageHit> {
        self.attack_runtime
            .as_ref()
            .and_then(|r| r.active)
            .filter(|attack| attack.overlaps(origin, position, size))
            .map(AttackBox::hit)
            .into_iter()
            .chain(self.projectile_hits.iter().copied())
            .max_by_key(|hit| hit.damage)
    }

    pub fn blast_damage(&self, position: Vec2, size: Vec2) -> u16 {
        self.blasts
            .iter()
            .filter(|blast| {
                let nearest = blast
                    .position
                    .clamp(position - size / 2.0, position + size / 2.0);
                blast.exploded
                    && (nearest.as_dvec2() - blast.position.as_dvec2()).length()
                        <= f64::from(blast.radius)
            })
            .map(|blast| blast.damage)
            .max()
            .unwrap_or(0)
    }

    pub fn step(
        &mut self,
        action: &Action,
        preview: &Action,
        actor: &mut impl ActionActor,
        remaining: f64,
    ) -> (f64, bool) {
        if !remaining.is_finite() || remaining < 0.0 {
            return (0.0, false);
        }
        let mut position = actor.position();
        let motion = actor.motion();
        self.direction = motion.facing;
        self.vertical_velocity = motion.velocity.1;
        self.airborne = motion.airborne;
        let result = self.execute(action, preview, &mut position, remaining, actor);
        actor.set_position(position);
        if action.uses_ground_motion() {
            actor.emit(ActionOutput::Hold(position));
        }
        actor.set_motion(ActorMotion {
            facing: self.direction,
            velocity: (0.0, self.vertical_velocity),
            airborne: self.airborne,
        });
        if let Some(area) = self.attack_runtime.as_ref().and_then(|r| r.active) {
            actor.emit(ActionOutput::DamageArea {
                origin: position,
                area,
            });
        }
        result
    }

    pub fn finish_action(&mut self) {
        self.attack_runtime = None;
        self.action_elapsed = 0.0;
        self.action_started = false;
        self.jump_target = None;
    }

    pub fn motion(&self) -> ActorMotion {
        ActorMotion {
            facing: self.direction,
            velocity: (0.0, self.vertical_velocity),
            airborne: self.airborne,
        }
    }

    /// Runs one action for up to `remaining` seconds: shared setup, the action's own function,
    /// then the pose and gravity bookkeeping every action gets.
    fn execute(
        &mut self,
        action: &Action,
        preview: &Action,
        position: &mut Vec2,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let context = actor.context();
        if matches!(
            action,
            Action::MoveTowardPlayer { .. }
                | Action::MoveAwayFromPlayer { .. }
                | Action::Patrol { .. }
                | Action::Wait { .. }
                | Action::Telegraph { .. }
        ) {
            self.airborne = false;
        }
        let cue = self.begin_poses(action, preview, *position, context.player_position.x);
        let mut lunge_gravity_seconds = None;
        let (used, finished) = match action {
            Action::RadialBlast { .. } => self.radial_blast(action, position, context, remaining),
            Action::ShootProjectile { .. } => {
                self.shoot_projectile(action, position, context, remaining, actor)
            }
            Action::ImpactSlam { .. } => self.impact_slam(action, position, context, remaining),
            Action::JumpTo { .. } => self.jump_to(action, position, context, remaining),
            Action::Lunge { .. } => {
                let (result, gravity_seconds) = self.lunge(action, position, context, remaining);
                lunge_gravity_seconds = Some(gravity_seconds);
                result
            }
            Action::Wait { seconds } | Action::Telegraph { seconds } => {
                self.idle_for(*seconds, remaining)
            }
            Action::Hover { .. } => self.hover(action, position, remaining, actor),
            Action::MoveTowardPlayer { .. } => {
                self.move_toward_player(action, position, context, remaining)
            }
            Action::MoveAwayFromPlayer { .. } => {
                self.move_away_from_player(action, position, context, remaining)
            }
            Action::Patrol { .. } => self.patrol(action, position, context, remaining),
            Action::JumpToPlayer { .. } => {
                self.jump_to_player(action, position, context, remaining, actor)
            }
            Action::SlamDown { .. } => self.slam_down(action, position, context, remaining, actor),
            Action::AreaBlast { .. } => self.legacy_area_blast(action, context, remaining, actor),
            Action::GlideToPlayer { .. } => {
                Self::glide_to_player(action, *position, context, actor)
            }
            Action::GlideToFloor { .. } => Self::glide_to_floor(action, context, actor),
            Action::Pose { .. } => self.pose_action(action),
            Action::DamageTarget { .. } => Self::damage_target(action, context, actor),
            Action::SpawnShockwave { .. } => {
                Self::spawn_shockwave(action, *position, context, actor)
            }
            Action::Summon { .. } => Self::summon(action, context, actor),
            Action::SpawnRings { .. } => self.spawn_rings(action, preview, actor),
            Action::ExplodeRings { .. } => self.explode_rings(action, context, actor),
        };
        self.record_poses(action, &cue, used, finished);
        if self.airborne {
            // Authored jump/slam arcs and explicit hover retain their trajectories.
            self.vertical_velocity = 0.0;
        } else if actor.ground_physics() {
            let gravity_seconds = lunge_gravity_seconds.unwrap_or(used);
            self.apply_gravity(position, context, gravity_seconds);
        }
        self.action_started = true;
        (used, finished)
    }
}

impl Action {
    pub(crate) fn uses_ground_motion(&self) -> bool {
        matches!(
            self,
            Self::Lunge { .. }
                | Self::JumpTo { .. }
                | Self::ImpactSlam { .. }
                | Self::RadialBlast { .. }
                | Self::ShootProjectile { .. }
                | Self::MoveTowardPlayer { .. }
                | Self::MoveAwayFromPlayer { .. }
                | Self::Patrol { .. }
        )
    }
}
