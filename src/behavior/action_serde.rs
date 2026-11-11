//! Deserialization helpers for the action registry: the field and name adapters its generated
//! `Deserialize` impl uses, and the readers for the actions that accept an older document form.
use super::Action;
use crate::enemy_attack::default_knockback_secs;
use crate::enemy_attack::{default_arc_height, AttackBox, AttackVector, OptionalAttack};
use crate::enemy_attack::{default_feedback, AttackFeedback};
use crate::pose::{default_blend_ratio, default_strike_multiplier};
use serde::Deserialize;

// RON uses struct variants. This adapter keeps one registry while allowing old
// jump_to(speed: ...) documents alongside the time-based parabolic action.
pub(super) struct FieldsVisitor<T>(pub(super) std::marker::PhantomData<T>);
impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for FieldsVisitor<T> {
    type Value = T;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("labeled action parameters")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
    }
}
pub(super) struct ActionIdentifier(pub(super) String);
impl<'de> Deserialize<'de> for ActionIdentifier {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct IdentifierVisitor;
        impl serde::de::Visitor<'_> for IdentifierVisitor {
            type Value = ActionIdentifier;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("action name")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                Ok(ActionIdentifier(value.into()))
            }
        }
        d.deserialize_identifier(IdentifierVisitor)
    }
}
fn present<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(d).map(Some)
}
pub(super) fn deserialize_jump<'de, A: serde::de::VariantAccess<'de>>(
    variant: A,
) -> Result<Action, A::Error> {
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        dx: f32,
        dy: f32,
        #[serde(default, deserialize_with = "present")]
        speed: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        air_time_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        arc_height: Option<f32>,
        #[serde(default)]
        attack: OptionalAttack,
        #[serde(default, deserialize_with = "present")]
        recovery_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        telegraph_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        feedback: Option<AttackFeedback>,
        #[serde(default)]
        telegraph_pose: Option<String>,
        #[serde(default)]
        active_pose: Option<String>,
        #[serde(default)]
        recovery_pose: Option<String>,
        #[serde(default, deserialize_with = "present")]
        pose_blend_ratio: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        strike_stiffness_multiplier: Option<f32>,
    }
    let p: Fields = variant.struct_variant(&[], FieldsVisitor(std::marker::PhantomData))?;
    match (p.speed, p.air_time_secs) {
        (Some(speed), None) if p.arc_height.is_none() && p.attack.0.is_none() && p.recovery_secs.is_none() && p.telegraph_secs.is_none() && p.feedback.is_none() && p.telegraph_pose.is_none() && p.active_pose.is_none() && p.recovery_pose.is_none() && p.pose_blend_ratio.is_none() && p.strike_stiffness_multiplier.is_none()
            => Ok(Action::JumpToPlayer { speed, dx: p.dx, dy: p.dy }),
        (None, Some(air_time_secs)) => Ok(Action::JumpTo { dx: p.dx, dy: p.dy, air_time_secs,
            arc_height: p.arc_height.unwrap_or_else(default_arc_height), attack: p.attack,
            recovery_secs: p.recovery_secs.ok_or_else(|| A::Error::custom("jump_to: missing recovery_secs"))?,
            telegraph_secs: p.telegraph_secs.ok_or_else(|| A::Error::custom("jump_to: missing telegraph_secs"))?,
            telegraph_pose: p.telegraph_pose, active_pose: p.active_pose, recovery_pose: p.recovery_pose,
            pose_blend_ratio: p.pose_blend_ratio.unwrap_or_else(default_blend_ratio), strike_stiffness_multiplier: p.strike_stiffness_multiplier.unwrap_or_else(default_strike_multiplier),
            feedback: p.feedback.unwrap_or_else(default_feedback) }),
        _ => Err(A::Error::custom("jump_to: provide air_time_secs and attack parameters, or legacy speed; do not mix forms")),
    }
}
pub(super) fn deserialize_slam<'de, A: serde::de::VariantAccess<'de>>(
    variant: A,
) -> Result<Action, A::Error> {
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        speed: f32,
        #[serde(default, deserialize_with = "present")]
        attack: Option<AttackBox>,
        #[serde(default, deserialize_with = "present")]
        recovery_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        telegraph_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        feedback: Option<AttackFeedback>,
        #[serde(default)]
        telegraph_pose: Option<String>,
        #[serde(default)]
        active_pose: Option<String>,
        #[serde(default)]
        recovery_pose: Option<String>,
        #[serde(default, deserialize_with = "present")]
        pose_blend_ratio: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        strike_stiffness_multiplier: Option<f32>,
    }
    let p: Fields = variant.struct_variant(&[], FieldsVisitor(std::marker::PhantomData))?;
    match p.attack {
        Some(attack) => Ok(Action::ImpactSlam {
            speed: p.speed,
            attack,
            recovery_secs: p
                .recovery_secs
                .ok_or_else(|| A::Error::custom("slam_down: missing recovery_secs"))?,
            telegraph_secs: p
                .telegraph_secs
                .ok_or_else(|| A::Error::custom("slam_down: missing telegraph_secs"))?,
            telegraph_pose: p.telegraph_pose,
            active_pose: p.active_pose,
            recovery_pose: p.recovery_pose,
            pose_blend_ratio: p.pose_blend_ratio.unwrap_or_else(default_blend_ratio),
            strike_stiffness_multiplier: p
                .strike_stiffness_multiplier
                .unwrap_or_else(default_strike_multiplier),
            feedback: p.feedback.unwrap_or_else(default_feedback),
        }),
        None if p.recovery_secs.is_none()
            && p.telegraph_secs.is_none()
            && p.feedback.is_none()
            && p.telegraph_pose.is_none()
            && p.active_pose.is_none()
            && p.recovery_pose.is_none()
            && p.pose_blend_ratio.is_none()
            && p.strike_stiffness_multiplier.is_none() =>
        {
            Ok(Action::SlamDown { speed: p.speed })
        }
        None => Err(A::Error::custom(
            "slam_down: impact/recovery parameters require an attack box",
        )),
    }
}
pub(super) fn deserialize_blast<'de, A: serde::de::VariantAccess<'de>>(
    variant: A,
) -> Result<Action, A::Error> {
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        radius: f32,
        damage: i32,
        #[serde(default, deserialize_with = "present")]
        count: Option<u16>,
        #[serde(default, deserialize_with = "present")]
        fuse_time: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        knockback: Option<AttackVector>,
        #[serde(default, deserialize_with = "present")]
        knockback_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        active_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        telegraph_secs: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        feedback: Option<AttackFeedback>,
        #[serde(default)]
        telegraph_pose: Option<String>,
        #[serde(default)]
        active_pose: Option<String>,
        #[serde(default)]
        recovery_pose: Option<String>,
        #[serde(default, deserialize_with = "present")]
        pose_blend_ratio: Option<f32>,
        #[serde(default, deserialize_with = "present")]
        strike_stiffness_multiplier: Option<f32>,
    }
    let p: Fields = variant.struct_variant(&[], FieldsVisitor(std::marker::PhantomData))?;
    match (p.count, p.fuse_time, p.knockback, p.active_secs) {
        (Some(count), Some(fuse_time), None, None) if p.telegraph_secs.is_none() && p.feedback.is_none() && p.telegraph_pose.is_none() && p.active_pose.is_none() && p.recovery_pose.is_none() && p.pose_blend_ratio.is_none() && p.strike_stiffness_multiplier.is_none() && p.knockback_secs.is_none() => Ok(Action::AreaBlast {
            count, fuse_time, radius: p.radius, damage: u16::try_from(p.damage).map_err(|_| A::Error::custom("legacy blast damage must be 0..65535"))? }),
        (None, None, Some(knockback), Some(active_secs)) => Ok(Action::RadialBlast {
            radius: p.radius, damage: p.damage, knockback, active_secs,
            knockback_secs: p.knockback_secs.unwrap_or_else(default_knockback_secs),
            telegraph_secs: p.telegraph_secs.ok_or_else(|| A::Error::custom("area_blast: missing telegraph_secs"))?,
            telegraph_pose: p.telegraph_pose, active_pose: p.active_pose, recovery_pose: p.recovery_pose,
            pose_blend_ratio: p.pose_blend_ratio.unwrap_or_else(default_blend_ratio), strike_stiffness_multiplier: p.strike_stiffness_multiplier.unwrap_or_else(default_strike_multiplier),
            feedback: p.feedback.unwrap_or_else(default_feedback) }),
        _ => Err(A::Error::custom("area_blast: requires knockback and active_secs, or legacy count and fuse_time; do not mix forms")),
    }
}
