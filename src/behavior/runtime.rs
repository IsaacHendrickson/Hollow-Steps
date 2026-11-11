//! A running behavior: one entity's state cursor, timers and action runtime, advanced by
//! `update_with_player_size`.
use super::{Action, BehaviorDefinition, ConditionContext};
use crate::enemy_attack::AttackFeedback;
use crate::enemy_attack::{default_feedback, AttackBox, AttackWarning, DamageHit, Projectile};
use crate::player::WorldBounds;
use macroquad::math::Vec2;
use std::sync::Arc;

/// Inputs supplied by the owning entity, rather than a renderer or global world.
#[derive(Clone, Copy, Debug)]
pub struct BehaviorContext {
    pub player_position: Vec2,
    pub health: u16,
    pub size: Vec2,
    pub bounds: WorldBounds,
    pub patrol_left: f32,
    pub patrol_right: f32,
}

/// Each instance owns its timers and action cursor; definitions may be shared.
#[derive(Clone, Debug)]
pub struct Behavior {
    definition: Arc<BehaviorDefinition>,
    state: usize,
    action: usize,
    state_elapsed: f64,
    runtime: crate::action_runtime::ActionRuntime,
    random_state: Option<u32>,
    glide: Option<(Vec2, f32)>,
    outputs: Vec<crate::action_runtime::ActionOutput>,
}

/// A stationary warning circle; `exploded` is a damage event for one update.
#[derive(Clone, Debug, PartialEq)]
pub struct Blast {
    pub position: Vec2,
    pub radius: f32,
    pub damage: u16,
    pub exploded: bool,
}

impl Blast {
    pub fn position(&self) -> Vec2 {
        self.position
    }
    pub fn radius(&self) -> f32 {
        self.radius
    }
    pub fn exploded(&self) -> bool {
        self.exploded
    }
}

impl Behavior {
    pub fn new(definition: Arc<BehaviorDefinition>) -> Result<Self, String> {
        definition.validate()?;
        let state = definition
            .states
            .iter()
            .position(|state| state.name == definition.initial_state)
            .expect("validated initial state");
        Ok(Self {
            definition,
            state,
            action: 0,
            state_elapsed: 0.0,
            runtime: crate::action_runtime::ActionRuntime::default(),
            random_state: None,
            glide: None,
            outputs: Vec::new(),
        })
    }

    pub fn take_pose_steps(&mut self) -> Vec<crate::pose::PoseStep> {
        std::mem::take(&mut self.runtime.pose_steps)
    }

    /// Marks the owner as flying, so the behavior never applies gravity to it.
    pub(crate) fn set_flying(&mut self, flying: bool) {
        self.runtime.flying = flying;
    }

    pub fn set_random_seed(&mut self, seed: u32) {
        self.random_state = Some(seed);
    }

    pub fn state_name(&self) -> &str {
        &self.definition.states[self.state].name
    }

    pub fn state_elapsed(&self) -> f64 {
        self.state_elapsed
    }

    pub fn action_finished(&self) -> bool {
        self.action == self.definition.states[self.state].actions.len()
    }

    pub fn rings(&self) -> &[Blast] {
        &self.runtime.rings
    }

    pub(crate) fn take_spawn_positions(&mut self) -> Vec<Vec2> {
        let mut positions = Vec::new();
        self.outputs.retain(|output| {
            if let crate::action_runtime::ActionOutput::Spawn(position) = output {
                positions.push(*position);
                false
            } else {
                true
            }
        });
        positions
    }

    pub fn blasts(&self) -> &[Blast] {
        &self.runtime.blasts
    }

    pub fn blast_damage(&self, position: Vec2, size: Vec2) -> u16 {
        let immediate = self
            .outputs
            .iter()
            .filter_map(|output| match output {
                crate::action_runtime::ActionOutput::Damage(damage) => {
                    Some((*damage).min(u32::from(u16::MAX)) as u16)
                }
                _ => None,
            })
            .max()
            .unwrap_or(0);
        self.runtime.blast_damage(position, size).max(immediate)
    }

    pub(crate) fn clear_blast_hits(&mut self) {
        self.outputs.clear();
        self.runtime.blasts.retain(|blast| !blast.exploded);
    }

    /// Current damage geometry, independent of collision targets.
    pub fn active_damage_area(&self) -> Option<AttackBox> {
        self.runtime
            .attack_runtime
            .as_ref()
            .and_then(|runtime| runtime.active)
    }

