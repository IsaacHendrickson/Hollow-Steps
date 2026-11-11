//! Waiting, walking, patrolling, hovering and the jump and slam toward the player, plus the
//! movement helpers (`apply_gravity`, `move_in_plane`, ...) the other action modules share.
use super::{ActionActor, ActionOutput, ActionRuntime};
use crate::behavior::{Action, BehaviorContext};
use crate::player::WorldBounds;
use macroquad::math::Vec2;

impl ActionRuntime {
    /// `wait` and `telegraph`: stand still for `seconds` of action time.
    pub(super) fn idle_for(&mut self, seconds: f32, remaining: f64) -> (f64, bool) {
        let duration = f64::from(seconds);
        let used = remaining.min((duration - self.action_elapsed).max(0.0));
        self.action_elapsed += used;
        (used, self.action_elapsed >= duration)
    }

    pub(super) fn hover(
        &mut self,
        action: &Action,
        position: &Vec2,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::Hover { seconds } = *action else {
            unreachable!("hover runs only for hover")
        };
        self.airborne = true;
        actor.emit(ActionOutput::Hold(*position));
        self.idle_for(seconds, remaining)
    }

    pub(super) fn move_toward_player(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> (f64, bool) {
        let Action::MoveTowardPlayer { speed } = *action else {
            unreachable!("move_toward_player runs only for move_toward_player")
        };
        let target = context.player_position.x.clamp(
            context.bounds.left + context.size.x / 2.0,
            context.bounds.right - context.size.x / 2.0,
        );
        self.move_to(position, target, speed, remaining)
    }

    pub(super) fn move_away_from_player(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> (f64, bool) {
        let Action::MoveAwayFromPlayer { speed } = *action else {
            unreachable!("move_away_from_player runs only for move_away_from_player")
        };
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

    pub(super) fn patrol(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
    ) -> (f64, bool) {
        let Action::Patrol { speed } = *action else {
            unreachable!("patrol runs only for patrol")
        };
        self.patrol_travel(position, context, f64::from(speed) * remaining);
        (remaining, false)
    }

    /// `jump_to_player`: fly to a point near the player, clamped into the arena.
    pub(super) fn jump_to_player(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::JumpToPlayer { speed, dx, dy } = *action else {
            unreachable!("jump_to_player runs only for jump_to_player")
        };
        self.airborne = true;
        let target = *self.jump_target.get_or_insert_with(|| {
            Self::clamped_jump_target(
                actor.legacy_target().unwrap_or(context.player_position),
                context.size,
                context.bounds,
                dx,
                dy,
            )
        });
        self.fly_to(target, speed, position, remaining, actor)
    }

    /// `legacy_slam_down`: drop to the floor under where the slam started.
    pub(super) fn slam_down(
        &mut self,
        action: &Action,
        position: &mut Vec2,
        context: BehaviorContext,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::SlamDown { speed } = *action else {
            unreachable!("slam_down runs only for legacy_slam_down")
        };
        self.airborne = true;
        if actor.legacy_target().is_none() {
            self.jump_target = None;
        }
        let target = *self.jump_target.get_or_insert_with(|| {
            Vec2::new(position.x, context.bounds.floor_y - context.size.y / 2.0)
        });
        self.fly_to(target, speed, position, remaining, actor)
    }

    /// The straight flight both the jump and the slam end in; a boss is told where to glide.
    fn fly_to(
        &mut self,
        target: Vec2,
        speed: f32,
        position: &mut Vec2,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        if !self.action_started && actor.legacy_target().is_some() {
            actor.emit(ActionOutput::Glide { target, speed });
        }
        Self::move_in_plane(position, target, speed, remaining)
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

    fn patrol_travel(&mut self, position: &mut Vec2, context: BehaviorContext, mut travel: f64) {
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
