//! Charms lying in rooms: placing them in the editor, saving, loading, finding them in play and
//! the rules that keep the files consistent. Works on private copies of the fixtures.
mod support;

use hollow_steps::charms::Toggle;
use hollow_steps::editor::{Placement, Selection};
use hollow_steps::editor_guard::save_all;
use hollow_steps::editor_history::{redo_session, tracked, undo_session, History};
use hollow_steps::events::GameEvent;
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::{RoomId, FLOOR_Y};
use hollow_steps::room_charms::CHARM_SIZE;
use macroquad::math::Vec2;
use support::player_assets::Assets;

/// Where a charm lies on the first room's floor, `x` from the left.
fn on_floor(x: f32) -> Vec2 {
    Vec2::new(x, FLOOR_Y - CHARM_SIZE.y / 2.0)
}

/// The player starts at x = 180 (16 px either side): a charm at 215 is within reach, one at 700
/// is not.
const NEAR: f32 = 215.0;
const FAR: f32 = 700.0;

/// A copy of the fixtures whose first room holds a ward charm at `x`, written by the editor's
/// own Save. The fixture ward is not owned at the start.
fn with_ward(x: f32) -> Assets {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session
        .place(Placement::Charm("ward".into()), on_floor(x))
        .unwrap();
    game.save_editor().unwrap();
    assets
}

/// `with_ward` and the same charm lying in the second room too.
fn with_ward_in_two_rooms() -> Assets {
    let assets = with_ward(NEAR);
    let mut game = assets.editing();
    game.edit_room(RoomId::Second).unwrap();
    let floor = game.editor().unwrap().draft.layout.floor_y;
    let at = Vec2::new(400.0, floor - CHARM_SIZE.y / 2.0);
    let session = game.editor_mut().unwrap();
    session.place(Placement::Charm("ward".into()), at).unwrap();
    game.save_editor().unwrap();
    assets
}

fn playing(assets: &Assets) -> GameState {
    GameState::try_new(&assets.0).unwrap()
}

fn press(game: &mut GameState, input: PlayerInput) {
    game.update(input, 0.0);
}

fn pickup(game: &mut GameState) {
    press(
        game,
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
    );
}

fn walk_to_exit(game: &mut GameState, move_axis: f32) {
    let starting_room = game.room().id;
    for _ in 0..600 {
        game.update(
            PlayerInput {
                move_axis,
                ..PlayerInput::default()
            },
            1.0 / 60.0,
        );
        if game.room().id != starting_room {
            return;
        }
    }
    panic!("walking did not reach an exit");
}

fn first_room_text(assets: &Assets) -> String {
    std::fs::read_to_string(assets.file("rooms/first.ron")).unwrap()
}

// ---- loading --------------------------------------------------------------------------

#[test]
fn a_room_file_without_charms_loads_and_lies_none() {
    let assets = Assets::new(false);
    let game = playing(&assets);
    assert!(game.room_charms().is_empty());
    // A saved room that has none writes no `charms` field either.
    let mut editing = assets.editing();
    editing.editor_mut().unwrap().dirty = true;
    editing.save_editor().unwrap();
    assert!(!first_room_text(&assets).contains("charms"));
    assert!(playing(&assets).room_charms().is_empty());
}

#[test]
fn a_charm_placed_in_the_editor_is_saved_and_lies_in_the_room() {
    let assets = with_ward(NEAR);
    let text = first_room_text(&assets);
    assert!(text.contains("charm: \"ward\""), "{text}");
    let game = playing(&assets);
    let lying = game.room_charms();
    assert_eq!(lying.len(), 1);
    assert_eq!(lying[0].name, "Ward Charm");
    assert_eq!(lying[0].color, [120, 120, 230, 255]);
    assert_eq!(lying[0].position, on_floor(NEAR));
    assert!(lying[0].in_reach);
    // Far away it lies there but is out of reach.
    let far = playing(&with_ward(FAR));
    assert!(!far.room_charms()[0].in_reach);
}

