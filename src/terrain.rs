//! Solid rectangles that block the player. Pure rules, no rendering.
use macroquad::math::{Rect, Vec2};

/// Overlaps shallower than this are treated as touching, so float rounding
/// never snags a player sliding along a surface.
const TOUCH_EPSILON: f32 = 0.001;
/// How far from a solid's top still counts as standing on it.
const SUPPORT_EPSILON: f32 = 0.01;
/// Largest per-axis displacement processed in one call; guards huge deltas.
const MAX_DISPLACEMENT: f32 = 20_000.0;

/// What a move ran into.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Contacts {
    /// Blocked by a wall while moving horizontally.
    pub wall: bool,
    /// Landed on a solid top while moving down.
    pub floor: bool,
    /// Hit a solid underside while moving up.
    pub ceiling: bool,
}

/// Room limits applied inside every substep so fast moves cannot slip past
/// the floor or side walls and then around a solid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub left: f32,
    pub right: f32,
    /// Top edge of the floor.
    pub floor_y: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Terrain {
    solids: Vec<Rect>,
}

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x < b.right() - TOUCH_EPSILON
        && a.right() > b.x + TOUCH_EPSILON
        && a.y < b.bottom() - TOUCH_EPSILON
        && a.bottom() > b.y + TOUCH_EPSILON
}

fn body(center: Vec2, size: Vec2) -> Rect {
    Rect::new(
        center.x - size.x / 2.0,
        center.y - size.y / 2.0,
        size.x,
        size.y,
    )
}

impl Terrain {
    pub fn new(solids: Vec<Rect>) -> Self {
        Self { solids }
    }

    /// Builds terrain from authored `(x, y, width, height)` tuples.
    pub fn from_tuples(tuples: &[(f32, f32, f32, f32)]) -> Self {
        Self::new(
            tuples
                .iter()
                .map(|&(x, y, w, h)| Rect::new(x, y, w, h))
                .collect(),
        )
    }

    pub fn solids(&self) -> &[Rect] {
        &self.solids
    }

    pub fn is_empty(&self) -> bool {
        self.solids.is_empty()
    }

    /// Checks authored solids: finite, positive size, fully inside `room`.
    pub fn validate(tuples: &[(f32, f32, f32, f32)], room: Rect) -> Result<(), String> {
        for (index, &(x, y, w, h)) in tuples.iter().enumerate() {
            if ![x, y, w, h, x + w, y + h].iter().all(|n| n.is_finite()) || w <= 0.0 || h <= 0.0 {
                return Err(format!("terrain {index} must be finite with positive size"));
            }
            if x < room.x || y < room.y || x + w > room.right() || y + h > room.bottom() {
                return Err(format!("terrain {index} must be inside the room bounds"));
            }
        }
        Ok(())
    }

    /// True when a rectangle overlaps any solid.
    pub fn overlaps_rect(&self, rect: &Rect) -> bool {
        self.solids.iter().any(|solid| overlaps(solid, rect))
    }

    /// True when a body centered at `center` overlaps any solid.
    pub fn overlaps_body(&self, center: Vec2, size: Vec2) -> bool {
        self.overlaps_rect(&body(center, size))
    }

    /// Top edge of the highest solid the body is standing on, if any.
    pub fn support_top(&self, center: Vec2, size: Vec2) -> Option<f32> {
        let rect = body(center, size);
        self.solids
            .iter()
            .filter(|solid| {
                rect.x < solid.right() - TOUCH_EPSILON
                    && rect.right() > solid.x + TOUCH_EPSILON
                    && (rect.bottom() - solid.y).abs() <= SUPPORT_EPSILON
            })
            .map(|solid| solid.y)
            .reduce(f32::min)
    }

