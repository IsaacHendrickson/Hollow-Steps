//! Timed enemy attacks: wind-up, travel, active window and recovery, plus the shared motion defaults.
use macroquad::math::Vec2;
use serde::Deserialize;

use super::attack_box::AttackBox;
use crate::behavior::BehaviorContext;

#[derive(Clone, Debug)]
enum Motion {
    Lunge {
        velocity: Vec2,
        duration: f64,
    },
    Jump {
        target: Vec2,
        duration: f64,
        height: f32,
    },
}

#[derive(Clone, Debug)]
pub struct AttackRuntime {
    motion: Motion,
    attack: Option<AttackBox>,
    telegraph: f64,
    recovery: f64,
    elapsed: f64,
    start: Vec2,
    pub active: Option<AttackBox>,
}

impl AttackRuntime {
    pub fn lunge(
        position: Vec2,
        context: BehaviorContext,
        speed: f32,
        duration: f32,
        attack: AttackBox,
        recovery: f32,
        telegraph: f32,
    ) -> Self {
        let direction = (context.player_position - position).normalize_or_zero();
        Self {
            motion: Motion::Lunge {
                velocity: direction * speed,
                duration: f64::from(duration),
            },
            attack: Some(attack),
            telegraph: f64::from(telegraph),
            recovery: f64::from(recovery),
            elapsed: 0.0,
            start: position,
            active: None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn jump(
        position: Vec2,
        context: BehaviorContext,
        offset: Vec2,
        air_time: f32,
        height: f32,
        attack: Option<AttackBox>,
        recovery: f32,
        telegraph: f32,
    ) -> Self {
        let mut target = (context.player_position.as_dvec2() + offset.as_dvec2()).as_vec2();
        clamp_to_room(&mut target, context);
        Self {
            motion: Motion::Jump {
                target,
                duration: f64::from(air_time),
                height,
            },
            attack,
            telegraph: f64::from(telegraph),
            recovery: f64::from(recovery),
            elapsed: 0.0,
            start: position,
            active: None,
        }
    }

    pub fn slam(
        position: Vec2,
        context: BehaviorContext,
        speed: f32,
        attack: AttackBox,
        recovery: f32,
        telegraph: f32,
    ) -> Self {
        let target = Vec2::new(position.x, context.bounds.floor_y - context.size.y / 2.0);
        Self {
            motion: Motion::Jump {
                target,
                duration: f64::from((target.y - position.y).max(0.0)) / f64::from(speed),
                height: 0.0,
            },
            attack: Some(attack),
            telegraph: f64::from(telegraph),
            recovery: f64::from(recovery),
            elapsed: 0.0,
            start: position,
            active: None,
        }
    }

    /// Presentation spans of the last simulation step, including exact phase boundaries.
    pub fn pose_spans(&self, used: f64) -> Vec<(usize, f32)> {
        let duration = match self.motion {
            Motion::Lunge { duration, .. } | Motion::Jump { duration, .. } => duration,
        };
        let landing = matches!(self.motion, Motion::Jump { .. });
        let active_end = self.telegraph
            + duration
            + if landing {
                self.attack.map_or(0.0, |a| f64::from(a.active_secs))
            } else {
                0.0
            };
        let before = self.elapsed - used;
        let mut spans = Vec::new();
        for (phase, start, end) in [
            (0, 0.0, self.telegraph),
            (1, self.telegraph, active_end),
            (2, active_end, active_end + self.recovery),
        ] {
            let seconds = self.elapsed.min(end) - before.max(start);
            if seconds > 0.0
                || (self.elapsed == start && end > start)
                || (phase == 1 && start == end && self.elapsed >= start && before <= start)
            {
                spans.push((phase, seconds.max(0.0) as f32));
            }
        }
        spans
    }

    pub fn warning(&self) -> Option<AttackBox> {
        (self.elapsed < self.telegraph)
            .then_some(self.attack)
            .flatten()
    }

    /// Portion of the last step after lunge travel, including recovery.
    pub(crate) fn lunge_gravity_seconds(&self, used: f64) -> f64 {
        let Motion::Lunge { duration, .. } = self.motion else {
            return 0.0;
        };
        let end = self.telegraph + duration;
        (self.elapsed - end).max(0.0) - (self.elapsed - used - end).max(0.0)
    }

    pub fn step(&mut self, position: &mut Vec2, context: BehaviorContext, dt: f64) -> (f64, bool) {
        let (duration, landing) = match self.motion {
            Motion::Lunge { duration, .. } => (duration, false),
            Motion::Jump { duration, .. } => (duration, true),
        };
        let active_duration = if landing {
            self.attack.map_or(0.0, |a| f64::from(a.active_secs))
        } else {
            duration
        };
        let active_start = self.telegraph + if landing { duration } else { 0.0 };
        let end =
            self.telegraph + duration + if landing { active_duration } else { 0.0 } + self.recovery;
        let used = dt.min((end - self.elapsed).max(0.0));
        let previous_travel = (self.elapsed - self.telegraph).clamp(0.0, duration);
        self.elapsed += used;
        let travel_time = (self.elapsed - self.telegraph).clamp(0.0, duration);
        *position = match self.motion {
            Motion::Lunge { velocity, .. } => (position.as_dvec2()
                + velocity.as_dvec2() * (travel_time - previous_travel))
                .as_vec2(),
            Motion::Jump { target, height, .. } => parabolic_position(
                self.start,
                target,
                if duration == 0.0 {
                    1.0
                } else {
                    travel_time / duration
                },
                height,
            ),
        };
        clamp_to_room(position, context);
        self.active =
            if self.elapsed >= active_start && self.elapsed < active_start + active_duration {
                self.attack
            } else {
                None
            };
        (used, self.elapsed >= end)
    }

    pub fn windup_progress(&self) -> Option<f32> {
        (self.elapsed < self.telegraph).then(|| (self.elapsed / self.telegraph) as f32)
    }
}

fn clamp_to_room(position: &mut Vec2, context: BehaviorContext) {
    position.x = position.x.clamp(
        context.bounds.left + context.size.x / 2.0,
        context.bounds.right - context.size.x / 2.0,
    );
    position.y = position.y.clamp(
        (context.bounds.top + context.size.y / 2.0)
            .min(context.bounds.floor_y - context.size.y / 2.0),
        context.bounds.floor_y - context.size.y / 2.0,
    );
}

#[derive(Deserialize)]
struct MotionDefaults {
    jump_arc_height: f32,
    knockback_secs: f32,
}
fn motion_defaults() -> MotionDefaults {
    ron::from_str(include_str!("../../data/attack_motion_defaults.ron"))
        .expect("valid motion defaults")
}
pub(crate) fn default_arc_height() -> f32 {
    motion_defaults().jump_arc_height
}
pub(crate) fn default_knockback_secs() -> f32 {
    motion_defaults().knockback_secs
}

/// A quadratic Bezier-equivalent parabola with exact endpoints; height is authored.
pub fn parabolic_position(start: Vec2, target: Vec2, progress: f64, height: f32) -> Vec2 {
    if progress >= 1.0 {
        return target;
    }
    if progress <= 0.0 {
        return start;
    }
    let linear = start.as_dvec2().lerp(target.as_dvec2(), progress);
    (linear - macroquad::math::DVec2::Y * (4.0 * f64::from(height) * progress * (1.0 - progress)))
        .as_vec2()
}
