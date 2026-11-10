//! Window-free, data-driven enemy state machines. All time is simulation time.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use macroquad::math::Vec2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::action_parameters::EditorParameter;
use crate::enemy_attack::default_knockback_secs;
use crate::enemy_attack::{
    attack_damage, default_arc_height, projectile_count, spread_angles, valid_aim, valid_attack,
    valid_optional_attack, valid_size, Aim, AttackBox, AttackRuntime, AttackSize, DamageHit,
    OptionalAttack, Projectile,
};
use crate::enemy_attack::{blast_radius, valid_vector, AttackVector};
use crate::enemy_attack::{default_feedback, valid_feedback, AttackFeedback, AttackWarning};
use crate::player::WorldBounds;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehaviorDefinition {
    pub initial_state: String,
    pub states: Vec<State>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub name: String,
    pub actions: Vec<Action>,
    pub transitions: Vec<Transition>,
}

/// Shared action library. Boss-only actions are excluded from the enemy editor catalog.
macro_rules! actions {
    ($( $(#[$meta:meta])* $variant:ident => $name:literal { $( $(#[$fm:meta])* $field:ident : $ty:ty => ($label:literal, $validate:ident) ),+ } ),+ $(,)?;
     $( $boss:ident => $boss_name:literal { $( $param:ident : $param_ty:ty => ($param_label:literal, $param_validate:ident) ),+ } ),+ $(,)?) => {
        #[derive(Clone, Debug, Serialize, PartialEq)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        pub enum Action { $( $(#[$meta])* $variant { $( $(#[$fm])* $field: $ty ),+ } ),+, $( $boss { $( $param: $param_ty ),+ } ),+ }
        impl<'de> Deserialize<'de> for Action {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                struct ActionVisitor;
                impl<'de> serde::de::Visitor<'de> for ActionVisitor {
                    type Value = Action;
                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("a registered action") }
                    fn visit_enum<A: serde::de::EnumAccess<'de>>(self, data: A) -> Result<Action, A::Error> {
                        use serde::de::{Error, VariantAccess};
                        let (name, variant) = data.variant::<ActionIdentifier>()?;
                        if name.0 == "jump_to" { return deserialize_jump(variant); }
                        if name.0 == "slam_down" { return deserialize_slam(variant); }
                        if name.0 == "area_blast" { return deserialize_blast(variant); }
                        match name.0.as_str() {
                            $($name => {
                                #[derive(Deserialize)]
                                #[serde(deny_unknown_fields)]
                                struct Fields { $( $(#[$fm])* $field: $ty ),+ }
                                let fields: Fields = variant.struct_variant(&[$(stringify!($field)),+], FieldsVisitor(std::marker::PhantomData))?;
                                Ok(Action::$variant { $($field: fields.$field),+ })
                            },)+
                            $($boss_name => {
                                #[derive(Deserialize)]
                                #[serde(deny_unknown_fields)]
                                struct Fields { $( $param: $param_ty ),+ }
                                let fields: Fields = variant.struct_variant(&[$(stringify!($param)),+], FieldsVisitor(std::marker::PhantomData))?;
                                Ok(Action::$boss { $($param: fields.$param),+ })
                            },)+
                            _ => Err(A::Error::custom(format!("unknown action {:?}", name.0)))
                        }
                    }
                }
                deserializer.deserialize_enum("Action", Action::ALL_TYPES, ActionVisitor)
            }
        }
        impl Action {
            pub const TYPES: &'static [&'static str] = &[$($name),+];
            pub const ALL_TYPES: &'static [&'static str] = &[$($name),+, $($boss_name),+];
            pub fn all_kind(&self) -> usize {
                let name = match self { $(Self::$variant { .. } => $name,)+ $(Self::$boss { .. } => $boss_name),+ };
                Self::ALL_TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn all_defaults(kind: usize) -> Option<Self> {
                static DEFAULTS: OnceLock<Vec<Action>> = OnceLock::new();
                DEFAULTS.get_or_init(|| {
                    let mut actions: Vec<Action> = ron::from_str(include_str!("../data/action_defaults.ron")).expect("valid action defaults");
                    actions.extend(ron::from_str::<Vec<Action>>(include_str!("../data/boss_action_defaults.ron")).expect("valid boss action defaults"));
                    actions
                }).iter().find(|action| action.all_kind() == kind).cloned()
            }
            pub fn defaults(kind: usize) -> Option<Self> {
                static DEFAULTS: OnceLock<Vec<Action>> = OnceLock::new();
                DEFAULTS.get_or_init(|| ron::from_str(include_str!("../data/action_defaults.ron"))
                    .expect("valid action catalog defaults"))
                    .iter().find(|action| action.kind() == kind).cloned()
            }
            pub fn kind(&self) -> usize {
                let name = match self { $(Self::$variant { .. } => $name,)+ _ => return usize::MAX };
                Self::TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn parameters(&self) -> Vec<(&'static str, String)> {
                let mut values = Vec::new();
                match self {
                    $(Self::$variant { $($field),+ } => { $(values.extend($field.editor_values().into_iter().map(|v| (stringify!($field), v)));)+ },)+
                    $(Self::$boss { $($param),+ } => { $(values.extend($param.editor_values().into_iter().map(|v| (stringify!($param), v)));)+ }),+
                }
                values
            }
            pub fn parameter_labels(&self) -> Vec<String> {
                let mut labels = Vec::new();
                match self {
                    $(Self::$variant { .. } => { $(labels.extend(<$ty as EditorParameter>::editor_labels($label));)+ },)+
                    $(Self::$boss { .. } => { $(labels.extend(<$param_ty as EditorParameter>::editor_labels($param_label));)+ }),+
                }
                labels
            }
            pub fn validate(&self) -> Result<(), String> {
                match self { $(Self::$variant { $($field),+ } => { $(
                    $validate(*$field).map_err(|error| format!("{}: {error}", $label))?;
                )+ },)+ $(Self::$boss { $($param),+ } => { $(
                    $param_validate(*$param).map_err(|error| format!("{} ({}): {error}", stringify!($param), $param_label))?;
                )+ }),+ }
                Ok(())
            }
            pub fn set_parameter(&mut self, index: usize, text: &str) -> Result<(), String> {
                let mut offset = 0;
                match self {
                    $(Self::$variant { $($field),+ } => { $(
                        let count = <$ty as EditorParameter>::LABELS.len();
                        if (offset..offset + count).contains(&index) {
                            let mut value = *$field;
                            value.edit(index - offset, text).map_err(|e| format!("{}: {e}", $label))?;
                            $validate(value).map_err(|e| format!("{}: {e}", $label))?;
                            *$field = value;
                            return Ok(());
                        }
                        offset += count;
                    )+ },)+
                    $(Self::$boss { $($param),+ } => { $(
                        let count = <$param_ty as EditorParameter>::LABELS.len();
                        if (offset..offset + count).contains(&index) {
                            let mut value = *$param;
                            value.edit(index - offset, text).map_err(|e| format!("{}: {e}", $param_label))?;
                            $param_validate(value).map_err(|e| format!("{}: {e}", $param_label))?;
                            *$param = value;
                            return Ok(());
                        }
                        offset += count;
                    )+ }),+
                }
                let _ = offset;
                Err("missing action parameter".into())
            }
        }
    }
}
// RON uses struct variants. This adapter keeps one registry while allowing old
// jump_to(speed: ...) documents alongside the time-based parabolic action.
struct FieldsVisitor<T>(std::marker::PhantomData<T>);
impl<'de, T: Deserialize<'de>> serde::de::Visitor<'de> for FieldsVisitor<T> {
    type Value = T;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("labeled action parameters")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::deserialize(serde::de::value::MapAccessDeserializer::new(map))
    }
}
struct ActionIdentifier(String);
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
fn deserialize_jump<'de, A: serde::de::VariantAccess<'de>>(variant: A) -> Result<Action, A::Error> {
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
    }
    let p: Fields = variant.struct_variant(&[], FieldsVisitor(std::marker::PhantomData))?;
    match (p.speed, p.air_time_secs) {
        (Some(speed), None) if p.arc_height.is_none() && p.attack.0.is_none() && p.recovery_secs.is_none() && p.telegraph_secs.is_none() && p.feedback.is_none()
            => Ok(Action::JumpToPlayer { speed, dx: p.dx, dy: p.dy }),
        (None, Some(air_time_secs)) => Ok(Action::JumpTo { dx: p.dx, dy: p.dy, air_time_secs,
            arc_height: p.arc_height.unwrap_or_else(default_arc_height), attack: p.attack,
            recovery_secs: p.recovery_secs.ok_or_else(|| A::Error::custom("jump_to: missing recovery_secs"))?,
            telegraph_secs: p.telegraph_secs.ok_or_else(|| A::Error::custom("jump_to: missing telegraph_secs"))?,
            feedback: p.feedback.unwrap_or_else(default_feedback) }),
        _ => Err(A::Error::custom("jump_to: provide air_time_secs and attack parameters, or legacy speed; do not mix forms")),
    }
}
fn deserialize_slam<'de, A: serde::de::VariantAccess<'de>>(variant: A) -> Result<Action, A::Error> {
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
            feedback: p.feedback.unwrap_or_else(default_feedback),
        }),
        None if p.recovery_secs.is_none() && p.telegraph_secs.is_none() && p.feedback.is_none() => {
            Ok(Action::SlamDown { speed: p.speed })
        }
        None => Err(A::Error::custom(
            "slam_down: impact/recovery parameters require an attack box",
        )),
    }
}
fn deserialize_blast<'de, A: serde::de::VariantAccess<'de>>(
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
    }
    let p: Fields = variant.struct_variant(&[], FieldsVisitor(std::marker::PhantomData))?;
    match (p.count, p.fuse_time, p.knockback, p.active_secs) {
        (Some(count), Some(fuse_time), None, None) if p.telegraph_secs.is_none() && p.feedback.is_none() && p.knockback_secs.is_none() => Ok(Action::AreaBlast {
            count, fuse_time, radius: p.radius, damage: u8::try_from(p.damage).map_err(|_| A::Error::custom("legacy blast damage must be 0..255"))? }),
        (None, None, Some(knockback), Some(active_secs)) => Ok(Action::RadialBlast {
            radius: p.radius, damage: p.damage, knockback, active_secs,
            knockback_secs: p.knockback_secs.unwrap_or_else(default_knockback_secs),
            telegraph_secs: p.telegraph_secs.ok_or_else(|| A::Error::custom("area_blast: missing telegraph_secs"))?,
            feedback: p.feedback.unwrap_or_else(default_feedback) }),
        _ => Err(A::Error::custom("area_blast: requires knockback and active_secs, or legacy count and fuse_time; do not mix forms")),
    }
}