    pub fn attack_hit(&self, origin: Vec2, position: Vec2, size: Vec2) -> Option<DamageHit> {
        self.runtime.attack_hit(origin, position, size)
    }

    pub(crate) fn advance_projectiles(
        &mut self,
        player: Vec2,
        player_size: Vec2,
        bounds: WorldBounds,
        dt: f32,
    ) {
        self.runtime
            .advance_projectiles(player, player_size, bounds, dt);
    }

    pub(crate) fn pause_attack(&mut self) {
        if let Some(runtime) = &mut self.runtime.attack_runtime {
            runtime.active = None;
        }
    }

    pub fn projectiles(&self) -> &[Projectile] {
        &self.runtime.projectiles
    }

    pub fn attack_warning(&self) -> Option<AttackBox> {
        self.runtime.attack_runtime.as_ref()?.warning()
    }

    pub fn projectile_feedback(&self) -> AttackFeedback {
        self.definition.states[self.state]
            .actions
            .get(self.action)
            .and_then(Action::feedback)
            .unwrap_or_else(default_feedback)
    }

    pub fn warning(&self, size: Vec2) -> Option<AttackWarning> {
        let actions = &self.definition.states[self.state].actions;
        let current = actions.get(self.action)?;
        let (preview, progress) = if let Action::Telegraph { seconds } = current {
            if *seconds <= 0.0 {
                return None;
            }
            (
                actions.get(self.action + 1).unwrap_or(current),
                (self.runtime.action_elapsed / f64::from(*seconds)) as f32,
            )
        } else {
            let progress = match &self.runtime.attack_runtime {
                Some(runtime) => runtime.windup_progress()?,
                None if current.windup_seconds()? > 0.0 => 0.0,
                None => return None,
            };
            (current, progress)
        };
        let feedback = preview.feedback().unwrap_or_else(default_feedback);
        let attack = preview.preview_attack().unwrap_or(AttackBox {
            shape: crate::enemy_attack::AttackShape::Rect,
            offset: (0.0, 0.0),
            size: size.into(),
            damage: 0,
            knockback: (0.0, 0.0),
            knockback_secs: 0.0,
            active_secs: 0.0,
        });
        Some(AttackWarning {
            attack,
            feedback,
            strength: (1.0 + (std::f32::consts::TAU * feedback.flash_cycles * progress).cos())
                / 2.0,
        })
    }

    pub fn is_telegraphing(&self) -> bool {
        matches!(
            self.definition.states[self.state].actions.get(self.action),
            Some(Action::Telegraph { .. })
        ) || self.warning(Vec2::ONE).is_some()
    }

    fn transition(&mut self, position: Vec2, context: BehaviorContext) -> bool {
        let difference = context.player_position.as_dvec2() - position.as_dvec2();
        let conditions = ConditionContext {
            state_elapsed: self.state_elapsed,
            player_distance: difference.length(),
            health: context.health,
            action_finished: self.action_finished(),
        };
        let target = self.definition.states[self.state]
            .transitions
            .iter()
            .find(|transition| transition.condition.matches(conditions) != transition.invert)
            .map(|transition| transition.target.as_str());
        let Some(target) = target else {
            return false;
        };
        self.runtime.pose_steps.push(crate::pose::PoseStep::Clear);
        self.state = self
            .definition
            .states
            .iter()
            .position(|state| state.name == target)
            .expect("validated transition");
        self.action = 0;
        self.runtime.action_elapsed = 0.0;
        self.state_elapsed = 0.0;
        self.runtime.action_started = false;
        self.runtime.jump_target = None;
        self.runtime.attack_runtime = None;
        // Interrupting a fuse cancels its pending attack; keep hits already emitted.
        self.runtime.blasts.retain(|blast| blast.exploded);
        true
    }

    /// Transitions are checked before and after the action sequence, with at
    /// most one transition per update (including self-transitions).
    pub fn update(&mut self, position: &mut Vec2, context: BehaviorContext, delta_seconds: f32) {
        self.update_with_player_size(position, context, delta_seconds, Vec2::ZERO);
    }