#[test]
fn an_unknown_charm_id_stops_the_game_loading_and_a_reload_keeps_the_old_rooms() {
    let assets = with_ward(NEAR);
    let mut game = playing(&assets);
    let text = first_room_text(&assets).replace("\"ward\"", "\"nope\"");
    std::fs::write(assets.file("rooms/first.ron"), text).unwrap();
    let error = GameState::try_new(&assets.0).unwrap_err();
    assert!(error.contains("charm \"nope\" does not exist"), "{error}");
    assert!(error.contains("First"), "{error}");
    assert!(game.reload_rooms().is_err());
    assert_eq!(
        game.room_charms().len(),
        1,
        "a failed reload changes nothing"
    );
}

#[test]
fn a_charm_inside_a_solid_in_a_room_file_stops_the_game_loading() {
    let assets = with_ward(NEAR);
    let text = first_room_text(&assets).replace(
        "charms: [",
        "terrain: [(190.0, 440.0, 60.0, 20.0)], charms: [",
    );
    std::fs::write(assets.file("rooms/first.ron"), text).unwrap();
    let error = GameState::try_new(&assets.0).unwrap_err();
    assert!(error.contains("must not be inside terrain"), "{error}");
}

// ---- play -----------------------------------------------------------------------------

#[test]
fn picking_up_a_charm_adds_it_to_the_owned_charms_once_and_takes_it_from_the_room() {
    let assets = with_ward(NEAR);
    let mut game = playing(&assets);
    assert!(!game.charm_state().is_owned("ward"));
    assert!(game.owned_charms().iter().all(|charm| charm.id != "ward"));
    let max_health = game.player().config().max_health;

    pickup(&mut game);
    assert!(game.charm_state().is_owned("ward"));
    assert!(game.room_charms().is_empty());
    assert_eq!(game.take_events(), [GameEvent::ItemPickedUp]);
    assert!(game.owned_charms().iter().any(|charm| charm.id == "ward"));
    assert_eq!(
        game.player().config().max_health,
        max_health,
        "owned is not worn"
    );

    // A second press finds nothing, and ownership is not doubled.
    pickup(&mut game);
    assert!(game.take_events().is_empty());
    let owned = game.charm_state().owned();
    assert_eq!(owned.iter().filter(|id| *id == "ward").count(), 1);
    // It can be worn now.
    assert_eq!(game.toggle_charm("ward"), Toggle::Equipped);
    assert!(game.player().config().max_health > max_health);
}

#[test]
fn a_charm_out_of_reach_or_a_player_in_the_air_picks_nothing_up() {
    let mut far = playing(&with_ward(FAR));
    pickup(&mut far);
    assert!(!far.charm_state().is_owned("ward"));
    assert_eq!(far.room_charms().len(), 1);

    let mut near = playing(&with_ward(NEAR));
    press(
        &mut near,
        PlayerInput {
            jump_pressed: true,
            pickup_pressed: true,
            ..PlayerInput::default()
        },
    );
    assert!(!near.charm_state().is_owned("ward"), "same rule as an item");
    assert_eq!(near.room_charms().len(), 1);
}

#[test]
fn an_upward_strike_is_not_a_pickup() {
    let mut game = playing(&with_ward(NEAR));
    press(
        &mut game,
        PlayerInput {
            pickup_pressed: true,
            attack_pressed: true,
            up_held: true,
            ..PlayerInput::default()
        },
    );
    assert!(!game.charm_state().is_owned("ward"));
}

#[test]
fn a_picked_up_charm_stays_gone_after_room_travel_and_an_unfound_one_stays_put() {
    let assets = with_ward(NEAR);
    let mut found = playing(&assets);
    pickup(&mut found);
    walk_to_exit(&mut found, 1.0);
    assert_eq!(found.room().id, RoomId::Second);
    walk_to_exit(&mut found, -1.0);
    assert_eq!(found.room().id, RoomId::First);
    assert!(found.room_charms().is_empty(), "gone for the run");
    assert!(found.charm_state().is_owned("ward"));

    let mut unfound = playing(&assets);
    walk_to_exit(&mut unfound, 1.0);
    walk_to_exit(&mut unfound, -1.0);
    assert_eq!(unfound.room_charms().len(), 1, "still waiting where it was");
    assert!(!unfound.charm_state().is_owned("ward"));
}

