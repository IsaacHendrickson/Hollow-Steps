//! Test room (F6) with the Charms panel's unsaved list: it plays the edited charms, writes
//! nothing, and leaves the run's own charms as they were.
mod support;

use hollow_steps::charm_editor::EffectKind;
use hollow_steps::charms::Toggle;
use support::player_assets::Assets;

fn charms_text(assets: &Assets) -> String {
    std::fs::read_to_string(assets.file("charms.ron")).unwrap()
}

#[test]
fn the_test_room_plays_the_unsaved_charms_and_writes_nothing() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let written = charms_text(&assets);
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);
    let base = game.player().config().max_health;
    // Edit the equipped heart charm and make a charm that only the editor knows.
    let draft = &mut game.editor_mut().unwrap().charms;
    draft.set_effect_amount(0, 40.0).unwrap();
    let new = draft.new_charm();
    draft.set_effect_kind(new, EffectKind::MaxHealth).unwrap();
    draft.set_effect_amount(new, 7.0).unwrap();
    draft.set_owned_at_start(new, true).unwrap();

    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert_eq!(
        game.player().config().max_health,
        base - 10 + 40,
        "the edited heart charm counts"
    );
    assert!(
        game.charm_catalog().get("charm_1").is_some(),
        "the new charm exists in the run"
    );
    assert_eq!(
        game.toggle_charm("charm_1"),
        Toggle::Equipped,
        "and is owned, to try it"
    );
    assert_eq!(game.player().config().max_health, base - 10 + 40 + 7);
    assert_eq!(charms_text(&assets), written, "a test room writes nothing");

    game.end_test_room().unwrap();
    let session = game.editor().expect("back in the same session");
    assert!(session.charms.is_dirty(), "still unsaved after the test");
    assert_eq!(
        session.charms.edited()[0].effect,
        hollow_steps::charms::Effect::MaxHealth(40)
    );
    assert_eq!(charms_text(&assets), written);
}

#[test]
fn the_runs_own_charms_are_put_back_when_the_test_ends() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    // Nothing is equipped or owned beyond the start before the test.
    let draft = &mut game.editor_mut().unwrap().charms;
    let new = draft.new_charm();
    draft.set_owned_at_start(new, true).unwrap();
    game.sync_charms_from_editor();
    assert!(
        !game.charm_state().is_owned("charm_1"),
        "owned_at_start only matters for a new run"
    );

    game.start_test_room().unwrap();
    assert!(game.charm_state().is_owned("charm_1"));
    game.toggle_charm("swift");
    game.toggle_charm("charm_1");
    assert!(game.charm_state().is_equipped("swift"));
    game.end_test_room().unwrap();

    assert!(
        !game.charm_state().is_owned("charm_1"),
        "the grant belonged to the test"
    );
    assert!(
        !game.charm_state().is_equipped("swift"),
        "so did what was worn in it"
    );
    assert!(game.charm_state().equipped().is_empty());
}

#[test]
fn a_charm_list_with_a_problem_cannot_be_tested() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    game.editor_mut()
        .unwrap()
        .charms
        .set_name(1, "Heart Charm")
        .unwrap();
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("the charms cannot be used"), "{error}");
    assert!(error.contains("Heart Charm"), "{error}");
    assert!(!game.testing_room());
    assert!(game.editor().is_some(), "the editor stays as it was");
}

#[test]
fn a_charm_cost_above_the_saved_base_mana_cannot_be_tested() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    // The setter takes any number in the field's range; the base-mana rule is a problem that
    // blocks Save and the test, and the panel shows it.
    game.editor_mut()
        .unwrap()
        .charms
        .set_mana_cost(3, 101.0)
        .unwrap();
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("the charms cannot be used"), "{error}");
    assert!(error.contains("base max mana (100)"), "{error}");
    assert!(!game.testing_room());
    // At the base itself it plays.
    game.editor_mut()
        .unwrap()
        .charms
        .set_mana_cost(3, 100.0)
        .unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
}
