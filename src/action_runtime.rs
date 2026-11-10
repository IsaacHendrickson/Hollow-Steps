//! One action executor, independent of sequence transitions and encounter scheduling.
use crate::behavior::{Action, BehaviorContext, Blast};
use crate::enemy_attack::{spread_angles, Aim, AttackBox, AttackRuntime, DamageHit, Projectile};
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
            attack_runtime: None,
            projectiles: Vec::new(),
            projectile_hits: Vec::new(),
            pose_steps: Vec::new(),
        }
    }
}

impl ActionRuntime {
    pub fn step(
        &mut self,
        action: &Action,
        preview: &Action,
        actor: &mut impl ActionActor,
        remaining: f64,
    ) -> (f64, bool) {
        let mut position = actor.position();
        let context = actor.context();
        let player_size = actor.player_size();
        let motion = actor.motion();
        self.direction = motion.facing;
        self.vertical_velocity = motion.velocity.1;
        self.airborne = motion.airborne;
        let result = self.execute(
            action,
            preview,
            &mut position,
            context,
            player_size,
            remaining,
            actor,
        );
        actor.set_position(position);
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

    pub fn motion(&self) -> ActorMotion {
        ActorMotion {
            facing: self.direction,
            velocity: (0.0, self.vertical_velocity),
            airborne: self.airborne,
        }
    }

    fn execute(
        &mut self,
        action: &Action,
        preview: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        player_size: Vec2,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let action = action.clone();
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
        let pose_bindings = preview.pose_bindings();
        let (mut blend_secs, strike_multiplier) = preview.pose_timing();
        if let Action::Telegraph { seconds } = action {
            blend_secs = preview.pose_blend_seconds(seconds);
        }
        if !self.action_started && pose_bindings.is_some() {
            self.pose_steps.push(crate::pose::PoseStep::Face(
                context.player_position.x < position.x,
            ));
        }
        let mut lunge_gravity_seconds = None;
        let (used, finished) = match action {
            Action::RadialBlast {
                radius,
                damage,
                knockback,
                knockback_secs,
                active_secs,
                telegraph_secs,
                ..
            } => {
                self.airborne = true;
                let attack = AttackBox {
                    shape: crate::enemy_attack::AttackShape::Circle,
                    offset: (0.0, 0.0),
                    size: (radius * 2.0, radius * 2.0),
                    damage,
                    knockback: knockback.0,
                    knockback_secs,
                    active_secs,
                };
                self.attack_runtime
                    .get_or_insert_with(|| {
                        AttackRuntime::lunge(
                            *position,
                            context,
                            0.0,
                            active_secs,
                            attack,
                            0.0,
                            telegraph_secs,
                        )
                    })
                    .step(position, context, remaining)
            }
            Action::ShootProjectile {
                speed,
                damage,
                lifetime_secs,
                size,
                aim,
                count,
                spread_degrees,
                telegraph_secs,
                ..
            } => {
                self.airborne = true;
                let target = *self.jump_target.get_or_insert(context.player_position);
                let warning = AttackBox {
                    shape: crate::enemy_attack::AttackShape::Rect,
                    offset: (0.0, 0.0),
                    size: size.0,
                    damage: 0,
                    knockback: (0.0, 0.0),
                    knockback_secs: 0.0,
                    active_secs: 0.0,
                };
                let (used, finished) = self
                    .attack_runtime
                    .get_or_insert_with(|| {
                        AttackRuntime::lunge(
                            *position,
                            context,
                            0.0,
                            0.0,
                            warning,
                            0.0,
                            telegraph_secs,
                        )
                    })
                    .step(position, context, remaining);
                if finished {
                    let horizontal = if target.x < position.x {
                        -Vec2::X
                    } else {
                        Vec2::X
                    };
                    let direction = match aim {
                        Aim::Horizontal => horizontal,
                        Aim::AtPlayer => (target - *position).try_normalize().unwrap_or(horizontal),
                    };
                    for angle in spread_angles(count, spread_degrees) {
                        let radians = angle.to_radians();
                        let mut projectile = Projectile {
                            position: *position,
                            velocity: Vec2::new(
                                direction.x * radians.cos() - direction.y * radians.sin(),
                                direction.x * radians.sin() + direction.y * radians.cos(),
                            ) * speed,
                            size: Vec2::from(size.0),
                            damage,
                            remaining_secs: f64::from(lifetime_secs),
                        };
                        let (alive, hit) = projectile.step(
                            (remaining - used).max(0.0),
                            context.bounds,
                            context.player_position,
                            player_size,
                        );
                        self.projectile_hits.extend(hit);
                        if alive {
                            actor.emit(ActionOutput::Projectile(projectile.clone()));
                            self.projectiles.push(projectile);
                        }
                    }
                }
                (used, finished)
            }
            Action::ImpactSlam {
                speed,
                attack,
                recovery_secs,
                telegraph_secs,
                ..
            } => {
                self.airborne = true;
                self.attack_runtime
                    .get_or_insert_with(|| {
                        AttackRuntime::slam(
                            *position,
                            context,
                            speed,
                            attack,
                            recovery_secs,
                            telegraph_secs,
                        )
                    })
                    .step(position, context, remaining)
            }
            Action::JumpTo {
                dx,
                dy,
                air_time_secs,
                arc_height,
                attack,
                recovery_secs,
                telegraph_secs,
                ..
            } => {
                self.airborne = true;
                self.attack_runtime
                    .get_or_insert_with(|| {
                        AttackRuntime::jump(
                            *position,
                            context,
                            Vec2::new(dx, dy),
                            air_time_secs,
                            arc_height,
                            attack.0,
                            recovery_secs,
                            telegraph_secs,
                        )
                    })
                    .step(position, context, remaining)
            }
            Action::Lunge {
                speed,
                duration_secs,
                attack,
                recovery_secs,
                telegraph_secs,
                ..
            } => {
                self.airborne = false;
                if self.attack_runtime.is_none() {
                    self.vertical_velocity = 0.0;
                }
                let runtime = self.attack_runtime.get_or_insert_with(|| {
                    AttackRuntime::lunge(
                        *position,
                        context,
                        speed,
                        duration_secs,
                        attack,
                        recovery_secs,
                        telegraph_secs,
                    )
                });
                let result = runtime.step(position, context, remaining);
                lunge_gravity_seconds = Some(runtime.lunge_gravity_seconds(result.0));
                result
            }
            Action::Wait { seconds }
            | Action::Telegraph { seconds }
            | Action::Hover { seconds } => {
                if matches!(action, Action::Hover { .. }) {
                    self.airborne = true;
                }
                let duration = f64::from(seconds);
                let used = remaining.min((duration - self.action_elapsed).max(0.0));
                self.action_elapsed += used;
                (used, self.action_elapsed >= duration)
            }
            Action::MoveTowardPlayer { speed } => {
                let target = context.player_position.x.clamp(
                    context.bounds.left + context.size.x / 2.0,
                    context.bounds.right - context.size.x / 2.0,
                );
                self.move_to(position, target, speed, remaining)
            }
            Action::MoveAwayFromPlayer { speed } => {
                let difference = position.x - context.player_position.x;
                if difference != 0.0 {
                    self.direction = difference.signum();
                }
                let target = if self.direction > 0.0 {
                    context.bounds.right - context.size.x / 2.0
                } else {
                    context.bounds.left + context.size.x / 2.0
                };
                self.move_to(position, target, speed, remaining)
            }
            Action::Patrol { speed } => {
                self.patrol(position, context, f64::from(speed) * remaining);
                (remaining, false)
            }
            Action::JumpToPlayer { dx, dy, speed } => {
                self.airborne = true;
                let target = *self.jump_target.get_or_insert_with(|| {
                    Self::clamped_jump_target(
                        context.player_position,
                        context.size,
                        context.bounds,
                        dx,
                        dy,
                    )
                });
                Self::move_in_plane(position, target, speed, remaining)
            }
            Action::SlamDown { speed } => {
                self.airborne = true;
                Self::move_in_plane(
                    position,
                    Vec2::new(position.x, context.bounds.floor_y - context.size.y / 2.0),
                    speed,
                    remaining,
                )
            }
            Action::AreaBlast {
                count,
                radius,
                fuse_time,
                damage,
            } => {
                if !self.action_started {
                    for _ in 0..count {
                        let blast_position = actor.random_arena_position();
                        self.blasts.push(Blast {
                            position: blast_position,
                            radius,
                            damage,
                            exploded: false,
                        });
                    }
                }
                let duration = f64::from(fuse_time);
                let used = remaining.min((duration - self.action_elapsed).max(0.0));
                self.action_elapsed += used;
                let finished = self.action_elapsed >= duration;
                if finished {
                    for blast in &mut self.blasts {
                        blast.exploded = true;
                    }
                }
                (used, finished)
            }
            _ => unreachable!("validated enemy action"),
        };
        if let (Some(bindings), Some(runtime)) = (pose_bindings, &self.attack_runtime) {
            for (phase, seconds) in runtime.pose_spans(used) {
                self.pose_steps.push(crate::pose::PoseStep::Apply {
                    name: bindings[phase].clone(),
                    seconds,
                    blend_secs,
                    stiffness_multiplier: if phase == 1 { strike_multiplier } else { 1.0 },
                    instant: phase == 1 && seconds == 0.0 && finished,
                });
            }
            if finished {
                self.pose_steps.push(crate::pose::PoseStep::Clear);
            }
        } else if let (Action::Telegraph { .. }, Some(bindings)) =
            (&action, preview.pose_bindings())
        {
            self.pose_steps.push(crate::pose::PoseStep::Apply {
                name: bindings[0].clone(),
                seconds: used as f32,
                blend_secs,
                stiffness_multiplier: 1.0,
                instant: false,
            });
        } else {
            self.pose_steps
                .push(crate::pose::PoseStep::Idle(used as f32));
        }
        if self.airborne {
            // Authored jump/slam arcs and explicit hover retain their trajectories.
            self.vertical_velocity = 0.0;
        } else {
            let gravity_seconds = lunge_gravity_seconds.unwrap_or(used);
            self.apply_gravity(position, context, gravity_seconds);
        }
        self.action_started = true;
        (used, finished)
    }
    /// Shared vertical integration for ground movement, lunges, and idle time.
    pub(crate) fn apply_gravity(&mut self, position: &mut Vec2, context: BehaviorContext, dt: f64) {
        const GRAVITY: f64 = 1_400.0;
        if dt <= 0.0 {
            return;
        }
        self.vertical_velocity += GRAVITY * dt;
        let floor = f64::from(context.bounds.floor_y - context.size.y / 2.0);
        let y = f64::from(position.y) + self.vertical_velocity * dt;
        if y >= floor {
            position.y = floor as f32;
            self.vertical_velocity = 0.0;
        } else {
            position.y = y as f32;
        }
    }

    pub(crate) fn clamped_jump_target(
        player: Vec2,
        size: Vec2,
        bounds: WorldBounds,
        dx: f32,
        dy: f32,
    ) -> Vec2 {
        let half = size.as_dvec2() / 2.0;
        let target = player.as_dvec2() + macroquad::math::DVec2::new(f64::from(dx), f64::from(dy));
        Vec2::new(
            target.x.clamp(
                f64::from(bounds.left) + half.x,
                f64::from(bounds.right) - half.x,
            ) as f32,
            target.y.clamp(
                half.y.min(f64::from(bounds.floor_y) - half.y),
                f64::from(bounds.floor_y) - half.y,
            ) as f32,
        )
    }

    pub(crate) fn move_in_plane(
        position: &mut Vec2,
        target: Vec2,
        speed: f32,
        dt: f64,
    ) -> (f64, bool) {
        let difference = target.as_dvec2() - position.as_dvec2();
        let distance = difference.length();
        if distance == 0.0 {
            return (0.0, true);
        }
        if speed == 0.0 {
            return (dt, false);
        }
        let duration = distance / f64::from(speed);
        if dt >= duration {
            *position = target;
            (duration, true)
        } else {
            *position = (position.as_dvec2() + difference * (dt / duration)).as_vec2();
            (dt, false)
        }
    }

    fn move_to(&mut self, position: &mut Vec2, target: f32, speed: f32, dt: f64) -> (f64, bool) {
        let difference = f64::from(target) - f64::from(position.x);
        if difference == 0.0 {
            return (0.0, true);
        }
        self.direction = difference.signum() as f32;
        if speed == 0.0 {
            return (dt, false);
        }
        let duration = difference.abs() / f64::from(speed);
        if dt >= duration {
            position.x = target;
            (duration, true)
        } else {
            position.x =
                (f64::from(position.x) + difference.signum() * f64::from(speed) * dt) as f32;
            (dt, false)
        }
    }

    fn patrol(&mut self, position: &mut Vec2, context: BehaviorContext, mut travel: f64) {
        // Return from a chase or knockback without teleporting onto the interval.
        let nearest = position.x.clamp(context.patrol_left, context.patrol_right);
        let difference = f64::from(nearest) - f64::from(position.x);
        if difference != 0.0 {
            self.direction = difference.signum() as f32;
            let step = travel.min(difference.abs());
            position.x = (f64::from(position.x) + difference.signum() * step) as f32;
            travel -= step;
        }
        if position.x >= context.patrol_left && position.x <= context.patrol_right {
            let width = f64::from(context.patrol_right) - f64::from(context.patrol_left);
            if width > 0.0 {
                let offset = f64::from(position.x) - f64::from(context.patrol_left);
                let phase = if self.direction > 0.0 {
                    offset
                } else {
                    2.0 * width - offset
                };
                let phase = (phase + travel) % (2.0 * width);
                self.direction = if phase < width { 1.0 } else { -1.0 };
                position.x = (f64::from(context.patrol_left)
                    + if phase < width {
                        phase
                    } else {
                        2.0 * width - phase
                    }) as f32;
            }
        }
    }
}
