//! Window-free part geometry: points and parts resolved to pixels, with facing and rotation.
use macroquad::math::Vec2;

use super::template::{BodyPart, PointPositions, Shape};

/// Resolved pixel geometry. Rotation is clockwise in the screen's y-down coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PartTransform {
    pub center: Vec2,
    pub size: Vec2,
    pub rotation: f32,
    pub mirrored: bool,
    pub endpoints: Option<(Vec2, Vec2)>,
}

impl PartTransform {
    /// Transform a shape vertex expressed in fractions of its own size.
    pub fn vertex(&self, local: Vec2) -> Vec2 {
        let mut offset = local * self.size;
        if self.mirrored {
            offset.x = -offset.x;
        }
        self.center + rotate(offset, self.rotation)
    }
}

fn rotate(point: Vec2, degrees: f32) -> Vec2 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

pub fn resolve_point(
    name: &str,
    points: &PointPositions,
    center: Vec2,
    box_size: Vec2,
    scale: f32,
    facing_left: bool,
) -> Result<Vec2, String> {
    let point = points
        .get(name)
        .ok_or_else(|| format!("unknown point {name:?}"))?;
    let mut offset = *point * box_size * scale;
    if facing_left {
        offset.x = -offset.x;
    }
    Ok(center + offset)
}

pub fn resolve_part(
    part: &BodyPart,
    points: &PointPositions,
    center: Vec2,
    box_size: Vec2,
    scale: f32,
    facing_left: bool,
) -> Result<PartTransform, String> {
    let rotation = if facing_left {
        -part.rotation
    } else {
        part.rotation
    };
    let (position, endpoints) = if part.shape == Shape::Line {
        let from = resolve_point(
            part.from.as_deref().ok_or("Line missing from point")?,
            points,
            center,
            box_size,
            scale,
            facing_left,
        )?;
        let to = resolve_point(
            part.to.as_deref().ok_or("Line missing to point")?,
            points,
            center,
            box_size,
            scale,
            facing_left,
        )?;
        let midpoint = (from + to) / 2.0;
        (
            midpoint,
            Some((
                midpoint + rotate(from - midpoint, rotation),
                midpoint + rotate(to - midpoint, rotation),
            )),
        )
    } else {
        let anchor = resolve_point(
            part.anchor.as_deref().ok_or("missing anchor point")?,
            points,
            center,
            box_size,
            scale,
            facing_left,
        )?;
        let mut offset = Vec2::from(part.offset.ok_or("missing offset")?) * box_size * scale;
        if facing_left {
            offset.x = -offset.x;
        }
        (anchor + offset, None)
    };
    Ok(PartTransform {
        center: position,
        size: Vec2::from(part.size) * box_size * scale,
        rotation,
        mirrored: facing_left,
        endpoints,
    })
}
