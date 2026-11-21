//! Terrain hazards: rectangles that hurt the player on touch and knock them away. Here: how a
//! room file stores one (`HazardPlacement`, in `RoomLayout::hazards`), how the layout checks
//! them, and the hit a hazard deals (`Hazard::hit`, gathered by `damage::DamageResolver`).
//! The editor's add, move and edit operations are in `editor_hazard`; `docs/hazards.md`.
//!
//! A hazard is not solid: the player walks into it and takes the hit. Standing on top of one
//! (touching its top edge) is safe; the body must overlap it. The hit throws the player away
//! from the side that was touched: sideways (and up) from a side, straight up from the top,
//! straight down from underneath. How a hazard looks (style, facing, color) is `hazard_look`.
use macroquad::math::{Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::editor::RoomLayout;
use crate::enemy_attack::DamageHit;
use crate::hazard_look::{self, HazardFacing, HazardStyle};
use crate::terrain_style::{Rgb, Shape};

/// The smallest width and height of a hazard, in pixels.
pub const MIN_HAZARD_SIZE: f32 = 8.0;
/// The largest damage a hazard can deal (the player's health is a few dozen).
pub const MAX_HAZARD_DAMAGE: u16 = 999;
/// The largest knockback or lift speed, in pixels per second.
pub const MAX_HAZARD_SPEED: f32 = 2000.0;
/// The longest horizontal knockback, in seconds.
pub const MAX_HAZARD_KNOCKBACK_SECS: f32 = 2.0;

/// Overlaps shallower than this count as touching (the same as `terrain`'s).
const TOUCH_EPSILON: f32 = 0.001;
/// The player's body at the room spawn, which must stay clear of every hazard.
const SPAWN_BODY: Vec2 = Vec2::new(32.0, 48.0);

/// What the editor gives a new hazard; the author then edits each hazard's own numbers.
const NEW_SIZE: (f32, f32) = (64.0, 16.0);
const NEW_DAMAGE: u16 = 10;
/// The push must beat walking toward the hazard (220 px/s), or holding the key undoes it.
const NEW_KNOCKBACK_SPEED: f32 = 420.0;
const NEW_LIFT_SPEED: f32 = 320.0;
const NEW_KNOCKBACK_SECS: f32 = 0.25;

/// One hazard in a room file.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HazardPlacement {
    /// `(x, y, width, height)` in room coordinates, like a solid.
    pub rect: (f32, f32, f32, f32),
    /// Health the hit takes (`HIT_DAMAGE` is 10). At least 1.
    pub damage: u16,
    /// How hard the hit throws the player away from the side that was touched, in pixels per
    /// second: away sideways from a side, straight up from the top, straight down from below.
    pub knockback_speed: f32,
    /// Extra upward speed when the hit throws the player sideways, in pixels per second.
    pub lift_speed: f32,
    /// How long the sideways push lasts, in seconds.
    pub knockback_secs: f32,
    /// What it looks like. Files from before styles existed have spikes.
    #[serde(default)]
    pub style: HazardStyle,
    /// The side its points or surface face. Files from before facing existed face up.
    #[serde(default)]
    pub facing: HazardFacing,
    /// The color, each part 0 to 1, or `None` for the style's own. Not written when `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<Rgb>,
}

/// What the editor panel edits for one hazard: its size, its hit and its look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HazardFields {
    pub width: f32,
    pub height: f32,
    pub damage: u16,
    pub knockback_speed: f32,
    pub lift_speed: f32,
    pub knockback_secs: f32,
    pub style: HazardStyle,
    pub facing: HazardFacing,
    pub color: Option<Rgb>,
}

/// A hazard in the room being played.
#[derive(Clone, Debug, PartialEq)]
pub struct Hazard {
    pub rect: Rect,
    pub damage: u16,
    pub knockback_speed: f32,
    pub lift_speed: f32,
    pub knockback_secs: f32,
    pub style: HazardStyle,
    pub facing: HazardFacing,
    pub color: Option<Rgb>,
}

/// Which side of a hazard a touching body is on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Contact {
    Top,
    Bottom,
    Left,
    Right,
}

