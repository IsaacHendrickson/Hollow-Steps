//! A save of `vitals.ron` is refused when its max mana is below the cost of a charm in the charm
//! list that will be saved with it, so the next start never fails with "mana_cost must be between
//! 0 and the base max mana". The fixture charms cost 15 (heart), 10 (swift), 20 (sharp) and
//! 5 (ward) against a max mana of 100.
mod support;

use hollow_steps::charm_editor::CharmDraft;
use hollow_steps::charms::CharmCatalog;
use hollow_steps::editor_guard::save_all;
use hollow_steps::game::GameState;
use hollow_steps::player_editor::{CharmCost, PlayerDraft};
use hollow_steps::tuning::Field;
use hollow_steps::vitals::{VitalsConfig, VitalsField};
use support::player_assets::Assets;

/// `player.ron`, `vitals.ron` and `charms.ron` as they are on disk (`None` when not there).
fn files(assets: &Assets) -> [Option<String>; 3] {
    ["player.ron", "vitals.ron", "charms.ron"]
        .map(|name| std::fs::read_to_string(assets.file(name)).ok())
}

fn set_max_mana(game: &mut GameState, value: f32) {
    game.editor_mut()
        .unwrap()
        .player
        .set_vitals_field(VitalsField::MaxMana, value)
        .unwrap();
}

fn saved_max_mana(assets: &Assets) -> f32 {
    VitalsConfig::load(&assets.0).unwrap().max_mana
}

fn loads(assets: &Assets) {
    if let Err(error) = GameState::try_new(&assets.0) {
        panic!("the game no longer loads: {error}");
    }
}

fn charm_index(game: &GameState, id: &str) -> usize {
    let edited = game.editor().unwrap().charms.edited();
    edited.iter().position(|charm| charm.id == id).unwrap()
}

#[test]
fn lowering_max_mana_below_a_saved_charm_cost_is_refused_and_nothing_is_written() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    set_max_mana(&mut game, 15.0);
    // A stats edit travels with it: the whole Player panel Save is refused, not just the vitals.
    game.editor_mut()
        .unwrap()
        .player
        .set_field(Field::MoveSpeed, 250.0)
        .unwrap();
    let before = files(&assets);

    let error = game.save_player().unwrap_err();

    assert!(error.contains("Sharp Charm"), "names the charm: {error}");
    assert!(error.contains("20"), "gives the lowest allowed: {error}");
    assert!(error.contains("15"), "says what it compared: {error}");
    assert_eq!(files(&assets), before, "nothing was written");
    assert!(
        game.editor().unwrap().player.is_vitals_dirty(),
        "the edit is still there to be fixed"
    );
    loads(&assets);
}

#[test]
fn a_max_mana_equal_to_the_highest_cost_is_allowed_and_one_below_it_is_not() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    set_max_mana(&mut game, 19.5);
    assert!(game.save_player().is_err());
    assert_eq!(saved_max_mana(&assets), 100.0);

    set_max_mana(&mut game, 20.0);
    game.save_player().unwrap();
    assert_eq!(saved_max_mana(&assets), 20.0);
    loads(&assets);
}

#[test]
fn the_f4_save_refuses_it_too() {
    let assets = Assets::new(false);
    let mut game = GameState::try_new(&assets.0).unwrap();
    let lowered = game
        .vitals_config()
        .with_field(VitalsField::MaxMana, 15.0)
        .unwrap();
    game.set_vitals_config(lowered).unwrap();
    let before = files(&assets);

    let error = game.save_vitals().unwrap_err();

    assert!(
        error.contains("Sharp Charm") && error.contains("20"),
        "{error}"
    );
    assert_eq!(files(&assets), before);
    assert!(game.vitals_unsaved(), "still unsaved, so it can be raised");
    loads(&assets);

    let enough = game
        .vitals_config()
        .with_field(VitalsField::MaxMana, 20.0)
        .unwrap();
    game.set_vitals_config(enough).unwrap();
    game.save_vitals().unwrap();
    assert_eq!(saved_max_mana(&assets), 20.0);
    loads(&assets);
}

#[test]
fn save_all_refuses_before_it_writes_anything() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    // Three things are unsaved: a charm (written first by Save all), the stats and the vitals.
    let session = game.editor_mut().unwrap();
    session.charms.set_mana_cost(0, 12.0).unwrap();
    session.player.set_field(Field::MoveSpeed, 250.0).unwrap();
    set_max_mana(&mut game, 15.0);
    let before = files(&assets);

    let error = save_all(&mut game).unwrap_err();

    assert!(
        error.contains("Sharp Charm") && error.contains("20"),
        "{error}"
    );
    assert_eq!(files(&assets), before, "not even the charms were written");
    assert!(game.editor().unwrap().charms.is_dirty());
    loads(&assets);
}

