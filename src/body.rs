//! Data-only body templates, validation, and window-free visual transforms.
//!
//! One module per job: `template` (points, parts, springs, catalog loader), `visual` (the
//! authored look), `instance` (a live body), `geometry` (part transforms). Every public item is
//! re-exported here, so paths do not change.
mod geometry;
mod instance;
mod template;
#[cfg(test)]
mod tests;
mod visual;

pub use geometry::{resolve_part, resolve_point, PartTransform};
pub use instance::BodyInstance;
#[cfg(test)]
use macroquad::math::Vec2;
pub use template::{
    load_body_templates, BodyPart, BodyPoint, BodyTemplate, BodyTemplates, ColorSlot, Follow,
    IdleMotion, PointPositions, Shape,
};
pub use visual::BodyVisual;
