//! The Upgrades panel's draft in a game: edits are in effect at once, Save writes upgrades.ron and
//! keeps the player where it is, and the Save / Discard / Cancel dialog lists and handles the
//! upgrade list. Each test uses a private hub world with a known list (`support::upgrade_world`).
mod support;

use hollow_steps::charms::Effect;
use hollow_steps::editor_guard::{
    perform, save_all, unsaved_besides_rooms, GuardChoice, GuardStep, GuardedAction, UnsavedGuard,
};
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomId;
use hollow_steps::snapshot_history::SnapshotHistory;
use hollow_steps::tuning::Field;
use hollow_steps::upgrades::UpgradeCatalog;
use support::upgrade_world::{editing_hub, hub_assets, upgrades_text, SAMPLE};

fn max_health(game: &GameState) -> u32 {
    u32::from(game.player().config().max_health)
}

/// Makes Vitality give 25 health a level instead of 10.
fn edit_vitality(game: &mut GameState) {
    let draft = &mut game.editor_mut().unwrap().upgrades;
    draft.set_effect_amount(0, 25.0).unwrap();
}

#[test]
fn an_edit_is_in_effect_after_the_next_sync_and_nothing_is_written() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 2)]);
    let with_saved = max_health(&game);
    edit_vitality(&mut game);
    assert_eq!(
        max_health(&game),
        with_saved,
        "not until the editor's frame syncs"
    );
    game.sync_upgrades_from_editor();
    assert_eq!(
        max_health(&game),
        with_saved - 20 + 50,
        "two levels of 25 instead of 10"
    );
    assert_eq!(
        upgrades_text(&assets),
        SAMPLE,
        "nothing is written until Save"
    );
    assert!(game.upgrades_unsaved());
    // Putting the number back puts the stats back.
    game.editor_mut().unwrap().upgrades.reset_all();
    game.sync_upgrades_from_editor();
    assert_eq!(max_health(&game), with_saved);
}

#[test]
fn an_edit_with_a_problem_is_not_played_and_cannot_be_saved() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 2)]);
    let with_saved = max_health(&game);
    let draft = &mut game.editor_mut().unwrap().upgrades;
    draft.set_effect_amount(0, 25.0).unwrap();
    draft.set_name(1, "Vitality").unwrap();
    game.sync_upgrades_from_editor();
    assert_eq!(
        max_health(&game),
        with_saved,
        "an invalid list is not applied"
    );
    let error = game.upgrade_problem().unwrap_err();
    assert!(error.contains("Vitality"), "{error}");
    assert!(game.save_upgrades().is_err());
    assert_eq!(
        upgrades_text(&assets),
        SAMPLE,
        "an invalid list writes nothing"
    );
    assert!(!assets.file("upgrades.ron.tmp").exists());
    assert!(game.upgrades_unsaved(), "the edits are kept");
}

#[test]
fn save_writes_the_list_and_applies_it_without_resetting_the_player() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 3)]);
    game.toggle_editor().unwrap();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        0.1,
    );
    let place = game.player().position();
    let spawn = game.room().spawn;
    assert_ne!(place, spawn, "the player has walked");
    game.toggle_editor().unwrap();
    edit_vitality(&mut game);
    let with_saved = max_health(&game);

    let path = game.save_upgrades().unwrap();
    assert_eq!(path, assets.file("upgrades.ron"));
    let on_disk = UpgradeCatalog::load(&assets.0).unwrap();
    assert_eq!(on_disk.upgrades()[0].effect, Effect::MaxHealth(25));
    assert_eq!(on_disk.upgrades().len(), 3);
    assert_eq!(
        max_health(&game),
        with_saved - 30 + 75,
        "the running game uses the saved list at once"
    );
    assert_eq!(game.player().position(), place, "the player was not reset");
    assert!(!game.upgrades_unsaved());
    assert!(!game.editor().unwrap().upgrades.is_dirty());
}