impl From<&HazardPlacement> for Hazard {
    fn from(placement: &HazardPlacement) -> Self {
        Self {
            rect: placement.rect(),
            damage: placement.damage,
            knockback_speed: placement.knockback_speed,
            lift_speed: placement.lift_speed,
            knockback_secs: placement.knockback_secs,
            style: placement.style,
            facing: placement.facing,
            color: placement.color,
        }
    }
}

impl From<&Hazard> for HazardPlacement {
    fn from(hazard: &Hazard) -> Self {
        Self {
            rect: (hazard.rect.x, hazard.rect.y, hazard.rect.w, hazard.rect.h),
            damage: hazard.damage,
            knockback_speed: hazard.knockback_speed,
            lift_speed: hazard.lift_speed,
            knockback_secs: hazard.knockback_secs,
            style: hazard.style,
            facing: hazard.facing,
            color: hazard.color,
        }
    }
}

impl Hazard {
    /// The body centered at `center` overlaps the hazard (touching an edge does not count).
    pub fn touches(&self, center: Vec2, size: Vec2) -> bool {
        let body = Rect::new(
            center.x - size.x / 2.0,
            center.y - size.y / 2.0,
            size.x,
            size.y,
        );
        body.x < self.rect.right() - TOUCH_EPSILON
            && body.right() > self.rect.x + TOUCH_EPSILON
            && body.y < self.rect.bottom() - TOUCH_EPSILON
            && body.bottom() > self.rect.y + TOUCH_EPSILON
    }

    /// The color it is drawn in: its own, or its style's.
    pub fn draw_color(&self) -> Rgb {
        self.color.unwrap_or_else(|| self.style.default_color())
    }

    /// The hazard as a draw list in its style, facing and color; `time` is seconds (only lava
    /// moves). Drawing only, never collision.
    pub fn look(&self, time: f32) -> Vec<Shape> {
        hazard_look::shapes(self.rect, self.style, self.facing, self.draw_color(), time)
    }

    /// The side of the hazard that a touching body is on: the one it has gone the least far
    /// into. A body that is shallower in than it is wide lands on the top or underside; one
    /// that is deeper in walked in from a side. Exactly level with the center is the top, and
    /// exactly in line with it is the right.
    fn contact(&self, center: Vec2, size: Vec2) -> Contact {
        let across = |low: f32, high: f32, from: f32, to: f32| high.min(to) - low.max(from);
        let (half_x, half_y) = (size.x / 2.0, size.y / 2.0);
        let overlap_x = across(
            center.x - half_x,
            center.x + half_x,
            self.rect.x,
            self.rect.right(),
        );
        let overlap_y = across(
            center.y - half_y,
            center.y + half_y,
            self.rect.y,
            self.rect.bottom(),
        );
        let middle = self.rect.center();
        if overlap_y < overlap_x {
            if center.y <= middle.y {
                Contact::Top
            } else {
                Contact::Bottom
            }
        } else if center.x >= middle.x {
            Contact::Right
        } else {
            Contact::Left
        }
    }

    /// The hit on a body that touches the hazard (`velocity` is how it is moving): the damage
    /// and a throw away from the side it touched. From the top the body ends up moving straight
    /// up at `knockback_speed`, and from below straight down; from a side it is pushed away
    /// sideways for `knockback_secs` and ends up moving up at `lift_speed`. The vertical speed
    /// is set, not added to, so a fall does not eat the throw; a body already moving that way
    /// faster is not slowed.
    pub fn hit(&self, center: Vec2, size: Vec2, velocity: Vec2) -> Option<DamageHit> {
        if !self.touches(center, size) {
            return None;
        }
        let (push, vertical) = match self.contact(center, size) {
            Contact::Top => (0.0, -self.knockback_speed),
            Contact::Bottom => (0.0, self.knockback_speed),
            Contact::Left => (-self.knockback_speed, -self.lift_speed),
            Contact::Right => (self.knockback_speed, -self.lift_speed),
        };
        Some(DamageHit {
            damage: i32::from(self.damage),
            knockback: Vec2::new(push, launch(vertical, velocity.y)),
            knockback_secs: self.knockback_secs,
        })
    }
}

/// What to add to a vertical speed `current` so that it becomes `target`: nothing when the
/// target is zero, or when `current` already goes that way at least as fast.
fn launch(target: f32, current: f32) -> f32 {
    let already_there = if target < 0.0 {
        current <= target
    } else {
        current >= target
    };
    if target == 0.0 || already_there {
        0.0
    } else {
        target - current
    }
}