#[test]
fn a_charm_already_owned_is_not_placed_again() {
    // The fixture heart charm is owned from the start.
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session
        .place(Placement::Charm("heart".into()), on_floor(NEAR))
        .unwrap();
    session
        .place(Placement::Charm("ward".into()), on_floor(FAR))
        .unwrap();
    game.save_editor().unwrap();
    assert!(
        first_room_text(&assets).contains("\"heart\""),
        "the file keeps it"
    );
    let playing = playing(&assets);
    let names: Vec<_> = playing.room_charms().into_iter().map(|c| c.name).collect();
    assert_eq!(
        names,
        ["Ward Charm"],
        "only the one not owned yet lies there"
    );

    // The same after the editor's own Save rebuilds the rooms of a running game.
    let mut running = playing_with_editor(&assets);
    running.editor_mut().unwrap().dirty = true;
    running.save_editor().unwrap();
    running.toggle_editor().unwrap();
    let names: Vec<_> = running.room_charms().into_iter().map(|c| c.name).collect();
    assert_eq!(names, ["Ward Charm"]);
}

fn playing_with_editor(assets: &Assets) -> GameState {
    assets.editing()
}

#[test]
fn finding_a_charm_takes_every_copy_out_of_every_room() {
    // The same charm placed in the first and the second room can be found only once.
    let assets = with_ward_in_two_rooms();
    let mut game = playing(&assets);
    pickup(&mut game);
    walk_to_exit(&mut game, 1.0);
    assert!(
        game.room_charms().is_empty(),
        "the second room's copy went too"
    );
}

// ---- the editor -----------------------------------------------------------------------

#[test]
fn the_placement_list_offers_the_charms_of_the_working_list_by_name() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let label = |session: &hollow_steps::editor::EditorSession, name: &str| {
        session
            .placement_catalog()
            .into_iter()
            .find(|(label, _)| label == name)
    };
    let (_, placement) = label(session, "Charm: Ward Charm").expect("listed");
    assert!(matches!(placement, Placement::Charm(id) if id == "ward"));
    assert!(
        label(session, "Item: Dash").is_some(),
        "the rest of the list is still there"
    );
    // A charm made in the Charms panel can be placed before it is saved, under its name.
    let new = session.charms.new_charm();
    session.charms.set_name(new, "Moth Charm").unwrap();
    assert!(label(session, "Charm: Moth Charm").is_some());
    // A rename shows at once; a deleted charm leaves the list.
    session.charms.set_name(0, "Red Charm").unwrap();
    assert!(
        label(session, "Charm: Red Charm").is_some()
            && label(session, "Charm: Heart Charm").is_none()
    );
    session.charms.delete(0).unwrap();
    assert!(label(session, "Charm: Red Charm").is_none());
}

#[test]
fn place_move_and_delete_a_charm_with_undo_and_redo() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let original = session.draft.clone();

    let selection = tracked(&mut history, session, |s| {
        s.place(Placement::Charm("ward".into()), on_floor(300.0))
    })
    .unwrap();
    assert_eq!(selection, Selection::Charm(0));
    let placed = session.draft.clone();
    assert_eq!(placed.layout.charms.len(), 1);

    tracked(&mut history, session, |s| {
        s.move_selection(Selection::Charm(0), on_floor(340.0))
    })
    .unwrap();
    assert_eq!(
        session.draft.layout.charms[0].position,
        (340.0, FLOOR_Y - 8.0)
    );
    let moved = session.draft.clone();

    tracked(&mut history, session, |s| {
        s.draft.delete(Selection::Charm(0))
    })
    .unwrap();
    assert!(session.draft.layout.charms.is_empty());

    assert!(undo_session(&mut history, session).unwrap());
    assert_eq!(
        session.draft, moved,
        "undo brings the charm back where it was"
    );
    assert!(undo_session(&mut history, session).unwrap());
    assert_eq!(session.draft, placed);
    assert!(undo_session(&mut history, session).unwrap());
    assert_eq!(session.draft, original);
    assert!(redo_session(&mut history, session).unwrap());
    assert_eq!(session.draft, placed);
    assert!(redo_session(&mut history, session).unwrap());
    assert_eq!(session.draft, moved);
}