#[test]
fn deleting_an_upgrade_keeps_the_levels_but_gives_no_effect() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 2), ("keen_edge", 1)]);
    let with_both = max_health(&game);
    let profile_before = std::fs::read_to_string(assets.file("profile.ron")).unwrap();
    game.editor_mut().unwrap().upgrades.delete(0).unwrap();
    game.sync_upgrades_from_editor();
    assert_eq!(
        max_health(&game),
        with_both - 20,
        "no effect from a deleted upgrade"
    );
    assert_eq!(
        game.profile().upgrades["vitality"],
        2,
        "the levels are kept"
    );
    game.save_upgrades().unwrap();
    assert_eq!(game.profile().upgrades["vitality"], 2);
    assert_eq!(
        std::fs::read_to_string(assets.file("profile.ron")).unwrap(),
        profile_before,
        "the profile file is not touched"
    );
    // Listing an upgrade with that id again gives the levels back.
    let draft = &mut game.editor_mut().unwrap().upgrades;
    let new = draft.new_upgrade();
    assert_eq!(draft.edited()[new].id, "upgrade_1");
    let mut list = draft.edited().to_vec();
    list[new].id = "vitality".into();
    list[new].effect = Effect::MaxHealth(10);
    list[new].costs = vec![30, 60, 100];
    draft.set_edited(list);
    game.sync_upgrades_from_editor();
    assert_eq!(max_health(&game), with_both);
}

#[test]
fn the_panels_undo_and_redo_put_back_what_is_played() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 1)]);
    let with_saved = max_health(&game);
    let mut history = SnapshotHistory::default();
    let draft = &mut game.editor_mut().unwrap().upgrades;
    let before = draft.edited().to_vec();
    draft.set_effect_amount(0, 30.0).unwrap();
    history.record(before, draft.edited());
    game.sync_upgrades_from_editor();
    assert_eq!(max_health(&game), with_saved - 10 + 30);

    assert!(history.undo(&mut game.editor_mut().unwrap().upgrades));
    game.sync_upgrades_from_editor();
    assert_eq!(max_health(&game), with_saved);
    assert!(!game.upgrades_unsaved());
    assert!(history.redo(&mut game.editor_mut().unwrap().upgrades));
    game.sync_upgrades_from_editor();
    assert_eq!(max_health(&game), with_saved - 10 + 30);
    assert!(game.upgrades_unsaved());
}

#[test]
fn a_dirty_upgrade_list_makes_a_clean_room_ask_and_the_dialog_lists_upgrades() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[]);
    let mut guard = UnsavedGuard::default();
    assert!(unsaved_besides_rooms(game.editor().unwrap()).is_empty());
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Perform(GuardedAction::Close)
    );

    edit_vitality(&mut game);
    assert!(!game.editor().unwrap().dirty, "the room itself is clean");
    assert_eq!(unsaved_besides_rooms(game.editor().unwrap()), ["Upgrades"]);
    for action in [
        GuardedAction::Close,
        GuardedAction::OpenRoom(RoomId::Second),
        GuardedAction::NewRoom,
    ] {
        assert_eq!(guard.request_for(action, game.editor()), GuardStep::Wait);
        assert_eq!(guard.unsaved(), ["Upgrades"]);
        guard.choose(GuardChoice::Cancel);
    }

    // Listed after the player and the charms.
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    let cost = session.charms.edited()[0].mana_cost;
    session.charms.set_mana_cost(0, cost + 5.0).unwrap();
    assert_eq!(
        unsaved_besides_rooms(game.editor().unwrap()),
        ["Player", "Charms", "Upgrades"]
    );
    guard.request_for(GuardedAction::Close, game.editor());
    assert_eq!(guard.unsaved(), ["Player", "Charms", "Upgrades"]);
    guard.choose(GuardChoice::Cancel);

    // Putting the list back makes it clean again.
    let session = game.editor_mut().unwrap();
    session.player.reset_all();
    session.charms.reset_all();
    session.upgrades.reset_all();
    assert!(unsaved_besides_rooms(game.editor().unwrap()).is_empty());
}

#[test]
fn save_writes_the_upgrades_only_when_they_are_dirty() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[]);
    // A clean list writes nothing, even when something else is saved.
    game.editor_mut()
        .unwrap()
        .player
        .set_field(Field::JumpSpeed, 650.0)
        .unwrap();
    save_all(&mut game).unwrap();
    assert_eq!(upgrades_text(&assets), SAMPLE);

    edit_vitality(&mut game);
    save_all(&mut game).unwrap();
    assert_ne!(upgrades_text(&assets), SAMPLE);
    let on_disk = UpgradeCatalog::load(&assets.0).unwrap();
    assert_eq!(on_disk.upgrades()[0].effect, Effect::MaxHealth(25));
    assert!(!game.editor().unwrap().upgrades.is_dirty());
    assert!(
        !game.editor().unwrap().message.starts_with("Saved"),
        "the clean room was not saved"
    );

    // A dirty room is saved with it.
    let session = game.editor_mut().unwrap();
    session.dirty = true;
    session.upgrades.set_cost(0, 0, 31.0).unwrap();
    save_all(&mut game).unwrap();
    let session = game.editor().unwrap();
    assert!(session.message.starts_with("Saved"), "{}", session.message);
    assert!(!session.dirty && !session.upgrades.is_dirty());
}

