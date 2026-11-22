//! `min_tier` on enemy and boss placements: the file format and what the loaders below the
//! game do with it. The game's own world loading is the only place a placement is left out;
//! these tests are about everything else keeping it.
mod support;

use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::room::{
    load_rooms, test_rooms, BossPlacement, EnemyPlacement, RoomEnemies, RoomId,
};

fn catalog() -> Vec<(RoomId, RoomLayout)> {
    test_rooms()
        .iter()
        .map(|room| (room.id, RoomLayout::from_room(room)))
        .collect()
}

fn draft(source: &str) -> RoomDraft {
    let document: RoomEnemies = ron::from_str(source).unwrap();
    RoomDraft::from_document(document, &test_rooms()[0])
}

fn serialized(draft: &RoomDraft) -> String {
    let definitions = EnemyDefinitions::load(&support::fixture_assets()).unwrap();
    draft.serialize(&definitions, &catalog()).unwrap()
}

const WITH_TIERS: &str = r#"(
    enemies: [
        (definition: "chase", patrol_left: 560.0, patrol_right: 800.0),
        (definition: "chase", patrol_left: 300.0, patrol_right: 400.0, min_tier: 2),
    ],
    boss: Some((definition: "Ash Warden", position: (560.0, 436.0), min_tier: 5)),
)"#;

#[test]
fn a_min_tier_is_written_and_read_back_on_an_enemy_and_a_boss() {
    let draft = draft(WITH_TIERS);
    assert_eq!(draft.enemies[0].min_tier, 0);
    assert_eq!(draft.enemies[1].min_tier, 2);
    assert_eq!(draft.boss.as_ref().unwrap().min_tier, 5);

    let source = serialized(&draft);
    assert_eq!(source.matches("min_tier").count(), 2, "{source}");
    assert!(source.contains("min_tier: 2") && source.contains("min_tier: 5"));

    let document: RoomEnemies = ron::from_str(&source).unwrap();
    assert_eq!(document.enemies, draft.enemies);
    assert_eq!(document.boss, draft.boss);
}

#[test]
fn a_placement_at_every_tier_writes_no_min_tier() {
    let mut draft = draft(WITH_TIERS);
    draft.enemies[1].min_tier = 0;
    draft.boss.as_mut().unwrap().min_tier = 0;
    let source = serialized(&draft);
    assert!(!source.contains("min_tier"), "{source}");
    let document: RoomEnemies = ron::from_str(&source).unwrap();
    assert_eq!(document.enemies, draft.enemies);
}

#[test]
fn a_room_file_from_before_min_tier_loads_as_shown_at_every_tier() {
    let old = r#"(
        enemies: [(definition: "chase", patrol_left: 560.0, patrol_right: 800.0)],
        boss: Some((definition: "Ash Warden", position: (560.0, 436.0))),
    )"#;
    let document: RoomEnemies = ron::from_str(old).unwrap();
    assert_eq!(document.enemies[0].min_tier, 0);
    assert_eq!(document.boss.unwrap().min_tier, 0);
    assert_eq!(
        EnemyPlacement::new("chase", 560.0, 800.0).min_tier,
        0,
        "the constructor is the every-tier placement"
    );
    assert_eq!(BossPlacement::new("Ash Warden", (560.0, 436.0)).min_tier, 0);
}

#[test]
fn a_min_tier_above_the_highest_tier_is_an_error_naming_it() {
    let enemy = |tier: u32| {
        format!(
            r#"(enemies: [(definition: "chase", patrol_left: 560.0, patrol_right: 800.0, min_tier: {tier})])"#
        )
    };
    assert!(ron::from_str::<RoomEnemies>(&enemy(99)).is_ok());
    let error = ron::from_str::<RoomEnemies>(&enemy(100))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("min_tier 100") && error.contains("99"),
        "{error}"
    );
    let boss = r#"(enemies: [], boss: Some((definition: "Ash Warden", position: (560.0, 436.0), min_tier: 100)))"#;
    assert!(ron::from_str::<RoomEnemies>(boss).is_err());
    assert!(
        ron::from_str::<RoomEnemies>(&enemy(0).replace("min_tier: 0", "min_tier: -1")).is_err()
    );
}

#[test]
fn a_misspelt_min_tier_is_still_an_error() {
    let typo = r#"(enemies: [(definition: "chase", patrol_left: 560.0, patrol_right: 800.0, mintier: 2)])"#;
    assert!(ron::from_str::<RoomEnemies>(typo).is_err());
}

#[test]
fn building_a_draft_keeps_each_placements_min_tier_on_the_live_objects() {
    let draft = draft(WITH_TIERS);
    let definitions = EnemyDefinitions::load(&support::fixture_assets()).unwrap();
    let room = draft.build(RoomId::First, &definitions).unwrap();
    let tiers: Vec<_> = room.enemies.iter().map(|enemy| enemy.min_tier()).collect();
    assert_eq!(
        tiers,
        [0, 2],
        "the draft is built whole: the editor sees every placement"
    );
}

#[test]
fn the_loaders_below_the_game_leave_every_placement_in_the_room() {
    // A scratch copy of the fixtures whose first room holds one placement shown from tier 2.
    let dir = std::env::temp_dir().join(format!("hollow-steps-min-tier-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&support::fixture_assets(), &dir);
    std::fs::write(
        dir.join("rooms/first.ron"),
        WITH_TIERS.replace("min_tier: 5", "min_tier: 9"),
    )
    .unwrap();

    let [first, _, _] = load_rooms(&dir).unwrap();
    assert_eq!(
        first.enemies.len(),
        2,
        "load_rooms does not know about tiers"
    );
    assert_eq!(first.enemies[1].min_tier(), 2);
    assert_eq!(first.boss_special.as_ref().unwrap().min_tier(), 9);
    assert!(
        first.exits_locked(),
        "a boss that is in the room locks its exits"
    );

    let definitions = EnemyDefinitions::load(&dir).unwrap();
    let extra = hollow_steps::editor::load_extra_rooms(&dir, &definitions).unwrap();
    assert!(extra
        .iter()
        .all(|room| room.enemies.iter().all(|e| e.min_tier() == 0)));
    let _ = std::fs::remove_dir_all(&dir);
}

fn copy(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
