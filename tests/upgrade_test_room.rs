//! Test room (F6) with the Upgrades panel's unsaved list: it plays the edited upgrades, writes
//! nothing, and the session is still dirty afterwards.
mod support;

use hollow_steps::charms::Effect;
use hollow_steps::game::GameState;
use support::upgrade_world::{editing_hub, hub_assets, upgrades_text, SAMPLE};

fn max_health(game: &GameState) -> u32 {
    u32::from(game.player().config().max_health)
}

#[test]
fn the_test_room_plays_the_unsaved_upgrades_and_writes_nothing() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 2)]);
    let with_saved = max_health(&game);
    let profile_before = std::fs::read_to_string(assets.file("profile.ron")).unwrap();
    let draft = &mut game.editor_mut().unwrap().upgrades;
    draft.set_effect_amount(0, 25.0).unwrap();
    // The editor's frame has not synced yet: the test room takes the draft as it is.
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert_eq!(
        max_health(&game),
        with_saved - 20 + 50,
        "the edited Vitality counts in the test room"
    );
    assert_eq!(upgrades_text(&assets), SAMPLE, "a test room writes nothing");
    assert_eq!(
        std::fs::read_to_string(assets.file("profile.ron")).unwrap(),
        profile_before,
        "and spends no coins"
    );

    game.end_test_room().unwrap();
    let session = game.editor().expect("back in the same session");
    assert!(session.upgrades.is_dirty(), "still unsaved after the test");
    assert_eq!(session.upgrades.edited()[0].effect, Effect::MaxHealth(25));
    assert_eq!(upgrades_text(&assets), SAMPLE);
    assert_eq!(game.profile().upgrades["vitality"], 2);
}

#[test]
fn a_new_and_a_deleted_upgrade_are_played_as_edited() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("keen_edge", 1)]);
    let with_saved = max_health(&game);
    let draft = &mut game.editor_mut().unwrap().upgrades;
    draft.new_upgrade();
    draft.delete(2).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(
        max_health(&game),
        with_saved,
        "health is not what these change"
    );
    assert_eq!(
        game.upgrade_catalog().upgrades().len(),
        3,
        "one made, one deleted"
    );
    assert!(game.upgrade_catalog().get("keen_edge").is_none());
    game.end_test_room().unwrap();
    assert!(game.editor().unwrap().upgrades.is_dirty());
}

#[test]
fn an_upgrade_list_with_a_problem_refuses_the_test_and_changes_nothing() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 2)]);
    let with_saved = max_health(&game);
    let draft = &mut game.editor_mut().unwrap().upgrades;
    draft.set_effect_amount(0, 25.0).unwrap();
    draft.set_name(1, "").unwrap();
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("the upgrades cannot be used"), "{error}");
    assert!(!game.testing_room());
    let session = game.editor().expect("the editor stays open");
    assert!(
        session.message.starts_with("Cannot test the room"),
        "{}",
        session.message
    );
    assert!(session.upgrades.is_dirty(), "the edits are kept");
    assert_eq!(
        max_health(&game),
        with_saved,
        "an invalid list is not played"
    );
    assert_eq!(upgrades_text(&assets), SAMPLE);
}

#[test]
fn the_unsaved_upgrades_are_played_again_after_a_second_test() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 1)]);
    let with_saved = max_health(&game);
    game.editor_mut()
        .unwrap()
        .upgrades
        .set_effect_amount(0, 40.0)
        .unwrap();
    for _ in 0..2 {
        game.start_test_room().unwrap();
        assert_eq!(max_health(&game), with_saved - 10 + 40);
        game.end_test_room().unwrap();
    }
    // Saving, then testing, still plays the saved list.
    game.save_upgrades().unwrap();
    game.start_test_room().unwrap();
    assert_eq!(max_health(&game), with_saved - 10 + 40);
    game.end_test_room().unwrap();
}
