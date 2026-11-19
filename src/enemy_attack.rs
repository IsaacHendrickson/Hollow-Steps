//! Shared, window-free enemy attack geometry and simulation.
//!
//! One module per job: `attack_box` (shape, box, hit), `runtime` (timed attacks and motion
//! defaults), `projectile`, `options` (aim, size, vector, feedback and their validators). Every
//! item is re-exported here at its old visibility, so paths do not change.
mod attack_box;
mod options;
mod projectile;
mod runtime;

pub use attack_box::{active_window, AttackBox, AttackShape, DamageHit};
pub use options::{
    spread_angles, Aim, AttackFeedback, AttackSize, AttackVector, AttackWarning, OptionalAttack,
};
pub use projectile::Projectile;
pub use runtime::{parabolic_position, AttackRuntime};

pub(crate) use attack_box::valid_attack;
pub(crate) use options::{
    attack_damage, blast_radius, default_feedback, projectile_count, valid_aim, valid_feedback,
    valid_optional_attack, valid_size, valid_vector, warning_strength,
};
pub(crate) use runtime::{default_arc_height, default_knockback_secs};
