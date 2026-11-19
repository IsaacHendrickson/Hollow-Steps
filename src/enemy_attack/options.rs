//! Editable attack option values (aim, size, vector, feedback), their text forms and validators.
use serde::{Deserialize, Serialize};

use super::attack_box::AttackBox;

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
    ron::from_str(include_str!("../../data/attack_feedback_defaults.ron"))
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
