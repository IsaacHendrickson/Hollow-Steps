//! The editor's charm list and the running game: what an edit changes at once, what Save does,
//! and what a refused or discarded list leaves alone. Works on a private copy of the fixtures.
mod support;

use hollow_steps::charm_editor::EffectKind;
use hollow_steps::charms::{CharmCatalog, Effect, Toggle};
use hollow_steps::game::GameState;
use support::player_assets::Assets;

/// What `charms.ron` says now (the fixtures ship one).
fn charms_text(assets: &Assets) -> String {
    std::fs::read_to_string(assets.file("charms.ron")).unwrap()
}

fn index_of(game: &GameState, id: &str) -> usize {
    game.editor()
        .unwrap()
        .charms
        .edited()
        .iter()
        .position(|charm| charm.id == id)
        .unwrap()
}

fn edit<R>(
    game: &mut GameState,
    change: impl FnOnce(&mut hollow_steps::charm_editor::CharmDraft) -> R,
) -> R {
    change(&mut game.editor_mut().unwrap().charms)
}

#[test]
fn the_session_starts_with_the_games_list_and_is_clean() {
    let assets = Assets::new(false);
    let game = assets.editing();
    let draft = &game.editor().unwrap().charms;
    assert_eq!(draft.edited(), game.charm_catalog().charms());
    assert!(!draft.is_dirty() && !game.charms_unsaved());
}

#[test]
fn editing_an_equipped_charm_changes_the_players_stats_at_once() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);
    let Some(Effect::MaxHealth(bonus)) = game.charm_catalog().get("heart").map(|c| c.effect) else {
        panic!("the fixture's heart charm adds max health");
    };
    let base = game.player().config().max_health;
    let heart = index_of(&game, "heart");
    edit(&mut game, |d| {
        d.set_effect_amount(heart, f32::from(bonus) + 7.0)
    })
    .unwrap();
    assert_eq!(
        game.player().config().max_health,
        base,
        "nothing happens until the frame syncs"
    );
    game.sync_charms_from_editor();
    assert_eq!(game.player().config().max_health, base + 7);
    assert!(game.charms_unsaved());
    // Nothing was written.
    assert_eq!(
        CharmCatalog::load(&assets.0, 100.0)
            .unwrap()
            .get("heart")
            .unwrap()
            .effect,
        Effect::MaxHealth(bonus)
    );
}

#[test]
fn an_unequipped_charm_edit_leaves_the_stats_alone_and_a_cost_edit_moves_max_mana() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let base = game.player().config().max_health;
    let heart = index_of(&game, "heart");
    edit(&mut game, |d| d.set_effect_amount(heart, 50.0)).unwrap();
    game.sync_charms_from_editor();
    assert_eq!(game.player().config().max_health, base);
    game.toggle_charm("swift");
    assert_eq!(game.vitals().max_mana(), 90.0);
    let swift = index_of(&game, "swift");
    edit(&mut game, |d| d.set_mana_cost(swift, 30.0)).unwrap();
    game.sync_charms_from_editor();
    assert_eq!(game.vitals().max_mana(), 70.0);
}

#[test]
fn a_list_with_a_problem_is_not_applied_and_save_writes_nothing() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let written = charms_text(&assets);
    game.toggle_charm("swift");
    let speed = game.player().config().move_speed;
    let swift = index_of(&game, "swift");
    edit(&mut game, |d| {
        d.set_effect_amount(swift, 100.0).unwrap();
        d.set_name(swift, "").unwrap();
    });
    game.sync_charms_from_editor();
    assert_eq!(
        game.player().config().move_speed,
        speed,
        "an invalid list is not played"
    );
    assert!(game.charm_problem().unwrap_err().contains("name"));
    let error = game.save_charms().unwrap_err();
    assert!(error.contains("name"), "{error}");
    assert_eq!(charms_text(&assets), written);
    assert!(game.charms_unsaved());
    // Fixing the name makes it playable and savable.
    edit(&mut game, |d| d.set_name(swift, "Swift Charm").unwrap());
    game.sync_charms_from_editor();
    assert!(game.player().config().move_speed > speed);
    assert!(game.charm_problem().is_ok());
}

#[test]
fn a_cost_above_the_saved_base_max_mana_blocks_save() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let written = charms_text(&assets);
    let ward = index_of(&game, "ward");
    edit(&mut game, |d| d.set_mana_cost(ward, 101.0)).unwrap();
    let error = game.save_charms().unwrap_err();
    assert!(error.contains("base max mana (100)"), "{error}");
    assert_eq!(charms_text(&assets), written);
}