impl HazardPlacement {
    /// A hazard of the standard size and numbers with its top-left corner at `(x, y)`.
    pub fn standard(x: f32, y: f32) -> Self {
        Self {
            rect: (x, y, NEW_SIZE.0, NEW_SIZE.1),
            damage: NEW_DAMAGE,
            knockback_speed: NEW_KNOCKBACK_SPEED,
            lift_speed: NEW_LIFT_SPEED,
            knockback_secs: NEW_KNOCKBACK_SECS,
            style: HazardStyle::default(),
            facing: HazardFacing::default(),
            color: None,
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.rect.0, self.rect.1, self.rect.2, self.rect.3)
    }

    pub fn fields(&self) -> HazardFields {
        HazardFields {
            width: self.rect.2,
            height: self.rect.3,
            damage: self.damage,
            knockback_speed: self.knockback_speed,
            lift_speed: self.lift_speed,
            knockback_secs: self.knockback_secs,
            style: self.style,
            facing: self.facing,
            color: self.color,
        }
    }

    /// The hazard with `fields` applied; a new size keeps the center line and the bottom edge
    /// (see `resized`).
    pub fn with_fields(&self, fields: HazardFields) -> Self {
        Self {
            damage: fields.damage,
            knockback_speed: fields.knockback_speed,
            lift_speed: fields.lift_speed,
            knockback_secs: fields.knockback_secs,
            style: fields.style,
            facing: fields.facing,
            color: fields.color,
            ..self.resized(fields.width, fields.height)
        }
    }

    /// The same hazard resized to `width` by `height`, keeping its center line and its bottom
    /// edge, so a spike strip grows upward from the floor it stands on.
    pub fn resized(&self, width: f32, height: f32) -> Self {
        let (x, y, w, h) = self.rect;
        Self {
            rect: (x + (w - width) / 2.0, y + h - height, width, height),
            ..self.clone()
        }
    }

    /// Checks one hazard (`index` is only for the message): finite numbers in range, at least
    /// the minimum size, fully inside `room`, a color (if any) between 0 and 1.
    pub fn check(&self, index: usize, room: Rect) -> Result<(), String> {
        let (x, y, w, h) = self.rect;
        if ![x, y, w, h, x + w, y + h].iter().all(|n| n.is_finite()) {
            return Err(format!("hazard {index} must have finite numbers"));
        }
        if w < MIN_HAZARD_SIZE || h < MIN_HAZARD_SIZE {
            return Err(format!(
                "hazard {index} must be at least {MIN_HAZARD_SIZE} px wide and tall"
            ));
        }
        if x < room.x || y < room.y || x + w > room.right() || y + h > room.bottom() {
            return Err(format!("hazard {index} must be inside the room bounds"));
        }
        if !(1..=MAX_HAZARD_DAMAGE).contains(&self.damage) {
            return Err(format!(
                "hazard {index} damage must be between 1 and {MAX_HAZARD_DAMAGE}"
            ));
        }
        for (label, value, max) in [
            ("knockback speed", self.knockback_speed, MAX_HAZARD_SPEED),
            ("lift speed", self.lift_speed, MAX_HAZARD_SPEED),
            (
                "knockback time",
                self.knockback_secs,
                MAX_HAZARD_KNOCKBACK_SECS,
            ),
        ] {
            if !(0.0..=max).contains(&value) {
                return Err(format!(
                    "hazard {index} {label} must be between 0 and {max}"
                ));
            }
        }
        if let Some((red, green, blue)) = self.color {
            if ![red, green, blue].iter().all(|c| (0.0..=1.0).contains(c)) {
                return Err(format!("hazard {index} color must be between 0 and 1"));
            }
        }
        Ok(())
    }
}

impl RoomLayout {
    /// Every hazard is valid and none overlaps the spawn point (the player would be hurt the
    /// moment the room starts).
    pub(crate) fn validate_hazards(&self) -> Result<(), String> {
        for (index, placement) in self.hazards.iter().enumerate() {
            placement.check(index, self.rect())?;
            if Hazard::from(placement).touches(self.spawn.into(), SPAWN_BODY) {
                return Err(format!("spawn must not be inside hazard {index}"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