    pub fn update_with_player_size(
        &mut self,
        position: &mut Vec2,
        context: BehaviorContext,
        delta_seconds: f32,
        player_size: Vec2,
    ) {
        self.outputs.clear();
        self.runtime.pose_steps.clear();
        self.clear_blast_hits();
        self.runtime.projectile_hits.clear();
        if !delta_seconds.is_finite()
            || delta_seconds <= 0.0
            || !context.player_position.is_finite()
        {
            return;
        }
        self.advance_projectiles(
            context.player_position,
            player_size,
            context.bounds,
            delta_seconds,
        );
        let transitioned = self.transition(*position, context);
        let blocking = self.definition.states[self.state]
            .actions
            .get(self.action)
            .is_some_and(|action| {
                matches!(
                    action,
                    Action::JumpToPlayer { .. } | Action::SlamDown { .. } | Action::Hover { .. }
                ) || action.uses_ground_motion()
            });
        if !blocking {
            if let Some((target, speed)) = self.glide {
                crate::action_runtime::ActionRuntime::advance_glide(
                    position,
                    target,
                    speed,
                    f64::from(delta_seconds),
                );
            }
        }
        let dt = f64::from(delta_seconds);
        self.state_elapsed += dt;
        let mut remaining = dt;
        while let Some(action) = self.definition.states[self.state].actions.get(self.action) {
            let preview = if matches!(action, Action::SpawnRings { .. }) {
                self.definition.states[self.state].actions[self.action + 1..]
                    .iter()
                    .find(|a| matches!(a, Action::ExplodeRings { .. }))
                    .unwrap_or(action)
            } else if matches!(action, Action::Telegraph { .. }) {
                self.definition.states[self.state]
                    .actions
                    .get(self.action + 1)
                    .unwrap_or(action)
            } else {
                action
            };
            let output_start = self.outputs.len();
            let mut actor = EnemyActionActor {
                position,
                context,
                player_size,
                motion: self.runtime.motion(),
                random_state: &mut self.random_state,
                outputs: &mut self.outputs,
            };
            let (used, finished) = self.runtime.step(action, preview, &mut actor, remaining);
            for output in &self.outputs[output_start..] {
                match output {
                    crate::action_runtime::ActionOutput::Glide { target, speed } => {
                        self.glide = Some((*target, *speed))
                    }
                    crate::action_runtime::ActionOutput::Hold(_) => self.glide = None,
                    _ => {}
                }
            }
            remaining = (remaining - used).max(0.0);
            if !finished {
                break;
            }
            self.runtime.finish_action();
            self.action += 1;
        }
        self.runtime
            .pose_steps
            .push(crate::pose::PoseStep::Idle(remaining as f32));
        if !self.runtime.airborne {
            self.runtime.apply_gravity(position, context, remaining);
        }
        if !transitioned {
            self.transition(*position, context);
        }
    }
}

/// Enemy view used while Behavior owns the sequence cursor.
struct EnemyActionActor<'a> {
    position: &'a mut Vec2,
    context: BehaviorContext,
    player_size: Vec2,
    motion: crate::action_runtime::ActorMotion,
    random_state: &'a mut Option<u32>,
    outputs: &'a mut Vec<crate::action_runtime::ActionOutput>,
}
impl EnemyActionActor<'_> {
    fn next_random(&mut self) -> f64 {
        let state = self
            .random_state
            .get_or_insert(self.position.x.to_bits().rotate_left(16) ^ self.position.y.to_bits());
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(*state) / f64::from(u32::MAX)
    }
}
impl crate::action_runtime::ActionActor for EnemyActionActor<'_> {
    fn position(&self) -> Vec2 {
        *self.position
    }
    fn set_position(&mut self, position: Vec2) {
        *self.position = position;
    }
    fn context(&self) -> BehaviorContext {
        self.context
    }
    fn player_size(&self) -> Vec2 {
        self.player_size
    }
    fn motion(&self) -> crate::action_runtime::ActorMotion {
        self.motion
    }
    fn set_motion(&mut self, motion: crate::action_runtime::ActorMotion) {
        self.motion = motion;
    }
    fn random_arena_position(&mut self) -> Vec2 {
        let bounds = self.context.bounds;
        let x = f64::from(bounds.left)
            + self.next_random() * (f64::from(bounds.right) - f64::from(bounds.left));
        let y = self.next_random() * f64::from(bounds.floor_y);
        Vec2::new(x as f32, y as f32)
    }
    fn emit(&mut self, output: crate::action_runtime::ActionOutput) {
        self.outputs.push(output);
    }
}
