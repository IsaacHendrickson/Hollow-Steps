//! Attack geometry: the shape and numbers of one hit, overlap tests and the damage it deals.
use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use super::runtime::default_knockback_secs;

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
