//! The Charms panel and the Save / Discard / Cancel dialog: what makes the editor ask, what the
//! dialog lists, and what Save and Discard do with the charm list.
mod support;

use hollow_steps::charms::{CharmCatalog, Effect};
use hollow_steps::editor_guard::{
    perform, save_all, unsaved_besides_rooms, GuardChoice, GuardStep, GuardedAction, UnsavedGuard,
};
use hollow_steps::game::GameState;
use hollow_steps::room::RoomId;
use hollow_steps::tuning::Field;
use support::player_assets::Assets;

fn charms_text(assets: &Assets) -> String {
    std::fs::read_to_string(assets.file("charms.ron")).unwrap()
}

/// Makes the first fixture charm cost 5 more mana than it did.
fn edit_a_cost(game: &mut GameState) {
    let draft = &mut game.editor_mut().unwrap().charms;
    let cost = draft.edited()[0].mana_cost;
    draft.set_mana_cost(0, cost + 5.0).unwrap();
}

#[test]
fn a_dirty_charm_list_makes_a_clean_room_ask_and_the_dialog_lists_charms() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let mut guard = UnsavedGuard::default();

    assert!(unsaved_besides_rooms(game.editor().unwrap()).is_empty());
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Perform(GuardedAction::Close)
    );

    edit_a_cost(&mut game);
    assert!(!game.editor().unwrap().dirty, "the room itself is clean");
    assert_eq!(unsaved_besides_rooms(game.editor().unwrap()), ["Charms"]);
    for action in [
        GuardedAction::Close,
        GuardedAction::OpenRoom(RoomId::Second),
        GuardedAction::NewRoom,
    ] {
        assert_eq!(
            guard.request_for(action, game.editor()),
            GuardStep::Wait,
            "a dirty list asks first"
        );
        assert_eq!(guard.unsaved(), ["Charms"]);
        guard.choose(GuardChoice::Cancel);
    }

    // With the player dirty as well, both are named.
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    assert_eq!(
        unsaved_besides_rooms(game.editor().unwrap()),
        ["Player", "Charms"]
    );
    guard.request_for(GuardedAction::Close, game.editor());
    assert_eq!(guard.unsaved(), ["Player", "Charms"]);
    guard.choose(GuardChoice::Cancel);

    // Putting the cost back makes the list clean again, and nothing asks.
    let session = game.editor_mut().unwrap();
    session.player.reset_all();
    session.charms.reset_all();
    assert!(unsaved_besides_rooms(game.editor().unwrap()).is_empty());
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Perform(GuardedAction::Close)
    );
}

#[test]
fn save_writes_the_charms_and_only_what_is_dirty() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let before = charms_text(&assets);
    // A clean charm list writes nothing, even when something else is saved.
    game.editor_mut()
        .unwrap()
        .player
        .set_field(Field::JumpSpeed, 650.0)
        .unwrap();
    save_all(&mut game).unwrap();
    assert_eq!(charms_text(&assets), before);

    edit_a_cost(&mut game);
    save_all(&mut game).unwrap();
    assert_ne!(charms_text(&assets), before);
    let on_disk = CharmCatalog::load(&assets.0, 100.0).unwrap();
    assert_eq!(on_disk.charms()[0].mana_cost, 20.0);
    assert!(!game.editor().unwrap().charms.is_dirty());
    assert!(
        !game.editor().unwrap().message.starts_with("Saved"),
        "the clean room was not saved"
    );

    // A dirty room is saved with it.
    let session = game.editor_mut().unwrap();
    session.dirty = true;
    session.charms.set_mana_cost(0, 25.0).unwrap();
    save_all(&mut game).unwrap();
    let session = game.editor().unwrap();
    assert!(session.message.starts_with("Saved"), "{}", session.message);
    assert!(!session.dirty && !session.charms.is_dirty());
}

