//! An enemy's health at a difficulty tier (`crate::difficulty`). Layered on when the enemy is
//! spawned, never written back to a definition or room file.
use super::Enemy;
use crate::difficulty::TierScale;

impl Enemy {
    /// Scales the enemy's maximum and current health to a tier: rounded, never below 1. Meant for
    /// a freshly spawned enemy, so the two stay equal.
    pub(crate) fn scale_health(&mut self, scale: &TierScale) {
        self.max_health = scale.enemy_health(self.max_health);
        self.health = scale.enemy_health(self.health);
    }

    /// The enemy with its health scaled to a tier (see `scale_health`).
    pub(crate) fn scaled(mut self, scale: &TierScale) -> Self {
        self.scale_health(scale);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::difficulty::TierScale;
    use crate::player::WorldBounds;

    const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

    fn scale(enemy_health: f64) -> TierScale {
        TierScale {
            enemy_health,
            ..TierScale::NEUTRAL
        }
    }

    #[test]
    fn health_and_maximum_health_are_scaled_together_and_rounded() {
        let mut enemy = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS);
        let base = enemy.max_health();
        assert_eq!(enemy.health(), base);
        enemy.scale_health(&scale(1.3));
        assert_eq!(enemy.max_health(), (f64::from(base) * 1.3).round() as u16);
        assert_eq!(enemy.health(), enemy.max_health());
        assert!(!enemy.is_dead());
    }

    #[test]
    fn a_neutral_scale_changes_nothing() {
        let plain = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS);
        let scaled = plain.clone().scaled(&TierScale::NEUTRAL);
        assert_eq!(scaled.health(), plain.health());
        assert_eq!(scaled.max_health(), plain.max_health());
    }

    #[test]
    fn an_enemy_stays_alive_and_a_dead_one_stays_dead() {
        let mut weak = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS);
        weak.scale_health(&scale(0.0001));
        assert_eq!(weak.health(), 1, "never below 1");
        let mut dead = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS);
        assert!(dead.hit_for(1, 1.0, u16::MAX));
        assert!(dead.is_dead());
        dead.scale_health(&scale(2.0));
        assert!(dead.is_dead(), "nothing to scale: it stays at 0");
    }

    #[test]
    fn health_saturates_at_what_an_enemy_can_hold() {
        let mut enemy = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS);
        enemy.scale_health(&scale(10_000.0));
        assert_eq!(enemy.max_health(), u16::MAX);
        assert_eq!(enemy.health(), u16::MAX);
    }
}
