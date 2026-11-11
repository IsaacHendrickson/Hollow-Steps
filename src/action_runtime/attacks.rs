//! Actions driven by an `AttackRuntime`, which times the wind-up, strike and recovery:
//! `area_blast`, `shoot_projectile`, `slam_down`, `jump_to` and `lunge`.
use super::{ActionActor, ActionOutput, ActionRuntime};
use crate::behavior::{Action, BehaviorContext};
use crate::enemy_attack::{spread_angles, Aim, AttackBox, AttackRuntime, AttackShape, Projectile};
use macroquad::math::Vec2;

impl ActionRuntime {
    pub(super) fn radial_blast(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> (f64, bool) {
        let Action::RadialBlast {
            radius,
            damage,
            knockback,
            knockback_secs,
            active_secs,
            telegraph_secs,
            ..
        } = *action
        else {
            unreachable!("radial_blast runs only for area_blast")
        };
        self.airborne = true;
        let attack = AttackBox {
            shape: AttackShape::Circle,
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

    pub(super) fn shoot_projectile(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::ShootProjectile {
            speed,
            damage,
            lifetime_secs,
            size,
            aim,
            count,
            spread_degrees,
            telegraph_secs,
            ..
        } = *action
        else {
            unreachable!("shoot_projectile runs only for shoot_projectile")
        };
        let player_size = actor.player_size();
        self.airborne = true;
        let target = *self.jump_target.get_or_insert(context.player_position);
        let warning = AttackBox {
            shape: AttackShape::Rect,
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
                AttackRuntime::lunge(*position, context, 0.0, 0.0, warning, 0.0, telegraph_secs)
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

    pub(super) fn impact_slam(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> (f64, bool) {
        let Action::ImpactSlam {
            speed,
            attack,
            recovery_secs,
            telegraph_secs,
            ..
        } = *action
        else {
            unreachable!("impact_slam runs only for slam_down")
        };
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

    pub(super) fn jump_to(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> (f64, bool) {
        let Action::JumpTo {
            dx,
            dy,
            air_time_secs,
            arc_height,
            attack,
            recovery_secs,
            telegraph_secs,
            ..
        } = *action
        else {
            unreachable!("jump_to runs only for jump_to")
        };
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

    /// Returns the result and how many seconds of gravity the step still owes: a lunge fixes
    /// its own height while it runs, so only the time after it counts.
    pub(super) fn lunge(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> ((f64, bool), f64) {
        let Action::Lunge {
            speed,
            duration_secs,
            attack,
            recovery_secs,
            telegraph_secs,
            ..
        } = *action
        else {
            unreachable!("lunge runs only for lunge")
        };
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
        (result, runtime.lunge_gravity_seconds(result.0))
    }
}
