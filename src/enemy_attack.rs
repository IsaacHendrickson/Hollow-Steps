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
    /// Full width/height; circles use size.x as their diameter (height is retained for switching shapes).
    pub size: (f32, f32),
    pub damage: i32,
    pub knockback: (f32, f32),
    #[serde(default = "default_knockback_secs")]
    pub knockback_secs: f32,
    pub active_secs: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DamageHit {
    pub damage: i32,
    pub knockback: Vec2,
    pub knockback_secs: f32,
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
        if self.damage < 0 {
            return Err("attack damage must be nonnegative".into());
        }
        crate::behavior::nonnegative(self.knockback_secs)
            .map_err(|e| format!("attack knockback seconds: {e}"))?;
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
            knockback_secs: self.knockback_secs,
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
#[derive(Deserialize)]
struct MotionDefaults {
    jump_arc_height: f32,
    knockback_secs: f32,
}
fn motion_defaults() -> MotionDefaults {
    ron::from_str(include_str!("../data/attack_motion_defaults.ron"))
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

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Aim {
    AtPlayer,
    Horizontal,
}
impl std::fmt::Display for Aim {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::AtPlayer => "AtPlayer",
            Self::Horizontal => "Horizontal",
        })
    }
}
impl std::str::FromStr for Aim {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        match text {
            "AtPlayer" => Ok(Self::AtPlayer),
            "Horizontal" => Ok(Self::Horizontal),
            _ => Err("unknown aim mode: expected AtPlayer or Horizontal".into()),
        }
    }
}
pub(crate) fn valid_aim(_: Aim) -> Result<(), String> {
    Ok(())
}
pub(crate) fn projectile_count(count: u16) -> Result<(), String> {
    if count >= 1 {
        Ok(())
    } else {
        Err("count must be at least 1".into())
    }
}
pub(crate) fn attack_damage(damage: i32) -> Result<(), String> {
    if damage >= 0 {
        Ok(())
    } else {
        Err("damage must be nonnegative".into())
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct AttackSize(pub (f32, f32));
impl std::fmt::Display for AttackSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
impl std::str::FromStr for AttackSize {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        valid_size(value)?;
        Ok(value)
    }
}
pub(crate) fn valid_size(size: AttackSize) -> Result<(), String> {
    crate::behavior::positive(size.0 .0)?;
    crate::behavior::positive(size.0 .1)
}

/// Spread is the total fan angle, including both outer projectiles.
pub fn spread_angles(count: u16, spread_degrees: f32) -> Vec<f32> {
    (0..count)
        .map(|index| {
            if count == 1 {
                0.0
            } else {
                (f32::from(index) / f32::from(count - 1) - 0.5) * spread_degrees
            }
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub size: Vec2,
    pub damage: i32,
    pub remaining_secs: f64,
}
impl Projectile {
    /// Continuous collision against the room and player, limited to the live portion of the step.
    pub fn step(
        &mut self,
        dt: f64,
        bounds: crate::player::WorldBounds,
        player: Vec2,
        player_size: Vec2,
    ) -> (bool, Option<DamageHit>) {
        if !dt.is_finite() || dt < 0.0 {
            return (self.remaining_secs > 0.0, None);
        }
        let min = Vec2::new(bounds.left, bounds.top) + self.size / 2.0;
        let max = Vec2::new(bounds.right, bounds.floor_y) - self.size / 2.0;
        if self.position.x < min.x
            || self.position.x > max.x
            || self.position.y < min.y
            || self.position.y > max.y
            || self.remaining_secs <= 0.0
        {
            return (false, None);
        }
        let mut wall_time = f64::INFINITY;
        for (p, v, lo, hi) in [
            (self.position.x, self.velocity.x, min.x, max.x),
            (self.position.y, self.velocity.y, min.y, max.y),
        ] {
            if v != 0.0 {
                wall_time = wall_time
                    .min((f64::from(if v > 0.0 { hi } else { lo }) - f64::from(p)) / f64::from(v));
            }
        }
        let travel = dt.min(self.remaining_secs).min(wall_time.max(0.0));
        let end = (self.position.as_dvec2() + self.velocity.as_dvec2() * travel).as_vec2();
        let half = (player_size + self.size) / 2.0;
        let hit = segment_rect_fraction(self.position, end, player - half, player + half).filter(
            |fraction| fraction * travel < self.remaining_secs && fraction * travel < wall_time,
        );
        self.position = end;
        self.remaining_secs = (self.remaining_secs - dt).max(0.0);
        if hit.is_some() {
            (
                false,
                Some(DamageHit {
                    damage: self.damage,
                    knockback: Vec2::ZERO,
                    knockback_secs: 0.0,
                }),
            )
        } else {
            (dt < wall_time && self.remaining_secs > 0.0, None)
        }
    }
}
fn segment_rect_fraction(start: Vec2, end: Vec2, min: Vec2, max: Vec2) -> Option<f64> {
    let mut enter: f64 = 0.0;
    let mut exit: f64 = 1.0;
    for (p, q, lo, hi) in [
        (start.x, end.x, min.x, max.x),
        (start.y, end.y, min.y, max.y),
    ] {
        let delta = f64::from(q) - f64::from(p);
        if delta == 0.0 {
            if p < lo || p > hi {
                return None;
            }
        } else {
            let a = (f64::from(lo) - f64::from(p)) / delta;
            let b = (f64::from(hi) - f64::from(p)) / delta;
            enter = enter.max(a.min(b));
            exit = exit.min(a.max(b));
            if enter > exit {
                return None;
            }
        }
    }
    Some(enter)
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(transparent)]
pub struct AttackVector(pub (f32, f32));
impl std::fmt::Display for AttackVector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
impl std::str::FromStr for AttackVector {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        valid_vector(value)?;
        Ok(value)
    }
}
pub(crate) fn valid_vector(vector: AttackVector) -> Result<(), String> {
    if vector.0 .0.is_finite() && vector.0 .1.is_finite() {
        Ok(())
    } else {
        Err("knockback must be finite".into())
    }
}
pub(crate) fn blast_radius(radius: f32) -> Result<(), String> {
    crate::behavior::positive(radius)?;
    if (radius * 2.0).is_finite() {
        Ok(())
    } else {
        Err("radius is too large".into())
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AttackFeedback {
    #[serde(default = "default_warning_tint_strength")]
    pub warning_tint_strength: f32,
    pub outline_alpha: f32,
    pub outline_width: f32,
    pub flash_cycles: f32,
    pub tint: (f32, f32, f32),
}
fn default_warning_tint_strength() -> f32 {
    default_feedback().warning_tint_strength
}
/// How strongly a wind-up tint shows at `progress` (0..1) of the wind-up: pulses
/// `flash_cycles` times, full strength at the start.
pub(crate) fn warning_strength(feedback: AttackFeedback, progress: f32) -> f32 {
    (1.0 + (std::f32::consts::TAU * feedback.flash_cycles * progress).cos()) / 2.0
}
pub(crate) fn default_feedback() -> AttackFeedback {
    ron::from_str(include_str!("../data/attack_feedback_defaults.ron"))
        .expect("valid attack feedback defaults")
}
pub(crate) fn valid_feedback(value: AttackFeedback) -> Result<(), String> {
    for channel in [
        value.outline_alpha,
        value.warning_tint_strength,
        value.tint.0,
        value.tint.1,
        value.tint.2,
    ] {
        if !channel.is_finite() || !(0.0..=1.0).contains(&channel) {
            return Err("warning alpha/tint must be between 0 and 1".into());
        }
    }
    crate::behavior::positive(value.outline_width)?;
    crate::behavior::nonnegative(value.flash_cycles)
}
impl std::fmt::Display for AttackFeedback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&ron::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
impl std::str::FromStr for AttackFeedback {
    type Err = String;
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let value: Self = ron::from_str(text).map_err(|e| e.to_string())?;
        valid_feedback(value)?;
        Ok(value)
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackWarning {
    pub attack: AttackBox,
    pub feedback: AttackFeedback,
    pub strength: f32,
}
impl AttackRuntime {
    pub fn windup_progress(&self) -> Option<f32> {
        (self.elapsed < self.telegraph).then(|| (self.elapsed / self.telegraph) as f32)
    }
}
