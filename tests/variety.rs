//! `group` and `chance` on enemy placements: the file format, the rules a file or a draft must
//! keep, and what the loaders below the game do with them. The game's own world loading, for a
//! run with a seed, is the only place a placement is left out; these tests are about everything
//! else keeping every placement (`src/game/variety_tests.rs` covers the game).
mod support;

use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::room::{load_rooms, test_rooms, EnemyPlacement, RoomEnemies, RoomId};

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

fn definitions() -> EnemyDefinitions {
    EnemyDefinitions::load(&support::fixture_assets()).unwrap()
}

fn serialized(draft: &RoomDraft) -> Result<String, String> {
    draft.serialize(&definitions(), &catalog())
}

fn enemy(extra: &str) -> String {
    format!(r#"(enemies: [(definition: "chase", patrol_left: 560.0, patrol_right: 800.0{extra})])"#)
}

const WITH_VARIETY: &str = r#"(
    enemies: [
        (definition: "chase", patrol_left: 560.0, patrol_right: 800.0),
        (definition: "chase", patrol_left: 300.0, patrol_right: 400.0, group: Some(2)),
        (definition: "chase", patrol_left: 100.0, patrol_right: 200.0, group: Some(2)),
        (definition: "chase", patrol_left: 420.0, patrol_right: 500.0, chance: 40),
        (definition: "chase", patrol_left: 600.0, patrol_right: 700.0, chance: 0, min_tier: 3),
    ],
    boss: Some((definition: "Ash Warden", position: (560.0, 436.0), min_tier: 5)),
)"#;

#[test]
fn group_and_chance_are_written_and_read_back() {
    let draft = draft(WITH_VARIETY);
    let groups: Vec<_> = draft.enemies.iter().map(|e| e.group).collect();
    let chances: Vec<_> = draft.enemies.iter().map(|e| e.chance).collect();
    assert_eq!(groups, [None, Some(2), Some(2), None, None]);
    assert_eq!(chances, [100, 100, 100, 40, 0]);

    let source = serialized(&draft).unwrap();
    assert_eq!(source.matches("group").count(), 2, "{source}");
    assert!(source.contains("group: Some(2)"), "{source}");
    assert!(
        source.contains("chance: 40") && source.contains("chance: 0"),
        "{source}"
    );
    assert_eq!(source.matches("chance").count(), 2, "{source}");

    let document: RoomEnemies = ron::from_str(&source).unwrap();
    assert_eq!(document.enemies, draft.enemies);
    assert_eq!(document.boss, draft.boss);
}

#[test]
fn a_placement_with_the_defaults_writes_neither_field() {
    let mut draft = draft(WITH_VARIETY);
    for placement in &mut draft.enemies {
        placement.group = None;
        placement.chance = 100;
    }
    let source = serialized(&draft).unwrap();
    assert!(
        !source.contains("group") && !source.contains("chance"),
        "{source}"
    );
    let document: RoomEnemies = ron::from_str(&source).unwrap();
    assert_eq!(document.enemies, draft.enemies);
}

#[test]
fn a_room_file_from_before_group_and_chance_loads_as_always_there() {
    let old = r#"(
        enemies: [(definition: "chase", patrol_left: 560.0, patrol_right: 800.0, min_tier: 2)],
        boss: Some((definition: "Ash Warden", position: (560.0, 436.0))),
    )"#;
    let document: RoomEnemies = ron::from_str(old).unwrap();
    assert_eq!(document.enemies[0].group, None);
    assert_eq!(document.enemies[0].chance, 100);
    assert_eq!(document.enemies[0].min_tier, 2);
    let placement = EnemyPlacement::new("chase", 560.0, 800.0);
    assert_eq!((placement.group, placement.chance), (None, 100));
}

#[test]
fn a_group_outside_1_to_9_is_an_error_naming_it() {
    for group in 1..=9 {
        assert!(ron::from_str::<RoomEnemies>(&enemy(&format!(", group: Some({group})"))).is_ok());
    }
    for group in [0, 10, 255] {
        let error = ron::from_str::<RoomEnemies>(&enemy(&format!(", group: Some({group})")))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains(&format!("group {group}")) && error.contains("1 to 9"),
            "{error}"
        );
    }
    assert!(ron::from_str::<RoomEnemies>(&enemy(", group: Some(-1)")).is_err());
    assert!(ron::from_str::<RoomEnemies>(&enemy(", group: Some(300)")).is_err());
    assert!(ron::from_str::<RoomEnemies>(&enemy(", group: None")).is_ok());
}