actions! {
    MoveTowardPlayer => "move_toward_player" { speed: f32 => ("speed toward player (pixels/second)", nonnegative) },
    MoveAwayFromPlayer => "move_away_from_player" { speed: f32 => ("speed away from player (pixels/second)", nonnegative) },
    Patrol => "patrol" { speed: f32 => ("patrol speed (pixels/second)", nonnegative) },
    Wait => "wait" { seconds: f32 => ("wait duration (seconds)", nonnegative) },
    Telegraph => "telegraph" { seconds: f32 => ("telegraph duration (seconds)", nonnegative) },
    JumpToPlayer => "jump_to_player" {
        speed: f32 => ("jump speed (pixels/second)", nonnegative),
        dx: f32 => ("target horizontal offset from player (pixels, positive right)", finite),
        dy: f32 => ("target vertical offset from player (pixels, negative above)", finite)
    },
    Hover => "hover" { seconds: f32 => ("hover duration (seconds)", nonnegative) },
    SlamDown => "legacy_slam_down" { speed: f32 => ("downward slam speed (pixels/second)", nonnegative) },
    AreaBlast => "legacy_area_blast" {
        count: u16 => ("blast count (integer 0..65535)", valid_count),
        radius: f32 => ("blast radius (pixels)", nonnegative),
        fuse_time: f32 => ("blast fuse time (seconds)", nonnegative),
        damage: u8 => ("blast damage (integer 0..255)", valid_damage)
    },
    Lunge => "lunge" {
        speed: f32 => ("lunge speed (pixels/second)", nonnegative),
        duration_secs: f32 => ("lunge duration (seconds)", nonnegative),
        attack: AttackBox => ("attack box", valid_attack),
        recovery_secs: f32 => ("recovery duration (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback)
    },
    JumpTo => "jump_to" {
        air_time_secs: f32 => ("air time (seconds)", positive),
        dx: f32 => ("target offset x (pixels)", finite),
        dy: f32 => ("target offset y (pixels)", finite),
        #[serde(default = "default_arc_height")]
        arc_height: f32 => ("arc height (pixels)", nonnegative),
        #[serde(default)]
        attack: OptionalAttack => ("optional landing attack box", valid_optional_attack),
        recovery_secs: f32 => ("landing recovery (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback)
    },
    #[serde(rename = "slam_down")]
    ImpactSlam => "slam_down" {
        speed: f32 => ("downward slam speed (pixels/second)", positive),
        attack: AttackBox => ("impact attack box", valid_attack),
        recovery_secs: f32 => ("impact recovery (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback)
    },
    ShootProjectile => "shoot_projectile" {
        speed: f32 => ("projectile speed (pixels/second)", positive),
        damage: i32 => ("projectile damage (integer)", attack_damage),
        lifetime_secs: f32 => ("projectile lifetime (seconds)", nonnegative),
        size: AttackSize => ("projectile size (pixels)", valid_size),
        aim: Aim => ("aim mode (AtPlayer or Horizontal)", valid_aim),
        count: u16 => ("projectile count (integer, at least 1)", projectile_count),
        spread_degrees: f32 => ("total fan spread (degrees)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback)
    },
    #[serde(rename = "area_blast")]
    RadialBlast => "area_blast" {
        radius: f32 => ("blast radius (pixels)", blast_radius),
        damage: i32 => ("blast damage (integer)", attack_damage),
        knockback: AttackVector => ("blast knockback (pixels/second)", valid_vector),
        #[serde(default = "default_knockback_secs")]
        knockback_secs: f32 => ("horizontal knockback duration (seconds)", nonnegative),
        active_secs: f32 => ("blast active duration (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback)
    };
    GlideToPlayer => "glide_to_player" {
        speed: f32 => ("Glide speed (px/s)", positive), above_floor: f32 => ("Height above floor (px)", nonnegative)
    },
    GlideToFloor => "glide_to_floor" { speed: f32 => ("Glide speed (px/s)", positive) },
    DamageTarget => "damage_target" {
        radius: f32 => ("Hit radius (px)", nonnegative), damage: u8 => ("Hit damage (0..255)", valid_damage)
    },
    Summon => "summon" { inset: f32 => ("Distance from arena edges (px)", nonnegative) },
    SpawnRings => "spawn_rings" { count: u16 => ("Ring count (1..256)", boss_count) },
    ExplodeRings => "explode_rings" {
        radius: f32 => ("Explosion radius (px)", nonnegative), damage: u8 => ("Explosion damage (0..255)", valid_damage)
    },
    SpawnShockwave => "spawn_shockwave" {
        radius: f32 => ("Shockwave radius (px)", nonnegative), damage: u8 => ("Shockwave damage (0..255)", valid_damage)
    },
}

pub(crate) fn positive(value: f32) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err("must be finite and positive".into())
    }
}
fn boss_count(value: u16) -> Result<(), String> {
    if (1..=256).contains(&value) {
        Ok(())
    } else {
        Err("must be an integer in 1..256".into())
    }
}

fn finite(value: f32) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("expected a finite value".into())
    }
}

// Integer ranges are enforced by the loader and editor's typed parsing.
fn valid_count(_: u16) -> Result<(), String> {
    Ok(())
}
fn valid_damage(_: u8) -> Result<(), String> {
    Ok(())
}

macro_rules! conditions {
    ($( $variant:ident => $name:literal ($ty:ty, $label:literal, $default:expr) ),+; $( $unit:ident => $unit_name:literal ),+ $(,)?) => {
        #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
        #[serde(rename_all = "snake_case")]
        pub enum Condition { $($variant($ty),)+ $($unit),+ }
        impl Condition {
            pub const TYPES: &'static [&'static str] = &[$($name,)+ $($unit_name),+];
            pub fn defaults(kind: usize) -> Option<Self> {
                Some(match *Self::TYPES.get(kind)? { $($name => Self::$variant($default),)+ $($unit_name => Self::$unit,)+ _ => return None })
            }
            pub fn kind(&self) -> usize {
                let name = match self { $(Self::$variant(_) => $name,)+ $(Self::$unit => $unit_name),+ };
                Self::TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn parameter(&self) -> Option<(&'static str, String)> {
                match self { $(Self::$variant(value) => Some(($label, value.to_string())),)+ $(Self::$unit => None),+ }
            }
            pub fn set_parameter(&mut self, text: &str) -> Result<(), String> {
                let text = text.trim();
                parse_nonnegative(text)?;
                match self {
                    $(Self::$variant(value) => *value = text.parse::<$ty>().map_err(|_| concat!("expected ", stringify!($ty), " value"))?,)+
                    $(Self::$unit => return Err("condition has no value".into())),+
                }
                Ok(())
            }
        }
    }
}
conditions! {
    TimerElapsed => "timer_elapsed" (f32, "seconds", 1.0),
    PlayerDistanceLessThan => "player_distance_less_than" (f32, "distance", 150.0),
    PlayerDistanceGreaterThan => "player_distance_greater_than" (f32, "distance", 150.0),
    HealthBelow => "health_below" (u8, "health (0..255)", 2);
    ActionFinished => "action_finished",
}

pub fn parse_nonnegative(text: &str) -> Result<f32, String> {
    let value = text
        .trim()
        .parse::<f32>()
        .map_err(|_| "expected a number".to_owned())?;
    nonnegative(value)?;
    Ok(value)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub target: String,
    pub condition: Condition,
    /// Inversion allows an exact complement, including distance equality.
    #[serde(default)]
    pub invert: bool,
}

impl BehaviorDefinition {
    /// A stationary starting point for authoring, with no inherited actions or links.
    pub fn blank() -> Self {
        Self {
            initial_state: "idle".into(),
            states: vec![State {
                name: "idle".into(),
                actions: vec![],
                transitions: vec![],
            }],
        }
    }

    pub fn from_ron(source: &str) -> Result<Self, String> {
        let definition: Self = ron::from_str(source).map_err(|error| error.to_string())?;
        definition.validate()?;
        Ok(definition)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut names = BTreeSet::new();
        for state in &self.states {
            if state.name.trim().is_empty() || !names.insert(state.name.as_str()) {
                return Err(format!("empty or duplicate state name {:?}", state.name));
            }
        }
        if !names.contains(self.initial_state.as_str()) {
            return Err(format!("missing initial state {:?}", self.initial_state));
        }
        for state in &self.states {
            for (index, action) in state.actions.iter().enumerate() {
                if action.kind() == usize::MAX {
                    return Err(format!(
                        "state {:?}, action {index}: boss-only action",
                        state.name
                    ));
                }
                action
                    .validate()
                    .map_err(|error| format!("state {:?}, action {index}: {error}", state.name))?;
            }
            for transition in &state.transitions {
                if !names.contains(transition.target.as_str()) {
                    return Err(format!(
                        "state {:?}: transition to missing state {:?}",
                        state.name, transition.target
                    ));
                }
                match transition.condition {
                    Condition::TimerElapsed(value)
                    | Condition::PlayerDistanceLessThan(value)
                    | Condition::PlayerDistanceGreaterThan(value) => {
                        nonnegative(value).map_err(|error| {
                            format!("state {:?}, transition condition: {error}", state.name)
                        })?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn nonnegative(value: f32) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("expected a finite nonnegative value, got {value}"))
    }
}

pub(crate) fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let source =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    ron::from_str(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn ron_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in
        std::fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .path();
        if path.extension().is_some_and(|extension| extension == "ron") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn load_behaviors(
    directory: &Path,
) -> Result<BTreeMap<String, Arc<BehaviorDefinition>>, String> {
    let mut definitions = BTreeMap::new();
    for path in ron_files(directory)? {
        let definition: BehaviorDefinition = read_ron(&path)?;
        definition
            .validate()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{}: invalid behavior filename", path.display()))?;
        definitions.insert(name.to_owned(), Arc::new(definition));
    }
    Ok(definitions)
}

/// Inputs supplied by the owning entity, rather than a renderer or global world.
#[derive(Clone, Copy, Debug)]
pub struct BehaviorContext {
    pub player_position: Vec2,
    pub health: u8,
    pub size: Vec2,
    pub bounds: WorldBounds,
    pub patrol_left: f32,
    pub patrol_right: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct ConditionContext {
    pub state_elapsed: f64,
    pub player_distance: f64,
    pub health: u8,
    pub action_finished: bool,
}

impl Condition {
    pub fn matches(&self, context: ConditionContext) -> bool {
        match *self {
            Self::TimerElapsed(seconds) => context.state_elapsed >= f64::from(seconds),
            Self::PlayerDistanceLessThan(distance) => context.player_distance < f64::from(distance),
            Self::PlayerDistanceGreaterThan(distance) => {
                context.player_distance > f64::from(distance)
            }
            Self::HealthBelow(health) => context.health < health,
            Self::ActionFinished => context.action_finished,
        }
    }
}

/// Each instance owns its timers and action cursor; definitions may be shared.
#[derive(Clone, Debug)]
pub struct Behavior {
    definition: Arc<BehaviorDefinition>,
    state: usize,
    action: usize,
    state_elapsed: f64,
    action_elapsed: f64,
    direction: f32,
    action_started: bool,
    jump_target: Option<Vec2>,
    airborne: bool,
    vertical_velocity: f64,
    blasts: Vec<Blast>,
    random_state: Option<u32>,
    attack_runtime: Option<AttackRuntime>,
    projectiles: Vec<Projectile>,
    projectile_hits: Vec<DamageHit>,
}

/// A stationary warning circle; `exploded` is a damage event for one update.
#[derive(Clone, Debug, PartialEq)]
pub struct Blast {
    pub position: Vec2,
    pub radius: f32,
    pub damage: u8,
    pub exploded: bool,
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
            action_elapsed: 0.0,
            direction: 1.0,
            action_started: false,
            jump_target: None,
            airborne: false,
            vertical_velocity: 0.0,
            blasts: Vec::new(),
            random_state: None,
            attack_runtime: None,
            projectiles: Vec::new(),
            projectile_hits: Vec::new(),
        })
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

    pub fn blasts(&self) -> &[Blast] {
        &self.blasts
    }

    pub fn blast_damage(&self, position: Vec2, size: Vec2) -> u8 {
        self.blasts
            .iter()
            .filter(|blast| {
                let nearest = blast
                    .position
                    .clamp(position - size / 2.0, position + size / 2.0);
                blast.exploded
                    && (nearest.as_dvec2() - blast.position.as_dvec2()).length()
                        <= f64::from(blast.radius)
            })
            .map(|blast| blast.damage)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn clear_blast_hits(&mut self) {
        self.blasts.retain(|blast| !blast.exploded);
    }

    pub fn attack_hit(&self, origin: Vec2, position: Vec2, size: Vec2) -> Option<DamageHit> {
        self.attack_runtime
            .as_ref()
            .and_then(|r| r.active)
            .filter(|attack| attack.overlaps(origin, position, size))
            .map(AttackBox::hit)
            .into_iter()
            .chain(self.projectile_hits.iter().copied())
            .max_by_key(|hit| hit.damage)
    }

    pub(crate) fn advance_projectiles(
        &mut self,
        player: Vec2,
        player_size: Vec2,
        bounds: WorldBounds,
        dt: f32,
    ) {
        self.projectile_hits.clear();
        self.projectiles.retain_mut(|projectile| {
            let (alive, hit) = projectile.step(f64::from(dt), bounds, player, player_size);
            self.projectile_hits.extend(hit);
            alive
        });
    }

    pub(crate) fn pause_attack(&mut self) {
        if let Some(runtime) = &mut self.attack_runtime {
            runtime.active = None;
        }
    }

    pub fn projectiles(&self) -> &[Projectile] {
        &self.projectiles
    }

    pub fn attack_warning(&self) -> Option<AttackBox> {
        self.attack_runtime.as_ref()?.warning()
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
                (self.action_elapsed / f64::from(*seconds)) as f32,
            )
        } else {
            let progress = match &self.attack_runtime {
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
        self.state = self
            .definition
            .states
            .iter()
            .position(|state| state.name == target)
            .expect("validated transition");
        self.action = 0;
        self.action_elapsed = 0.0;
        self.state_elapsed = 0.0;
        self.action_started = false;
        self.jump_target = None;
        self.attack_runtime = None;
        // Interrupting a fuse cancels its pending attack; keep hits already emitted.
        self.blasts.retain(|blast| blast.exploded);
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
        self.clear_blast_hits();
        self.projectile_hits.clear();
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
        let dt = f64::from(delta_seconds);
        self.state_elapsed += dt;
        let mut remaining = dt;
        while let Some(action) = self.definition.states[self.state]
            .actions
            .get(self.action)
            .cloned()
        {
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
                            Aim::AtPlayer => {
                                (target - *position).try_normalize().unwrap_or(horizontal)
                            }
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
                    self.attack_runtime
                        .get_or_insert_with(|| {
                            AttackRuntime::lunge(
                                *position,
                                context,
                                speed,
                                duration_secs,
                                attack,
                                recovery_secs,
                                telegraph_secs,
                            )
                        })
                        .step(position, context, remaining)
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
                            let x = f64::from(context.bounds.left)
                                + self.next_random(*position)
                                    * (f64::from(context.bounds.right)
                                        - f64::from(context.bounds.left));
                            let y = self.next_random(*position) * f64::from(context.bounds.floor_y);
                            self.blasts.push(Blast {
                                position: Vec2::new(x as f32, y as f32),
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
            if self.airborne {
                // Authored jump/slam arcs and explicit hover retain their trajectories.
                self.vertical_velocity = 0.0;
            } else {
                self.apply_gravity(position, context, used);
            }
            self.action_started = true;
            remaining = (remaining - used).max(0.0);
            if !finished {
                break;
            }
            self.attack_runtime = None;
            self.action += 1;
            self.action_elapsed = 0.0;
            self.action_started = false;
            self.jump_target = None;
        }
        if !self.airborne {
            self.apply_gravity(position, context, remaining);
        }
        if !transitioned {
            self.transition(*position, context);
        }
    }

    /// Shared vertical integration for ground movement, lunges, and idle time.
    fn apply_gravity(&mut self, position: &mut Vec2, context: BehaviorContext, dt: f64) {
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

    fn next_random(&mut self, position: Vec2) -> f64 {
        // Per-instance deterministic PRNG; coefficients are algorithm constants.
        let state = self
            .random_state
            .get_or_insert(position.x.to_bits().rotate_left(16) ^ position.y.to_bits());
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(*state) / f64::from(u32::MAX)
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

impl Action {
    pub fn windup_seconds(&self) -> Option<f32> {
        match self {
            Self::Lunge { telegraph_secs, .. }
            | Self::JumpTo { telegraph_secs, .. }
            | Self::ImpactSlam { telegraph_secs, .. }
            | Self::ShootProjectile { telegraph_secs, .. }
            | Self::RadialBlast { telegraph_secs, .. } => Some(*telegraph_secs),
            _ => None,
        }
    }
    pub fn feedback(&self) -> Option<AttackFeedback> {
        match self {
            Self::Lunge { feedback, .. }
            | Self::JumpTo { feedback, .. }
            | Self::ImpactSlam { feedback, .. }
            | Self::ShootProjectile { feedback, .. }
            | Self::RadialBlast { feedback, .. } => Some(*feedback),
            _ => None,
        }
    }
    pub fn preview_attack(&self) -> Option<AttackBox> {
        match self {
            Self::Lunge { attack, .. } | Self::ImpactSlam { attack, .. } => Some(*attack),
            Self::JumpTo { attack, .. } => attack.0,
            Self::RadialBlast {
                radius,
                damage,
                knockback,
                knockback_secs,
                active_secs,
                ..
            } => Some(AttackBox {
                shape: crate::enemy_attack::AttackShape::Circle,
                offset: (0.0, 0.0),
                size: (*radius * 2.0, *radius * 2.0),
                damage: *damage,
                knockback: knockback.0,
                knockback_secs: *knockback_secs,
                active_secs: *active_secs,
            }),
            Self::ShootProjectile { size, damage, .. } => Some(AttackBox {
                shape: crate::enemy_attack::AttackShape::Rect,
                offset: (0.0, 0.0),
                size: size.0,
                damage: *damage,
                knockback: (0.0, 0.0),
                knockback_secs: 0.0,
                active_secs: 0.0,
            }),
            _ => None,
        }
    }
}