#[test]
fn save_all_uses_the_charm_list_it_is_about_to_write() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    // The Sharp Charm is deleted in the Charms panel and not saved yet; Save all saves that
    // list first, so max mana 15 (the Heart Charm's cost) fits it.
    let sharp = charm_index(&game, "sharp");
    game.editor_mut().unwrap().charms.delete(sharp).unwrap();
    set_max_mana(&mut game, 15.0);

    save_all(&mut game).unwrap();

    assert_eq!(saved_max_mana(&assets), 15.0);
    let on_disk = CharmCatalog::load(&assets.0, 15.0).unwrap();
    assert!(on_disk.get("sharp").is_none());
    loads(&assets);
}

#[test]
fn a_plain_player_save_is_held_to_the_charms_on_disk_not_the_unsaved_list() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    // Deleted in the panel, still in charms.ron: saving 15 now and then giving up the deletion
    // would leave a game that cannot start.
    let sharp = charm_index(&game, "sharp");
    game.editor_mut().unwrap().charms.delete(sharp).unwrap();
    set_max_mana(&mut game, 15.0);
    let before = files(&assets);

    let error = game.save_player().unwrap_err();

    assert!(error.contains("Sharp Charm"), "{error}");
    assert_eq!(files(&assets), before);
}

#[test]
fn raising_max_mana_is_always_allowed() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    set_max_mana(&mut game, 250.0);
    game.save_player().unwrap();
    assert_eq!(saved_max_mana(&assets), 250.0);
    loads(&assets);

    // And a draft whose saved max mana is somehow below a cost may still be raised past it.
    let costs = [CharmCost {
        name: "Heavy Charm".into(),
        cost: 30.0,
    }];
    let mut draft = PlayerDraft::default();
    draft.set_vitals_field(VitalsField::MaxMana, 30.0).unwrap();
    draft.validate_vitals(&costs).unwrap();
}

#[test]
fn the_rule_names_the_dearest_charm_and_ignores_an_empty_list() {
    let costs = [
        CharmCost {
            name: "Heart Charm".into(),
            cost: 15.0,
        },
        CharmCost {
            name: "Sharp Charm".into(),
            cost: 20.0,
        },
        CharmCost {
            name: "Swift Charm".into(),
            cost: 10.0,
        },
    ];
    let mut draft = PlayerDraft::default();
    draft.set_vitals_field(VitalsField::MaxMana, 12.0).unwrap();

    let message = draft.vitals_problem(&costs).unwrap();
    assert!(message.contains("Sharp Charm"), "{message}");
    assert!(
        message.contains("lowest allowed max mana is 20"),
        "{message}"
    );
    assert!(message.contains("max mana 12"), "{message}");
    assert_eq!(draft.vitals_problem(&[]), None, "no charm, no rule");

    draft.set_vitals_field(VitalsField::MaxMana, 20.0).unwrap();
    assert_eq!(draft.vitals_problem(&costs), None);
}

#[test]
fn the_charms_panel_says_its_number_comes_from_the_saved_vitals() {
    let assets = Assets::new(false);
    let game = GameState::try_new(&assets.0).unwrap();
    let mut draft = CharmDraft::from_catalog(game.charm_catalog());
    // The cost is above 100, the max mana saved in vitals.ron, whatever F4 says now.
    draft.set_mana_cost(0, 150.0).unwrap();

    let message = draft.problem_for(0, "mana_cost", 100.0).unwrap();

    assert!(message.contains("base max mana (100)"), "{message}");
    assert!(message.contains("saved in vitals.ron"), "{message}");
}

#[test]
fn a_hand_edit_that_still_breaks_loading_ends_with_one_hint_line() {
    let assets = Assets::new(false);
    let text = std::fs::read_to_string(assets.file("vitals.ron")).unwrap();
    std::fs::write(
        assets.file("vitals.ron"),
        text.replace("max_mana: 100.0", "max_mana: 15.0"),
    )
    .unwrap();

    let error = GameState::try_new(&assets.0).unwrap_err();

    assert!(
        error.contains("mana_cost must be between 0 and the base max mana (15)"),
        "the old text stays: {error}"
    );
    assert!(error.contains("charm \"sharp\""), "{error}");
    let last = error.lines().last().unwrap();
    assert!(
        last.contains(
            "lower the charm's mana_cost in assets/charms.ron, or raise max_mana in \
             assets/vitals.ron (or delete that file)"
        ),
        "the last line is the hint: {error}"
    );
    assert_eq!(error.matches("assets/vitals.ron").count(), 1, "{error}");
}

#[test]
fn other_charm_file_errors_get_no_hint() {
    let assets = Assets::new(false);
    std::fs::write(assets.file("charms.ron"), "(charms: [], surprise: 1)").unwrap();

    let error = GameState::try_new(&assets.0).unwrap_err();

    assert!(error.contains("charms"), "{error}");
    assert!(!error.contains("assets/vitals.ron"), "{error}");
}