#[test]
fn a_charm_cannot_be_placed_in_a_solid_or_outside_the_room() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.draft.layout.terrain = vec![(600.0, FLOOR_Y - 40.0, 100.0, 40.0)];
    let before = session.draft.clone();
    let inside = session
        .place(Placement::Charm("ward".into()), on_floor(650.0))
        .unwrap_err();
    assert!(
        inside.contains("charm \"ward\" must not be inside terrain"),
        "{inside}"
    );
    let outside = session.place(Placement::Charm("ward".into()), Vec2::new(-50.0, 100.0));
    let outside = outside.unwrap_err();
    assert!(
        outside.contains("charm \"ward\" must be inside the room"),
        "{outside}"
    );
    assert_eq!(session.draft, before, "a refused placement changes nothing");
}

#[test]
fn a_room_cannot_be_saved_naming_a_charm_that_is_not_saved_yet() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let before = first_room_text_or_empty(&assets);
    let session = game.editor_mut().unwrap();
    let new = session.charms.new_charm();
    let id = session.charms.edited()[new].id.clone();
    session
        .place(Placement::Charm(id.clone()), on_floor(NEAR))
        .unwrap();
    let error = game.save_editor().unwrap_err();
    assert!(
        error.contains(&format!("charm \"{id}\" is not saved yet")),
        "{error}"
    );
    assert!(error.contains("save the charms first"), "{error}");
    assert_eq!(
        first_room_text_or_empty(&assets),
        before,
        "nothing was written"
    );

    game.save_charms().unwrap();
    game.save_editor().unwrap();
    assert!(first_room_text(&assets).contains(&format!("charm: \"{id}\"")));
    assert_eq!(playing(&assets).room_charms().len(), 1);
}

fn first_room_text_or_empty(assets: &Assets) -> String {
    std::fs::read_to_string(assets.file("rooms/first.ron")).unwrap_or_default()
}

#[test]
fn a_room_cannot_be_saved_naming_a_charm_that_does_not_exist() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.draft.layout.charms = vec![hollow_steps::room_charms::CharmPlacement {
        charm: "nope".into(),
        position: on_floor(NEAR).into(),
    }];
    let error = game.save_editor().unwrap_err();
    assert!(error.contains("charm \"nope\" does not exist"), "{error}");
}

#[test]
fn a_charm_a_saved_room_places_cannot_be_deleted_until_the_room_lets_it_go() {
    let assets = with_ward(NEAR);
    let charms_before = std::fs::read_to_string(assets.file("charms.ron")).unwrap();
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let ward = session
        .charms
        .edited()
        .iter()
        .position(|charm| charm.id == "ward")
        .unwrap();
    session.charms.delete(ward).unwrap();
    let error = game.save_charms().unwrap_err();
    assert!(
        error.contains("charm \"ward\" is placed in first.ron"),
        "{error}"
    );
    assert!(error.contains("save the room before deleting"), "{error}");
    assert_eq!(
        std::fs::read_to_string(assets.file("charms.ron")).unwrap(),
        charms_before
    );
    assert!(
        game.charms_unsaved(),
        "the edit stays for the user to resolve"
    );

    // Take it out of the room and save the room; the charm can go.
    game.editor_mut()
        .unwrap()
        .draft
        .delete(Selection::Charm(0))
        .unwrap();
    game.save_editor().unwrap();
    game.save_charms().unwrap();
    let after = playing(&assets);
    assert!(after.charm_catalog().get("ward").is_none());
    assert!(after.room_charms().is_empty());
}

