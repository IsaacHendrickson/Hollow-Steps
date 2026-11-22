//! A boss's health at a difficulty tier (`crate::difficulty`). Layered on when the boss is
//! placed, never written back to a boss definition or room file.
use super::BossSpecials;
use crate::difficulty::TierScale;

impl BossSpecials {
    /// Scales the boss's maximum and current health to a tier: rounded, never below 1. Meant for
    /// a freshly placed boss, so the two stay equal; the definition itself is not changed.
    pub(crate) fn scale_health(&mut self, scale: &TierScale) {
        self.max_health = scale.boss_health(self.max_health);
        self.health = scale.boss_health(self.health);
    }

    /// The lowest tier this boss is in the room at; 0 means every tier. It comes from the
    /// placement's `min_tier` and only the game's world loading acts on it.
    pub fn min_tier(&self) -> u32 {
        self.min_tier
    }

    /// The boss marked as appearing from `min_tier` on.
    pub fn with_min_tier(mut self, min_tier: u32) -> Self {
        self.min_tier = min_tier;
        self
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use macroquad::math::Vec2;

    use super::*;
    use crate::boss_definition::BossDefinition;
    use crate::difficulty::TierScale;

    fn boss() -> BossSpecials {
        let definition =
            BossDefinition::from_ron(include_str!("../../tests/fixtures/bosses/ground_pound.ron"))
                .unwrap();
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(560.0, 436.0)).unwrap()
    }

    fn scale(boss_health: f64) -> TierScale {
        TierScale {
            boss_health,
            ..TierScale::NEUTRAL
        }
    }

    #[test]
    fn health_and_maximum_health_are_scaled_together_and_the_definition_is_not() {
        let mut boss = boss();
        let base = boss.max_health();
        assert_eq!(boss.health(), base);
        boss.scale_health(&scale(1.36));
        assert_eq!(boss.max_health(), (f64::from(base) * 1.36).round() as u32);
        assert_eq!(boss.health(), boss.max_health());
        assert!(!boss.is_defeated());
        assert!(
            boss.same_definition(&self::boss()),
            "the file's numbers stay"
        );
    }

    #[test]
    fn a_neutral_scale_changes_nothing_and_health_never_drops_below_one() {
        let mut plain = boss();
        plain.scale_health(&TierScale::NEUTRAL);
        assert_eq!(plain.health(), self::boss().health());
        let mut weak = boss();
        weak.scale_health(&scale(0.0001));
        assert_eq!(weak.health(), 1);
        assert!(!weak.is_defeated());
    }

    #[test]
    fn a_scaled_boss_needs_the_scaled_amount_of_damage() {
        let mut boss = boss();
        boss.scale_health(&scale(2.0));
        let base = self::boss().max_health();
        boss.damage_boss(base);
        assert!(!boss.is_defeated(), "the old health is no longer enough");
        boss.damage_boss(base);
        assert!(boss.is_defeated());
    }
}
