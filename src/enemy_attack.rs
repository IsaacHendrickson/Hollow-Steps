//! Shared, window-free enemy attack geometry and simulation.
use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum AttackShape {
    Rect,
    Circle,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttackBox {
    pub shape: AttackShape,
    pub offset: (f32, f32),
    /// Full width/height; circles use size.x as their diameter and require equal axes.
    pub size: (f32, f32),
    pub damage: i32,
    pub knockback: (f32, f32),
    pub active_secs: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageHit {
    pub damage: i32,
    pub knockback: Vec2,
}

impl AttackBox {
    pub fn validate(self) -> Result<(), String> {
        for (label, value) in [
            ("offset x", self.offset.0),
            ("offset y", self.offset.1),
            ("knockback x", self.knockback.0),
            ("knockback y", self.knockback.1),
        ] {
            if !value.is_finite() {
                return Err(format!("attack {label}: must be finite"));
            }
        }
        crate::behavior::positive(self.size.0).map_err(|e| format!("attack width: {e}"))?;
        crate::behavior::positive(self.size.1).map_err(|e| format!("attack height: {e}"))?;
        if self.shape == AttackShape::Circle && self.size.0 != self.size.1 {
            return Err("attack circle width and height must match".into());
        }
        if self.damage < 0 {
            return Err("attack damage must be nonnegative".into());
        }
        crate::behavior::nonnegative(self.active_secs)
            .map_err(|e| format!("attack active seconds: {e}"))
    }

    pub fn center(self, origin: Vec2) -> Vec2 {
        origin + Vec2::from(self.offset)
    }

    pub fn overlaps(self, origin: Vec2, position: Vec2, size: Vec2) -> bool {
        let center = self.center(origin);
        match self.shape {
            AttackShape::Rect => {
                let separation = (center.as_dvec2() - position.as_dvec2()).abs();
                let reach = (Vec2::from(self.size).as_dvec2() + size.as_dvec2()) / 2.0;
                separation.x <= reach.x && separation.y <= reach.y
            }
            AttackShape::Circle => {
                let nearest = center.clamp(position - size / 2.0, position + size / 2.0);
                center.as_dvec2().distance(nearest.as_dvec2()) <= f64::from(self.size.0) / 2.0
            }
        }
    }

    pub fn hit(self) -> DamageHit {
        DamageHit {
            damage: self.damage,
            knockback: Vec2::from(self.knockback),
        }
    }
}

/// Half-open interval: no damage before activation or at/after expiration.
pub fn active_window(elapsed: f64, start: f64, duration: f32) -> bool {
    elapsed >= start && elapsed < start + f64::from(duration)
}

impl std::fmt::Display for AttackBox {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
impl std::str::FromStr for AttackBox {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let attack: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        attack.validate()?;
        Ok(attack)
    }
}
pub(crate) fn valid_attack(attack: AttackBox) -> Result<(), String> {
    attack.validate()
}

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

    pub fn warning(&self) -> Option<AttackBox> {
        (self.elapsed < self.telegraph)
            .then_some(self.attack)
            .flatten()
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
        self.elapsed += used;
        let travel_time = (self.elapsed - self.telegraph).clamp(0.0, duration);
        *position = match self.motion {
            Motion::Lunge { velocity, .. } => {
                (self.start.as_dvec2() + velocity.as_dvec2() * travel_time).as_vec2()
            }
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
}

fn clamp_to_room(position: &mut Vec2, context: BehaviorContext) {
    position.x = position.x.clamp(
        context.bounds.left + context.size.x / 2.0,
        context.bounds.right - context.size.x / 2.0,
    );
    position.y = position.y.clamp(
        (context.size.y / 2.0).min(context.bounds.floor_y - context.size.y / 2.0),
        context.bounds.floor_y - context.size.y / 2.0,
    );
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct OptionalAttack(pub Option<AttackBox>);
impl std::fmt::Display for OptionalAttack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
impl std::str::FromStr for OptionalAttack {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        valid_optional_attack(value)?;
        Ok(value)
    }
}
pub(crate) fn valid_optional_attack(value: OptionalAttack) -> Result<(), String> {
    value.0.map_or(Ok(()), AttackBox::validate)
}
pub(crate) fn default_arc_height() -> f32 {
    #[derive(Deserialize)]
    struct Defaults {
        jump_arc_height: f32,
    }
    ron::from_str::<Defaults>(include_str!("../data/attack_motion_defaults.ron"))
        .expect("valid motion defaults")
        .jump_arc_height
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