#[test]
fn a_chance_outside_0_to_100_is_an_error_naming_it() {
    for chance in [0, 1, 50, 99, 100] {
        assert!(ron::from_str::<RoomEnemies>(&enemy(&format!(", chance: {chance}"))).is_ok());
    }
    let error = ron::from_str::<RoomEnemies>(&enemy(", chance: 101"))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("chance 101") && error.contains("100"),
        "{error}"
    );
    assert!(ron::from_str::<RoomEnemies>(&enemy(", chance: 255")).is_err());
    assert!(ron::from_str::<RoomEnemies>(&enemy(", chance: -1")).is_err());
    assert!(ron::from_str::<RoomEnemies>(&enemy(", chance: 12.5")).is_err());
}

#[test]
fn misspelt_fields_and_a_boss_with_either_are_still_errors() {
    assert!(ron::from_str::<RoomEnemies>(&enemy(", groups: Some(1)")).is_err());
    assert!(ron::from_str::<RoomEnemies>(&enemy(", chanse: 50")).is_err());
    for extra in [", group: Some(1)", ", chance: 50"] {
        let boss = format!(
            r#"(enemies: [], boss: Some((definition: "Ash Warden", position: (560.0, 436.0){extra})))"#
        );
        assert!(
            ron::from_str::<RoomEnemies>(&boss).is_err(),
            "a boss is never optional"
        );
    }
}

#[test]
fn a_grouped_placement_with_a_chance_under_100_is_refused_naming_the_placement() {
    let source = r#"(enemies: [
        (definition: "chase", patrol_left: 560.0, patrol_right: 800.0),
        (definition: "chase", patrol_left: 300.0, patrol_right: 400.0, group: Some(3), chance: 60),
    ])"#;
    let draft = draft(source);
    let error = draft.validate_local(&definitions()).unwrap_err();
    assert!(
        error.contains("enemy 1") && error.contains("chase"),
        "{error}"
    );
    assert!(
        error.contains("group 3") && error.contains("chance 60"),
        "{error}"
    );
    assert!(error.contains("chance 100"), "{error}");
    assert_eq!(
        serialized(&draft).unwrap_err(),
        error,
        "Save refuses it too"
    );
    assert!(draft.build(RoomId::First, &definitions()).is_err());

    // Chance 100 with a group, and any chance without one, are fine.
    let fine = source.replace("group: Some(3), chance: 60", "group: Some(3)");
    assert!(self::draft(&fine).validate_local(&definitions()).is_ok());
    let fine = source.replace("group: Some(3), chance: 60", "chance: 60");
    assert!(self::draft(&fine).validate_local(&definitions()).is_ok());
}

#[test]
fn building_a_draft_keeps_every_placement_with_its_group_and_chance() {
    let draft = draft(WITH_VARIETY);
    let room = draft.build(RoomId::First, &definitions()).unwrap();
    let seen: Vec<_> = room
        .enemies
        .iter()
        .map(|enemy| (enemy.group(), enemy.chance(), enemy.min_tier()))
        .collect();
    assert_eq!(
        seen,
        [
            (None, 100, 0),
            (Some(2), 100, 0),
            (Some(2), 100, 0),
            (None, 40, 0),
            (None, 0, 3)
        ],
        "the draft is built whole: the editor and the Test room see every placement"
    );
}

#[test]
fn the_loaders_below_the_game_leave_every_placement_in_the_room() {
    let dir = std::env::temp_dir().join(format!("hollow-steps-variety-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&support::fixture_assets(), &dir);
    let first = WITH_VARIETY.replace("chance: 0, min_tier: 3", "chance: 0");
    std::fs::write(dir.join("rooms/first.ron"), first).unwrap();

    let [first, _, _] = load_rooms(&dir).unwrap();
    assert_eq!(
        first.enemies.len(),
        5,
        "load_rooms does not know about seeds"
    );
    assert_eq!(first.enemies[1].group(), Some(2));
    assert_eq!(first.enemies[3].chance(), 40);
    assert_eq!(
        first.enemies[4].chance(),
        0,
        "chance 0 is still in the room outside a run"
    );
    assert!(first.boss_special.is_some());

    let extra = hollow_steps::editor::load_extra_rooms(&dir, &definitions()).unwrap();
    assert!(extra.iter().all(|room| room
        .enemies
        .iter()
        .all(|e| e.group().is_none() && e.chance() == 100)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_breaks_the_group_rule_names_itself_when_the_rooms_load() {
    let dir = std::env::temp_dir().join(format!("hollow-steps-variety-bad-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    copy(&support::fixture_assets(), &dir);
    let broken = r#"(enemies: [
        (definition: "chase", patrol_left: 560.0, patrol_right: 800.0, group: Some(1), chance: 10),
    ])"#;
    std::fs::write(dir.join("rooms/first.ron"), broken).unwrap();
    let error = load_rooms(&dir).map(|_| ()).unwrap_err();
    assert!(error.contains("first.ron"), "{error}");
    assert!(
        error.contains("enemy 0") && error.contains("group 1"),
        "{error}"
    );
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
