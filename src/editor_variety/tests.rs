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
fn a_placement_starts_in_no_group_with_full_chance() {
    let draft = draft();
    assert_eq!(draft.enemy_group(0), Some(None));
    assert_eq!(draft.enemy_chance(0), Some(100));
    assert_eq!(draft.enemy_group(2), None);
    assert_eq!(draft.enemy_chance(2), None);
}

#[test]
fn a_group_is_set_and_cleared_on_that_placement_only() {
    let mut draft = draft();
    draft.set_enemy_group(1, Some(3)).unwrap();
    assert_eq!(draft.enemy_group(0), Some(None));
    assert_eq!(draft.enemy_group(1), Some(Some(3)));
    draft.set_enemy_group(1, None).unwrap();
    assert_eq!(draft.enemy_group(1), Some(None));
}

#[test]
fn a_chance_is_set_on_that_placement_only() {
    let mut draft = draft();
    draft.set_enemy_chance(0, 25).unwrap();
    assert_eq!(draft.enemy_chance(0), Some(25));
    assert_eq!(draft.enemy_chance(1), Some(100));
    draft.set_enemy_chance(0, 0).unwrap();
    assert_eq!(draft.enemy_chance(0), Some(0));
}

#[test]
fn the_ends_of_each_range_are_accepted_and_one_more_is_refused_with_the_draft_unchanged() {
    let mut draft = draft();
    draft.set_enemy_group(0, Some(1)).unwrap();
    draft.set_enemy_group(0, Some(MAX_GROUP_FOR_TEST)).unwrap();
    draft.set_enemy_chance(1, 100).unwrap();
    draft.set_enemy_chance(1, 0).unwrap();
    let before = draft.clone();
    assert!(draft
        .set_enemy_group(0, Some(0))
        .unwrap_err()
        .contains("1 to 9"));
    assert!(draft.set_enemy_group(0, Some(10)).is_err());
    assert!(draft.set_enemy_group(0, Some(u8::MAX)).is_err());
    assert!(draft.set_enemy_chance(1, 101).unwrap_err().contains("101"));
    assert!(draft.set_enemy_chance(1, u8::MAX).is_err());
    assert_eq!(draft, before);
}

const MAX_GROUP_FOR_TEST: u8 = 9;

#[test]
fn a_grouped_placement_must_have_chance_100_in_both_directions() {
    let mut draft = draft();
    draft.set_enemy_chance(0, 50).unwrap();
    let before = draft.clone();
    let error = draft.set_enemy_group(0, Some(2)).unwrap_err();
    assert!(
        error.contains("chance 50") && error.contains("group"),
        "{error}"
    );
    assert_eq!(draft, before, "refused, unchanged");

    draft.set_enemy_chance(0, 100).unwrap();
    draft.set_enemy_group(0, Some(2)).unwrap();
    let grouped = draft.clone();
    let error = draft.set_enemy_chance(0, 99).unwrap_err();
    assert!(error.contains("group 2"), "{error}");
    assert_eq!(draft, grouped);
    draft.set_enemy_chance(0, 100).unwrap();
}

#[test]
fn a_missing_placement_is_an_error_and_changes_nothing() {
    let mut draft = draft();
    let before = draft.clone();
    assert!(draft
        .set_enemy_group(9, Some(1))
        .unwrap_err()
        .contains("enemy"));
    assert!(draft.set_enemy_chance(9, 5).unwrap_err().contains("enemy"));
    assert!(draft.set_enemy_group(9, None).is_err());
    assert_eq!(draft, before);
}

#[test]
fn the_draft_check_names_the_placement_that_breaks_a_rule() {
    let mut draft = draft();
    assert!(draft.validate_variety().is_ok());
    // A draft read from a file can break the rule the setters keep.
    draft.enemies[1].group = Some(4);
    draft.enemies[1].chance = 30;
    let error = draft.validate_variety().unwrap_err();
    assert!(
        error.contains("enemy 1") && error.contains("chase"),
        "{error}"
    );
    assert!(
        error.contains("group 4") && error.contains("chance 100"),
        "{error}"
    );
}

#[test]
fn group_and_chance_survive_moving_and_patrolling_the_enemy() {
    let mut draft = draft();
    draft.set_enemy_group(0, Some(5)).unwrap();
    draft.set_enemy_chance(1, 40).unwrap();
    let definitions = crate::enemy::EnemyDefinitions::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
    )
    .unwrap();
    for index in 0..2 {
        draft
            .move_to(
                crate::editor::Selection::Enemy(index),
                macroquad::math::Vec2::new(620.0, 436.0),
                &definitions,
            )
            .unwrap();
    }
    assert_eq!(draft.enemy_group(0), Some(Some(5)));
    assert_eq!(draft.enemy_chance(1), Some(40));
}
