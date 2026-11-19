//! Projectiles: continuous collision against the room and the player.
use macroquad::math::Vec2;

use super::attack_box::DamageHit;

#[derive(Clone, Debug, PartialEq)]
pub struct Projectile {
    pub position: Vec2,
    pub velocity: Vec2,
    pub size: Vec2,
    pub damage: i32,
    pub remaining_secs: f64,
}
impl Projectile {
    /// Continuous collision against the room and player, limited to the live portion of the step.
    pub fn step(
        &mut self,
        dt: f64,
        bounds: crate::player::WorldBounds,
        player: Vec2,
        player_size: Vec2,
    ) -> (bool, Option<DamageHit>) {
        if !dt.is_finite() || dt < 0.0 {
            return (self.remaining_secs > 0.0, None);
        }
        let min = Vec2::new(bounds.left, bounds.top) + self.size / 2.0;
        let max = Vec2::new(bounds.right, bounds.floor_y) - self.size / 2.0;
        if self.position.x < min.x
            || self.position.x > max.x
            || self.position.y < min.y
            || self.position.y > max.y
            || self.remaining_secs <= 0.0
        {
            return (false, None);
        }
        let mut wall_time = f64::INFINITY;
        for (p, v, lo, hi) in [
            (self.position.x, self.velocity.x, min.x, max.x),
            (self.position.y, self.velocity.y, min.y, max.y),
        ] {
            if v != 0.0 {
                wall_time = wall_time
                    .min((f64::from(if v > 0.0 { hi } else { lo }) - f64::from(p)) / f64::from(v));
            }
        }
        let travel = dt.min(self.remaining_secs).min(wall_time.max(0.0));
        let end = (self.position.as_dvec2() + self.velocity.as_dvec2() * travel).as_vec2();
        let half = (player_size + self.size) / 2.0;
        let hit = segment_rect_fraction(self.position, end, player - half, player + half).filter(
            |fraction| fraction * travel < self.remaining_secs && fraction * travel < wall_time,
        );
        self.position = end;
        self.remaining_secs = (self.remaining_secs - dt).max(0.0);
        if hit.is_some() {
            (
                false,
                Some(DamageHit {
                    damage: self.damage,
                    knockback: Vec2::ZERO,
                    knockback_secs: 0.0,
                }),
            )
        } else {
            (dt < wall_time && self.remaining_secs > 0.0, None)
        }
    }
}
fn segment_rect_fraction(start: Vec2, end: Vec2, min: Vec2, max: Vec2) -> Option<f64> {
    let mut enter: f64 = 0.0;
    let mut exit: f64 = 1.0;
    for (p, q, lo, hi) in [
        (start.x, end.x, min.x, max.x),
        (start.y, end.y, min.y, max.y),
    ] {
        let delta = f64::from(q) - f64::from(p);
        if delta == 0.0 {
            if p < lo || p > hi {
                return None;
            }
        } else {
            let a = (f64::from(lo) - f64::from(p)) / delta;
            let b = (f64::from(hi) - f64::from(p)) / delta;
            enter = enter.max(a.min(b));
            exit = exit.min(a.max(b));
            if enter > exit {
                return None;
            }
        }
    }
    Some(enter)
}