#[test]
fn an_upgrade_list_that_cannot_be_saved_fails_the_save_before_anything_is_written() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[]);
    let session = game.editor_mut().unwrap();
    session.dirty = true;
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    session.upgrades.set_name(0, "").unwrap();
    let error = save_all(&mut game).unwrap_err();
    assert!(error.contains("name"), "{error}");
    assert_eq!(upgrades_text(&assets), SAMPLE);
    assert!(
        !assets.file("player.ron").exists(),
        "the check comes before the first write"
    );
    let session = game.editor().unwrap();
    assert!(session.dirty && session.player.is_dirty() && session.upgrades.is_dirty());
}

#[test]
fn the_dialog_flow_saves_the_upgrades_before_it_closes_the_editor() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 1)]);
    let mut guard = UnsavedGuard::default();
    edit_vitality(&mut game);
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
        game.upgrade_catalog().upgrades()[0].effect,
        Effect::MaxHealth(25),
        "the saved list runs"
    );
    assert_eq!(
        UpgradeCatalog::load(&assets.0).unwrap().upgrades()[0].effect,
        Effect::MaxHealth(25)
    );
}

#[test]
fn a_failed_upgrade_save_keeps_the_dialog_open_with_the_reason() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[]);
    let mut guard = UnsavedGuard::default();
    game.editor_mut().unwrap().upgrades.set_name(0, "").unwrap();
    guard.request_for(GuardedAction::Close, game.editor());
    guard.choose(GuardChoice::Save);
    assert_eq!(guard.save_finished(save_all(&mut game)), GuardStep::Wait);
    let prompt = guard.prompt().expect("still open");
    assert!(
        prompt.error.as_deref().unwrap_or_default().contains("name"),
        "{prompt:?}"
    );
    assert!(game.editor().is_some() && game.upgrades_unsaved());
}

#[test]
fn discard_drops_the_upgrade_edits_from_the_session_and_from_play() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[("vitality", 2)]);
    let with_saved = max_health(&game);
    edit_vitality(&mut game);
    game.editor_mut().unwrap().upgrades.delete(1).unwrap();
    game.sync_upgrades_from_editor();
    assert_eq!(
        max_health(&game),
        with_saved - 20 + 50,
        "in effect before the choice"
    );
    assert_eq!(game.upgrade_catalog().upgrades().len(), 2);

    perform(&mut game, &GuardedAction::Close).unwrap();
    assert!(game.editor().is_none());
    assert_eq!(max_health(&game), with_saved);
    assert_eq!(
        game.upgrade_catalog().upgrades().len(),
        3,
        "the deleted upgrade is back"
    );
    assert_eq!(upgrades_text(&assets), SAMPLE);

    // The next session starts from the saved list, not the discarded edits.
    game.toggle_editor().unwrap();
    let draft = &game.editor().unwrap().upgrades;
    assert!(!draft.is_dirty());
    assert_eq!(draft.edited()[0].effect, Effect::MaxHealth(10));
}

#[test]
fn discard_also_applies_when_the_action_opens_another_room() {
    let assets = hub_assets();
    let mut game = editing_hub(&assets, &[]);
    edit_vitality(&mut game);
    game.sync_upgrades_from_editor();
    assert_eq!(
        game.upgrade_catalog().upgrades()[0].effect,
        Effect::MaxHealth(25)
    );
    perform(&mut game, &GuardedAction::OpenRoom(RoomId::Second)).unwrap();
    assert_eq!(
        game.upgrade_catalog().upgrades()[0].effect,
        Effect::MaxHealth(10)
    );
    assert!(!game.editor().unwrap().upgrades.is_dirty());
    assert!(!game.upgrades_unsaved());
}
