use macroquad::math::Vec2;

use crate::player::WorldBounds;

pub const ENEMY_SIZE: Vec2 = Vec2::new(36.0, 36.0);

/// A floor-bound enemy. Patrol endpoints and position are rectangle centers.
#[derive(Clone, Debug)]
pub struct Enemy {
    position: Vec2,
    patrol_left: f32,
    patrol_right: f32,
    direction: f32,
    speed: f32,
    detection_radius: f32,
}

impl Enemy {
    pub fn new(
        left: f32,
        right: f32,
        speed: f32,
        detection_radius: f32,
        bounds: WorldBounds,
    ) -> Self {
        assert!(left.is_finite() && right.is_finite() && left <= right);
        assert!(left >= bounds.left + ENEMY_SIZE.x / 2.0);
        assert!(right <= bounds.right - ENEMY_SIZE.x / 2.0);
        assert!(speed.is_finite() && speed >= 0.0);
        assert!(detection_radius.is_finite() && detection_radius >= 0.0);
        Self {
            position: Vec2::new(left, bounds.floor_y - ENEMY_SIZE.y / 2.0),
            patrol_left: left,
            patrol_right: right,
            direction: 1.0,
            speed,
            detection_radius,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn size(&self) -> Vec2 {
        ENEMY_SIZE
    }

    pub fn detection_radius(&self) -> f32 {
        self.detection_radius
    }

    pub fn update(&mut self, player_position: Vec2, delta_seconds: f32, bounds: WorldBounds) {
        let dt = if delta_seconds.is_finite() {
            delta_seconds.max(0.0)
        } else {
            0.0
        };
        let mut travel = self.speed as f64 * dt as f64;
        if self.position.distance(player_position) < self.detection_radius {
            let difference = player_position.x - self.position.x;
            if difference != 0.0 {
                self.direction = difference.signum();
                self.position.x += self.direction * travel.min(difference.abs() as f64) as f32;
            }
            self.position.x = self.position.x.clamp(
                bounds.left + ENEMY_SIZE.x / 2.0,
                bounds.right - ENEMY_SIZE.x / 2.0,
            );
        } else {
            // Walk back to the interval after chasing; never teleport onto it.
            let nearest = self.position.x.clamp(self.patrol_left, self.patrol_right);
            let difference = nearest - self.position.x;
            if difference != 0.0 {
                self.direction = difference.signum();
                let step = travel.min(difference.abs() as f64);
                self.position.x += self.direction * step as f32;
                travel -= step;
            }
            if self.position.x >= self.patrol_left && self.position.x <= self.patrol_right {
                let width = (self.patrol_right - self.patrol_left) as f64;
                if width > 0.0 {
                    let offset = (self.position.x - self.patrol_left) as f64;
                    let phase = if self.direction > 0.0 {
                        offset
                    } else {
                        2.0 * width - offset
                    };
                    let phase = (phase + travel) % (2.0 * width);
                    self.direction = if phase < width { 1.0 } else { -1.0 };
                    self.position.x = self.patrol_left
                        + if phase < width {
                            phase
                        } else {
                            2.0 * width - phase
                        } as f32;
                }
            }
        }
        self.position.y = bounds.floor_y - ENEMY_SIZE.y / 2.0;
    }

    pub fn touches(&self, position: Vec2, size: Vec2) -> bool {
        let separation = (self.position - position).abs();
        let reach = (ENEMY_SIZE + size) / 2.0;
        separation.x <= reach.x && separation.y <= reach.y
    }
}