    /// Moves a body by `displacement`, resolving x then y against solids.
    /// Long moves are split into steps of at most half the body's smaller
    /// side so fast movement cannot tunnel through a thin solid.
    pub fn move_body(
        &self,
        center: &mut Vec2,
        size: Vec2,
        displacement: Vec2,
        limits: Limits,
    ) -> Contacts {
        let mut contacts = Contacts::default();
        let displacement = Vec2::new(
            if displacement.x.is_finite() {
                displacement.x.clamp(-MAX_DISPLACEMENT, MAX_DISPLACEMENT)
            } else {
                0.0
            },
            if displacement.y.is_finite() {
                displacement.y.clamp(-MAX_DISPLACEMENT, MAX_DISPLACEMENT)
            } else {
                0.0
            },
        );
        let max_step = (size.x.min(size.y) / 2.0).max(1.0);
        let steps = (displacement.x.abs().max(displacement.y.abs()) / max_step)
            .ceil()
            .max(1.0);
        let step = displacement / steps;
        for _ in 0..steps as u32 {
            center.x += step.x;
            if step.x != 0.0 {
                for solid in &self.solids {
                    if !overlaps(solid, &body(*center, size)) {
                        continue;
                    }
                    center.x = if step.x > 0.0 {
                        solid.x - size.x / 2.0
                    } else {
                        solid.right() + size.x / 2.0
                    };
                    contacts.wall = true;
                }
                let (min_x, max_x) = (limits.left + size.x / 2.0, limits.right - size.x / 2.0);
                if center.x < min_x || center.x > max_x {
                    center.x = center.x.clamp(min_x, max_x);
                    contacts.wall = true;
                }
            }
            center.y += step.y;
            if step.y != 0.0 {
                for solid in &self.solids {
                    if !overlaps(solid, &body(*center, size)) {
                        continue;
                    }
                    if step.y > 0.0 {
                        center.y = solid.y - size.y / 2.0;
                        contacts.floor = true;
                    } else {
                        center.y = solid.bottom() + size.y / 2.0;
                        contacts.ceiling = true;
                    }
                }
                let floor_center = limits.floor_y - size.y / 2.0;
                if step.y > 0.0 && center.y > floor_center {
                    center.y = floor_center;
                    contacts.floor = true;
                }
            }
        }
        contacts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SIZE: Vec2 = Vec2::new(32.0, 48.0);
    const LIMITS: Limits = Limits {
        left: 0.0,
        right: 960.0,
        floor_y: 460.0,
    };

    #[test]
    fn validate_rejects_bad_solids() {
        let room = Rect::new(0.0, 0.0, 960.0, 460.0);
        assert!(Terrain::validate(&[(10.0, 10.0, 50.0, 20.0)], room).is_ok());
        assert!(Terrain::validate(&[(10.0, 10.0, 0.0, 20.0)], room).is_err());
        assert!(Terrain::validate(&[(10.0, 10.0, -5.0, 20.0)], room).is_err());
        assert!(Terrain::validate(&[(f32::NAN, 10.0, 5.0, 20.0)], room).is_err());
        assert!(Terrain::validate(&[(950.0, 10.0, 20.0, 20.0)], room).is_err());
        assert!(Terrain::validate(&[(10.0, -1.0, 20.0, 20.0)], room).is_err());
    }

    #[test]
    fn touching_is_not_overlap_and_support_is_found() {
        let terrain = Terrain::from_tuples(&[(100.0, 300.0, 100.0, 20.0)]);
        let standing = Vec2::new(150.0, 300.0 - 24.0);
        assert!(!terrain.overlaps_body(standing, SIZE));
        assert_eq!(terrain.support_top(standing, SIZE), Some(300.0));
        assert_eq!(terrain.support_top(Vec2::new(400.0, 276.0), SIZE), None);
    }

    #[test]
    fn move_body_never_tunnels() {
        let terrain = Terrain::from_tuples(&[(100.0, 300.0, 100.0, 4.0)]);
        let mut pos = Vec2::new(150.0, 100.0);
        let contacts = terrain.move_body(&mut pos, SIZE, Vec2::new(0.0, 5_000.0), LIMITS);
        assert!(contacts.floor);
        assert!((pos.y - (300.0 - 24.0)).abs() < 0.01);
    }

    #[test]
    fn dash_stops_at_a_wall() {
        use crate::item::Item;
        use crate::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
        let bounds = WorldBounds::new(0.0, 960.0, 460.0);
        let terrain = Terrain::from_tuples(&[(400.0, 300.0, 20.0, 160.0)]);
        let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
        player.collect_item(Item::Dash);
        player.place_on_floor(300.0, bounds);
        let dash = PlayerInput {
            dash_pressed: true,
            ..PlayerInput::default()
        };
        player.update_with_terrain(dash, 0.05, bounds, &terrain);
        for _ in 0..30 {
            player.update_with_terrain(PlayerInput::default(), 0.05, bounds, &terrain);
        }
        assert!((player.position().x - (400.0 - 16.0)).abs() < 0.01);
        // A single huge dash frame must not tunnel either.
        player.place_on_floor(300.0, bounds);
        player.update_with_terrain(dash, 0.2, bounds, &terrain);
        assert!(player.position().x <= 400.0 - 16.0 + 0.01);
    }

    #[test]
    fn non_finite_displacement_is_ignored() {
        let terrain = Terrain::default();
        let mut pos = Vec2::new(1.0, 2.0);
        terrain.move_body(&mut pos, SIZE, Vec2::new(f32::NAN, f32::INFINITY), LIMITS);
        assert!(pos.is_finite());
    }
}
