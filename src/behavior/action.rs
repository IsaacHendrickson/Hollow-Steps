//! The action registry: one `actions!` table supplies the `Action` enum, loading, validation
//! and the editor catalogs.
use super::action_serde::{
    deserialize_blast, deserialize_jump, deserialize_slam, ActionIdentifier, FieldsVisitor,
};
use super::validators::{
    boss_count, finite, nonnegative, positive, valid_count, valid_damage, valid_pose_name,
};
use crate::action_parameters::EditorParameter;
use crate::enemy_attack::default_knockback_secs;
use crate::enemy_attack::{
    attack_damage, default_arc_height, projectile_count, valid_aim, valid_attack,
    valid_optional_attack, valid_size, Aim, AttackBox, AttackSize, OptionalAttack,
};
use crate::enemy_attack::{blast_radius, valid_vector, AttackVector};
use crate::enemy_attack::{default_feedback, valid_feedback, AttackFeedback};
use crate::pose::{
    default_blend_ratio, default_pose_action_blend_secs, default_strike_multiplier,
    valid_blend_ratio,
};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// One action registry supplies loading, validation and both entity editors.
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
            pub const TYPES: &'static [&'static str] = Self::ALL_TYPES;
            pub const ALL_TYPES: &'static [&'static str] = &[$($name),+, $($boss_name),+];
            pub fn all_kind(&self) -> usize {
                let name = match self { $(Self::$variant { .. } => $name,)+ $(Self::$boss { .. } => $boss_name),+ };
                Self::ALL_TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn all_defaults(kind: usize) -> Option<Self> {
                static DEFAULTS: OnceLock<Vec<Action>> = OnceLock::new();
                DEFAULTS.get_or_init(|| {
                    let mut actions: Vec<Action> = ron::from_str(include_str!("../../data/action_defaults.ron")).expect("valid action defaults");
                    actions.extend(ron::from_str::<Vec<Action>>(include_str!("../../data/boss_action_defaults.ron")).expect("valid boss action defaults"));
                    actions
                }).iter().find(|action| action.all_kind() == kind).cloned()
            }
            pub fn defaults(kind: usize) -> Option<Self> { Self::all_defaults(kind) }
            pub fn kind(&self) -> usize { self.all_kind() }
            pub fn parameters(&self) -> Vec<(&'static str, String)> {
                let mut values = Vec::new();
                match self {
                    $(Self::$variant { $($field),+ } => { $(values.extend($field.to_owned().editor_values().into_iter().map(|v| (stringify!($field), v)));)+ },)+
                    $(Self::$boss { $($param),+ } => { $(values.extend($param.to_owned().editor_values().into_iter().map(|v| (stringify!($param), v)));)+ }),+
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
                    $validate($field.to_owned()).map_err(|error| format!("{}: {error}", $label))?;
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
                            let mut value = $field.to_owned();
                            value.edit(index - offset, text).map_err(|e| format!("{}: {e}", $label))?;
                            $validate(value.to_owned()).map_err(|e| format!("{}: {e}", $label))?;
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
        damage: u16 => ("blast damage (integer 0..65535)", valid_damage)
    },
    Lunge => "lunge" {
        speed: f32 => ("lunge speed (pixels/second)", nonnegative),
        duration_secs: f32 => ("lunge duration (seconds)", nonnegative),
        attack: AttackBox => ("attack box", valid_attack),
        recovery_secs: f32 => ("recovery duration (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback),
        #[serde(default)] telegraph_pose: Option<String> => ("telegraph_pose", valid_pose_name),
        #[serde(default)] active_pose: Option<String> => ("active_pose", valid_pose_name),
        #[serde(default)] recovery_pose: Option<String> => ("recovery_pose", valid_pose_name),
        #[serde(default = "default_blend_ratio")] pose_blend_ratio: f32 => ("pose blend ratio (0..1)", valid_blend_ratio),
        #[serde(default = "default_strike_multiplier")] strike_stiffness_multiplier: f32 => ("strike stiffness multiplier (positive)", positive)
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
        feedback: AttackFeedback => ("warning feedback", valid_feedback),
        #[serde(default)] telegraph_pose: Option<String> => ("telegraph_pose", valid_pose_name),
        #[serde(default)] active_pose: Option<String> => ("active_pose", valid_pose_name),
        #[serde(default)] recovery_pose: Option<String> => ("recovery_pose", valid_pose_name),
        #[serde(default = "default_blend_ratio")] pose_blend_ratio: f32 => ("pose blend ratio (0..1)", valid_blend_ratio),
        #[serde(default = "default_strike_multiplier")] strike_stiffness_multiplier: f32 => ("strike stiffness multiplier (positive)", positive)
    },
    #[serde(rename = "slam_down")]
    ImpactSlam => "slam_down" {
        speed: f32 => ("downward slam speed (pixels/second)", positive),
        attack: AttackBox => ("impact attack box", valid_attack),
        recovery_secs: f32 => ("impact recovery (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative),
        #[serde(default = "default_feedback")]
        feedback: AttackFeedback => ("warning feedback", valid_feedback),
        #[serde(default)] telegraph_pose: Option<String> => ("telegraph_pose", valid_pose_name),
        #[serde(default)] active_pose: Option<String> => ("active_pose", valid_pose_name),
        #[serde(default)] recovery_pose: Option<String> => ("recovery_pose", valid_pose_name),
        #[serde(default = "default_blend_ratio")] pose_blend_ratio: f32 => ("pose blend ratio (0..1)", valid_blend_ratio),
        #[serde(default = "default_strike_multiplier")] strike_stiffness_multiplier: f32 => ("strike stiffness multiplier (positive)", positive)
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
        feedback: AttackFeedback => ("warning feedback", valid_feedback),
        #[serde(default)] telegraph_pose: Option<String> => ("telegraph_pose", valid_pose_name),
        #[serde(default)] active_pose: Option<String> => ("active_pose", valid_pose_name),
        #[serde(default)] recovery_pose: Option<String> => ("recovery_pose", valid_pose_name),
        #[serde(default = "default_blend_ratio")] pose_blend_ratio: f32 => ("pose blend ratio (0..1)", valid_blend_ratio),
        #[serde(default = "default_strike_multiplier")] strike_stiffness_multiplier: f32 => ("strike stiffness multiplier (positive)", positive)
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
        feedback: AttackFeedback => ("warning feedback", valid_feedback),
        #[serde(default)] telegraph_pose: Option<String> => ("telegraph_pose", valid_pose_name),
        #[serde(default)] active_pose: Option<String> => ("active_pose", valid_pose_name),
        #[serde(default)] recovery_pose: Option<String> => ("recovery_pose", valid_pose_name),
        #[serde(default = "default_blend_ratio")] pose_blend_ratio: f32 => ("pose blend ratio (0..1)", valid_blend_ratio),
        #[serde(default = "default_strike_multiplier")] strike_stiffness_multiplier: f32 => ("strike stiffness multiplier (positive)", positive)
    },
    Pose => "pose" {
        #[serde(default = "default_pose_action_blend_secs")] blend_secs: f32 => ("pose blend time (seconds)", positive),
        #[serde(default)] pose_name: Option<String> => ("pose name (empty returns to the idle pose)", valid_pose_name)
    };
    GlideToPlayer => "glide_to_player" {
        speed: f32 => ("Glide speed (px/s)", positive), above_floor: f32 => ("Height above floor (px)", nonnegative)
    },
    GlideToFloor => "glide_to_floor" { speed: f32 => ("Glide speed (px/s)", positive) },
    DamageTarget => "damage_target" {
        radius: f32 => ("Hit radius (px)", nonnegative), damage: u16 => ("Hit damage (0..65535)", valid_damage)
    },
    Summon => "summon" { inset: f32 => ("Distance from arena edges (px)", nonnegative) },
    SpawnRings => "spawn_rings" { count: u16 => ("Ring count (1..256)", boss_count) },
    ExplodeRings => "explode_rings" {
        radius: f32 => ("Explosion radius (px)", nonnegative), damage: u16 => ("Explosion damage (0..65535)", valid_damage)
    },
    SpawnShockwave => "spawn_shockwave" {
        radius: f32 => ("Shockwave radius (px)", nonnegative), damage: u16 => ("Shockwave damage (0..65535)", valid_damage)
    },
}