#[test]
fn save_writes_the_file_and_the_list_runs_without_resetting_the_player() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    game.toggle_charm("swift");
    let (position, health) = (game.player().position(), game.player().health().current());
    let swift = index_of(&game, "swift");
    edit(&mut game, |d| {
        d.set_effect_amount(swift, 60.0).unwrap();
        d.set_mana_cost(swift, 25.0).unwrap();
        d.set_name(swift, "Wind Charm").unwrap();
    });
    let saved = game.save_charms().unwrap();
    assert_eq!(saved.path, assets.file("charms.ron"));
    assert!(saved.taken_off.is_empty());
    assert!(!game.charms_unsaved());
    let on_disk = CharmCatalog::load(&assets.0, 100.0).unwrap();
    assert_eq!(on_disk.charms(), game.editor().unwrap().charms.saved());
    assert_eq!(on_disk.get("swift").unwrap().name, "Wind Charm");
    // The running game uses it: still equipped, new cost and speed, player untouched.
    assert_eq!(game.charm_catalog().get("swift").unwrap().mana_cost, 25.0);
    assert!(game.charm_state().is_equipped("swift"));
    assert_eq!(game.vitals().max_mana(), 75.0);
    assert_eq!(
        (game.player().position(), game.player().health().current()),
        (position, health)
    );
    // A new game started from the same assets sees the saved file too.
    assert_eq!(
        GameState::try_new(&assets.0)
            .unwrap()
            .charm_catalog()
            .charms(),
        on_disk.charms()
    );
}

#[test]
fn deleting_an_equipped_charm_unequips_it_and_drops_it_once_saved() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    game.toggle_charm("heart");
    game.toggle_charm("sharp");
    let damage = game.player().config().melee.damage;
    let sharp = index_of(&game, "sharp");
    edit(&mut game, |d| d.delete(sharp)).unwrap();
    game.sync_charms_from_editor();
    assert!(
        game.player().config().melee.damage < damage,
        "the deleted charm no longer counts, even before saving"
    );
    game.save_charms().unwrap();
    assert!(!game.charm_state().is_equipped("sharp") && !game.charm_state().is_owned("sharp"));
    assert!(game.charm_state().is_equipped("heart"));
    assert!(game.charm_catalog().get("sharp").is_none());
    assert!(game.owned_charms().iter().all(|charm| charm.id != "sharp"));
}

#[test]
fn saved_costs_that_no_longer_fit_take_the_last_equipped_charm_off() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    for id in ["heart", "swift", "sharp"] {
        assert_eq!(game.toggle_charm(id), Toggle::Equipped);
    }
    let (heart, sharp) = (index_of(&game, "heart"), index_of(&game, "sharp"));
    edit(&mut game, |d| {
        d.set_mana_cost(heart, 50.0).unwrap();
        d.set_mana_cost(sharp, 50.0).unwrap();
    });
    let saved = game.save_charms().unwrap();
    assert_eq!(saved.taken_off, vec!["Sharp Charm".to_owned()]);
    assert_eq!(
        game.charm_state().equipped(),
        ["heart".to_owned(), "swift".to_owned()]
    );
    assert!(game.charm_state().is_owned("sharp"), "taken off, not lost");
    assert_eq!(game.vitals().max_mana(), 40.0);
}

#[test]
fn new_charms_and_kind_changes_are_saved_and_loaded() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let new = edit(&mut game, |d| d.new_charm());
    edit(&mut game, |d| {
        d.set_effect_kind(new, EffectKind::SwordDamage).unwrap();
        d.set_effect_amount(new, 3.0).unwrap();
        d.set_owned_at_start(new, true).unwrap();
    });
    game.save_charms().unwrap();
    let again = GameState::try_new(&assets.0).unwrap();
    let charm = again.charm_catalog().get("charm_1").unwrap();
    assert_eq!(charm.effect, Effect::SwordDamage(3));
    assert!(charm.owned_at_start);
}

#[test]
fn discard_puts_the_saved_list_back_in_the_session_and_in_play() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let written = charms_text(&assets);
    game.toggle_charm("heart");
    let base = game.player().config().max_health;
    let heart = index_of(&game, "heart");
    edit(&mut game, |d| {
        d.set_effect_amount(heart, 90.0).unwrap();
        d.new_charm();
    });
    game.sync_charms_from_editor();
    assert!(game.player().config().max_health > base);
    game.discard_charm_edits();
    assert_eq!(game.player().config().max_health, base);
    assert!(!game.charms_unsaved());
    assert!(game.charm_state().is_equipped("heart"));
    assert_eq!(charms_text(&assets), written);
}