#[test]
fn a_charm_list_that_cannot_be_saved_fails_the_save_and_writes_nothing() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let before = charms_text(&assets);
    let session = game.editor_mut().unwrap();
    session.dirty = true;
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    session.charms.set_name(0, "Sharp Charm").unwrap();
    let error = save_all(&mut game).unwrap_err();
    assert!(error.contains("Sharp Charm"), "{error}");
    assert_eq!(charms_text(&assets), before);
    assert!(
        !assets.file("player.ron").exists(),
        "nothing after the failure is written"
    );
    let session = game.editor().unwrap();
    assert!(session.dirty && session.player.is_dirty() && session.charms.is_dirty());
}

#[test]
fn the_dialog_flow_saves_the_charms_before_it_closes_the_editor() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let mut guard = UnsavedGuard::default();
    edit_a_cost(&mut game);
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Wait
    );
    assert_eq!(guard.choose(GuardChoice::Save), GuardStep::Save);
    let step = guard.save_finished(save_all(&mut game));
    let GuardStep::Perform(action) = step else {
        panic!("the save worked, so the action goes ahead: {step:?}");
    };
    perform(&mut game, &action).unwrap();
    assert!(game.editor().is_none());
    assert_eq!(
        game.charm_catalog().charms()[0].mana_cost,
        20.0,
        "the saved list runs"
    );
    assert_eq!(
        CharmCatalog::load(&assets.0, 100.0).unwrap().charms()[0].mana_cost,
        20.0
    );
}

#[test]
fn a_failed_charm_save_keeps_the_dialog_open_with_the_reason() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let mut guard = UnsavedGuard::default();
    game.editor_mut().unwrap().charms.set_name(0, "").unwrap();
    guard.request_for(GuardedAction::Close, game.editor());
    guard.choose(GuardChoice::Save);
    assert_eq!(guard.save_finished(save_all(&mut game)), GuardStep::Wait);
    let prompt = guard.prompt().expect("still open");
    assert!(
        prompt.error.as_deref().unwrap_or_default().contains("name"),
        "{prompt:?}"
    );
    assert!(game.editor().is_some() && game.charms_unsaved());
}

#[test]
fn discard_drops_the_charm_edits_from_the_session_and_from_play() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let before = charms_text(&assets);
    game.toggle_charm("heart");
    let base = game.player().config().max_health;
    let draft = &mut game.editor_mut().unwrap().charms;
    draft.set_effect_amount(0, 80.0).unwrap();
    draft.delete(1).unwrap();
    game.sync_charms_from_editor();
    assert!(
        game.player().config().max_health > base,
        "in effect before the choice"
    );
    assert_eq!(game.charm_catalog().charms().len(), 3);

    perform(&mut game, &GuardedAction::Close).unwrap();
    assert!(game.editor().is_none());
    assert_eq!(game.player().config().max_health, base);
    assert_eq!(
        game.charm_catalog().charms().len(),
        4,
        "the deleted charm is back"
    );
    assert!(game.charm_state().is_equipped("heart"));
    assert_eq!(charms_text(&assets), before);

    // The next session starts from the saved list, not the discarded edits.
    game.toggle_editor().unwrap();
    let draft = &game.editor().unwrap().charms;
    assert!(!draft.is_dirty());
    assert_eq!(
        draft.edited()[0].effect,
        game.charm_catalog().charms()[0].effect
    );
    assert!(matches!(draft.edited()[0].effect, Effect::MaxHealth(bonus) if bonus != 80));
}

#[test]
fn discard_also_applies_when_the_action_opens_another_room() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    edit_a_cost(&mut game);
    game.sync_charms_from_editor();
    assert_eq!(game.charm_catalog().charms()[0].mana_cost, 20.0);
    perform(&mut game, &GuardedAction::OpenRoom(RoomId::Second)).unwrap();
    assert_eq!(game.charm_catalog().charms()[0].mana_cost, 15.0);
    assert!(!game.editor().unwrap().charms.is_dirty());
    assert!(!game.charms_unsaved());
}
