//! What an action reveals about itself without running: wind-up, feedback, attack preview
//! and pose bindings.
use super::Action;
use crate::enemy_attack::{AttackBox, AttackFeedback};

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

impl Action {
    pub fn pose_bindings(&self) -> Option<[Option<String>; 3]> {
        self.pose_binding_refs()
            .map(|names| names.map(Clone::clone))
    }

    /// The same three pose names without copying them, for per-frame use.
    pub(crate) fn pose_binding_refs(&self) -> Option<[&Option<String>; 3]> {
        match self {
            Self::Lunge {
                telegraph_pose,
                active_pose,
                recovery_pose,
                ..
            }
            | Self::JumpTo {
                telegraph_pose,
                active_pose,
                recovery_pose,
                ..
            }
            | Self::ImpactSlam {
                telegraph_pose,
                active_pose,
                recovery_pose,
                ..
            }
            | Self::ShootProjectile {
                telegraph_pose,
                active_pose,
                recovery_pose,
                ..
            }
            | Self::RadialBlast {
                telegraph_pose,
                active_pose,
                recovery_pose,
                ..
            } => Some([telegraph_pose, active_pose, recovery_pose]),
            _ => None,
        }
    }
}

impl Action {
    pub fn pose_timing(&self) -> (f32, f32) {
        match self {
            Self::Lunge {
                telegraph_secs,
                pose_blend_ratio,
                strike_stiffness_multiplier,
                ..
            }
            | Self::JumpTo {
                telegraph_secs,
                pose_blend_ratio,
                strike_stiffness_multiplier,
                ..
            }
            | Self::ImpactSlam {
                telegraph_secs,
                pose_blend_ratio,
                strike_stiffness_multiplier,
                ..
            }
            | Self::ShootProjectile {
                telegraph_secs,
                pose_blend_ratio,
                strike_stiffness_multiplier,
                ..
            }
            | Self::RadialBlast {
                telegraph_secs,
                pose_blend_ratio,
                strike_stiffness_multiplier,
                ..
            } => (
                *telegraph_secs * *pose_blend_ratio,
                *strike_stiffness_multiplier,
            ),
            _ => (0.0, 1.0),
        }
    }
}

impl Action {
    pub fn pose_blend_seconds(&self, telegraph: f32) -> f32 {
        match self {
            Self::Lunge {
                pose_blend_ratio, ..
            }
            | Self::JumpTo {
                pose_blend_ratio, ..
            }
            | Self::ImpactSlam {
                pose_blend_ratio, ..
            }
            | Self::ShootProjectile {
                pose_blend_ratio, ..
            }
            | Self::RadialBlast {
                pose_blend_ratio, ..
            } => telegraph * *pose_blend_ratio,
            _ => 0.0,
        }
    }
}
