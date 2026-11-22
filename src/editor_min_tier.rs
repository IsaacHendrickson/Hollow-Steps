//! The room editor's edits of a placement's `min_tier` (`crate::room::EnemyPlacement`,
//! `BossPlacement`): the lowest difficulty tier the enemy or the boss is in the room at. The
//! draft always holds every placement, so a hidden one is still there to select and edit, and
//! Save writes it back as it was; only the game's world loading leaves it out of play.
use crate::editor::RoomDraft;
use crate::room::MAX_MIN_TIER;

fn check(tier: u32) -> Result<(), String> {
    if tier > MAX_MIN_TIER {
        return Err(format!("min tier must be at most {MAX_MIN_TIER}"));
    }
    Ok(())
}

impl RoomDraft {
    /// The min tier of enemy placement `index`; `None` when there is none.
    pub fn enemy_min_tier(&self, index: usize) -> Option<u32> {
        self.enemies.get(index).map(|enemy| enemy.min_tier)
    }

    /// The min tier of the room's boss placement; `None` when the room has no boss.
    pub fn boss_min_tier(&self) -> Option<u32> {
        self.boss.as_ref().map(|boss| boss.min_tier)
    }

    /// Sets the min tier of enemy placement `index`; refused, with the draft unchanged, above
    /// `MAX_MIN_TIER`.
    pub fn set_enemy_min_tier(&mut self, index: usize, tier: u32) -> Result<(), String> {
        check(tier)?;
        self.enemies.get_mut(index).ok_or("missing enemy")?.min_tier = tier;
        Ok(())
    }

    /// Sets the min tier of the room's boss; refused, with the draft unchanged, when the room
    /// has no boss or the tier is above `MAX_MIN_TIER`.
    pub fn set_boss_min_tier(&mut self, tier: u32) -> Result<(), String> {
        check(tier)?;
        self.boss.as_mut().ok_or("missing boss")?.min_tier = tier;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::{BossPlacement, EnemyPlacement, RoomEnemies};

    fn draft() -> RoomDraft {
        let [room, ..] = crate::room::test_rooms();
        RoomDraft::from_document(
            RoomEnemies {
                enemies: vec![
                    EnemyPlacement::new("chase", 560.0, 800.0),
                    EnemyPlacement::new("chase", 300.0, 400.0),
                ],
                boss: Some(BossPlacement::new("Ash Warden", (560.0, 436.0))),
                layout: None,
            },
            &room,
        )
    }

    #[test]
    fn an_enemy_min_tier_is_read_and_set_on_that_placement_only() {
        let mut draft = draft();
        assert_eq!(draft.enemy_min_tier(1), Some(0));
        draft.set_enemy_min_tier(1, 3).unwrap();
        assert_eq!(draft.enemy_min_tier(0), Some(0));
        assert_eq!(draft.enemy_min_tier(1), Some(3));
        assert_eq!(draft.boss_min_tier(), Some(0));
        assert_eq!(draft.enemy_min_tier(2), None);
    }

    #[test]
    fn the_boss_min_tier_is_read_and_set() {
        let mut draft = draft();
        draft.set_boss_min_tier(7).unwrap();
        assert_eq!(draft.boss_min_tier(), Some(7));
        assert_eq!(draft.enemy_min_tier(0), Some(0));
    }

    #[test]
    fn the_highest_tier_is_accepted_and_one_more_is_refused_with_the_draft_unchanged() {
        let mut draft = draft();
        draft.set_enemy_min_tier(0, MAX_MIN_TIER).unwrap();
        draft.set_boss_min_tier(MAX_MIN_TIER).unwrap();
        let before = draft.clone();
        assert!(draft
            .set_enemy_min_tier(0, MAX_MIN_TIER + 1)
            .unwrap_err()
            .contains("at most 99"));
        assert!(draft.set_boss_min_tier(MAX_MIN_TIER + 1).is_err());
        assert!(draft.set_enemy_min_tier(0, u32::MAX).is_err());
        assert_eq!(draft, before);
    }

    #[test]
    fn a_missing_enemy_or_boss_is_an_error_and_changes_nothing() {
        let mut draft = draft();
        draft.boss = None;
        let before = draft.clone();
        assert!(draft
            .set_enemy_min_tier(5, 1)
            .unwrap_err()
            .contains("enemy"));
        assert!(draft.set_boss_min_tier(1).unwrap_err().contains("boss"));
        assert_eq!(draft.boss_min_tier(), None);
        assert_eq!(draft, before);
    }

    #[test]
    fn a_min_tier_survives_moving_and_patrolling_the_enemy() {
        let mut draft = draft();
        draft.set_enemy_min_tier(0, 4).unwrap();
        let definitions = crate::enemy::EnemyDefinitions::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
        )
        .unwrap();
        draft
            .move_to(
                crate::editor::Selection::Enemy(0),
                macroquad::math::Vec2::new(620.0, 436.0),
                &definitions,
            )
            .unwrap();
        assert_eq!(draft.enemy_min_tier(0), Some(4));
    }
}