#[test]
fn save_all_saves_the_room_first_when_the_list_deletes_a_placed_charm() {
    let assets = with_ward(NEAR);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let ward = session
        .charms
        .edited()
        .iter()
        .position(|charm| charm.id == "ward")
        .unwrap();
    session.charms.delete(ward).unwrap();
    session.draft.delete(Selection::Charm(0)).unwrap();
    session.dirty = true;
    assert!(game.charm_save_needs_rooms_first());
    save_all(&mut game).unwrap();
    let after = playing(&assets);
    assert!(after.charm_catalog().get("ward").is_none());
    assert!(after.room_charms().is_empty());
    assert!(!first_room_text(&assets).contains("ward"));
}

#[test]
fn save_all_saves_the_charms_first_for_a_new_charm_placed_in_the_room() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let new = session.charms.new_charm();
    let id = session.charms.edited()[new].id.clone();
    session
        .place(Placement::Charm(id.clone()), on_floor(NEAR))
        .unwrap();
    session.dirty = true;
    assert!(!game.charm_save_needs_rooms_first());
    save_all(&mut game).unwrap();
    let after = playing(&assets);
    assert!(after.charm_catalog().get(&id).is_some());
    assert_eq!(after.room_charms().len(), 1);
}

#[test]
fn a_deletion_and_a_new_placement_together_are_refused_with_nothing_written() {
    let assets = with_ward(NEAR);
    let (charms, room) = (
        std::fs::read_to_string(assets.file("charms.ron")).unwrap(),
        first_room_text(&assets),
    );
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let ward = session
        .charms
        .edited()
        .iter()
        .position(|charm| charm.id == "ward")
        .unwrap();
    session.charms.delete(ward).unwrap();
    session.draft.delete(Selection::Charm(0)).unwrap();
    let new = session.charms.new_charm();
    let id = session.charms.edited()[new].id.clone();
    session.place(Placement::Charm(id), on_floor(FAR)).unwrap();
    session.dirty = true;
    let error = save_all(&mut game).unwrap_err();
    assert!(error.contains("save the charms first"), "{error}");
    assert_eq!(
        std::fs::read_to_string(assets.file("charms.ron")).unwrap(),
        charms
    );
    assert_eq!(first_room_text(&assets), room);
    assert!(game.charms_unsaved() && game.editor().unwrap().dirty);
}

#[test]
fn the_test_room_lies_the_drafts_charms_and_a_find_in_it_is_undone_when_it_ends() {
    let assets = with_ward_in_two_rooms();
    let mut game = assets.editing();
    // Move it in the draft, unsaved: the test plays what is on the canvas.
    game.editor_mut()
        .unwrap()
        .move_selection(Selection::Charm(0), on_floor(NEAR + 5.0))
        .unwrap();
    game.start_test_room().unwrap();
    let lying = game.room_charms();
    assert_eq!(lying.len(), 1);
    assert_eq!(lying[0].position, on_floor(NEAR + 5.0));
    // The player spawns at the layout's spawn, within reach of it: take it.
    assert!(lying[0].in_reach);
    pickup(&mut game);
    assert!(
        game.charm_state().is_owned("ward"),
        "a test can pick a charm up"
    );
    assert!(game.room_charms().is_empty());
    game.end_test_room().unwrap();
    assert!(
        !game.charm_state().is_owned("ward"),
        "the find belonged to the test"
    );
    // The saved first room still has its own charm for the real run.
    game.toggle_editor().unwrap();
    assert_eq!(game.room_charms().len(), 1);
    walk_to_exit(&mut game, 1.0);
    assert_eq!(
        game.room_charms().len(),
        1,
        "the second room kept its copy too"
    );
}
