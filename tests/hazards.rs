//! Terrain hazards end to end: author one in the editor, play it in a Test room, Save, and
//! restart from the files. Rooms come from fixtures, never the shipped assets.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{RoomLayout, Selection};
use hollow_steps::game::GameState;
use hollow_steps::hazard::HazardPlacement;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::{RoomEnemies, RoomId};
use macroquad::math::Vec2;

const DT: f32 = 1.0 / 120.0;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-hazards-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }

    fn text(&self, file: &str) -> String {
        std::fs::read_to_string(self.0.join("rooms").join(file)).unwrap()
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn idle() -> PlayerInput {
    PlayerInput::default()
}

fn walk_right() -> PlayerInput {
    PlayerInput {
        move_axis: 1.0,
        ..idle()
    }
}

fn run(game: &mut GameState, ticks: u32, input: PlayerInput) {
    for _ in 0..ticks {
        game.update(input, DT);
    }
}

/// The editor on the first room, a hazard added next to the spawn (and the number of the
/// hazard), then the room played as a Test room: the player stands at the spawn on the floor.
fn testing_with_a_hazard(game: &mut GameState) {
    game.toggle_editor().unwrap();
    let draft = &mut game.editor_mut().unwrap().draft;
    // The fixture room's enemy and items are not wanted in the way.
    draft.enemies.clear();
    draft.layout.items.clear();
    assert_eq!(draft.add_hazard().unwrap(), Selection::Hazard(0));
    game.start_test_room().unwrap();
    // Settle on the floor and let the arrival immunity (one second) run out.
    run(game, 130, idle());
    assert!(game.player().is_grounded());
}

#[test]
fn a_room_without_hazards_has_none_and_a_player_beside_none_is_never_hurt() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    assert!(game.room().hazards.is_empty());
    let full = game.player().health().current();
    run(&mut game, 240, idle());
    assert_eq!(game.player().health().current(), full);
}

#[test]
fn a_hazard_the_player_does_not_touch_does_nothing() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    testing_with_a_hazard(&mut game);
    let full = game.player().health().current();
    run(&mut game, 240, idle());
    assert_eq!(game.player().health().current(), full);
    assert_eq!(game.room().hazards.len(), 1);
}

/// Walks right into the hazard until the first tick that hurts; returns the player's x just
/// before that tick.
fn walk_into_the_hazard(game: &mut GameState) -> f32 {
    let full = game.player().health().current();
    for _ in 0..600 {
        let before = game.player().position().x;
        game.update(walk_right(), DT);
        if game.player().health().current() < full {
            return before;
        }
    }
    panic!("the player never walked into the hazard");
}

#[test]
fn walking_into_a_hazard_costs_its_damage_and_throws_the_player_back_and_up() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    testing_with_a_hazard(&mut game);
    let hazard = game.room().hazards[0].clone();
    let full = game.player().health().current();
    walk_into_the_hazard(&mut game);
    assert_eq!(
        game.player().health().current(),
        full - hazard.damage,
        "exactly the hazard's damage"
    );
    assert!(game.player().health().is_invincible());
    assert!(game.player().velocity().y < 0.0, "thrown up");
    assert!(!game.player().is_grounded(), "off the ground");

    // The player entered from the left, so away from the hazard's center is left: with no
    // key held the whole push is spent and the player ends clear of the hazard's left side.
    run(&mut game, 60, idle());
    assert!(game.player().position().x + 16.0 <= hazard.rect.x);
}

#[test]
fn the_push_is_stronger_than_walking_back_into_the_hazard() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    testing_with_a_hazard(&mut game);
    let x_at_hit = walk_into_the_hazard(&mut game);
    // Keep holding right for the length of the push: the player still ends up well behind.
    let ticks = (game.room().hazards[0].knockback_secs / DT).round() as u32;
    run(&mut game, ticks, walk_right());
    assert!(
        game.player().position().x < x_at_hit - 20.0,
        "pushed back from {x_at_hit} to {}",
        game.player().position().x
    );
}

#[test]
fn immunity_stops_a_second_hit_and_a_hit_after_it_ends_lands_again() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    {
        let draft = &mut game.editor_mut().unwrap().draft;
        draft.enemies.clear();
        draft.layout.items.clear();
        draft.add_hazard().unwrap();
        // A long strip with no knockback, so the player keeps walking inside it.
        let (x, y, _, h) = draft.hazard(0).unwrap().rect;
        let mut long = draft.hazard(0).unwrap().clone();
        long.rect = (x, y, 600.0, h);
        long.knockback_speed = 0.0;
        long.lift_speed = 0.0;
        long.knockback_secs = 0.0;
        draft.replace_hazard(0, long).unwrap();
    }
    game.start_test_room().unwrap();
    run(&mut game, 130, idle()); // arrival immunity over
    let damage = game.room().hazards[0].damage;
    let full = game.player().health().current();

    walk_into_the_hazard(&mut game);
    let after_first = game.player().health().current();
    assert_eq!(after_first, full - damage);
    // Still inside it half a second later (immunity is one second): no second hit.
    run(&mut game, 60, walk_right());
    assert_eq!(game.player().health().current(), after_first);
    // Past the immunity and still inside the strip: hit again.
    run(&mut game, 90, walk_right());
    assert_eq!(game.player().health().current(), after_first - damage);
}

#[test]
fn a_hazard_is_saved_with_the_room_and_loads_again() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.edit_room(RoomId::First).unwrap();
    let wanted = {
        let draft = &mut game.editor_mut().unwrap().draft;
        draft.add_hazard().unwrap();
        let mut edited = draft.hazard(0).unwrap().clone();
        edited.damage = 20;
        edited.knockback_speed = 300.0;
        edited.lift_speed = 150.0;
        edited.knockback_secs = 0.25;
        draft.replace_hazard(0, edited.clone()).unwrap();
        edited
    };
    game.editor_mut().unwrap().dirty = true;
    game.save_editor().unwrap();

    let text = temp.text("first.ron");
    assert!(text.contains("hazards"), "{text}");
    let document: RoomEnemies = ron::from_str(&text).unwrap();
    assert_eq!(document.layout.unwrap().hazards, vec![wanted.clone()]);

    // A restart reads it back into the room: the same numbers.
    let fresh = temp.game();
    let room = fresh.room();
    assert_eq!(room.id, RoomId::First);
    assert_eq!(room.hazards.len(), 1);
    assert_eq!(HazardPlacement::from(&room.hazards[0]), wanted);
    assert_eq!(RoomLayout::from_room(room).hazards, vec![wanted]);
    // Other rooms are untouched.
    assert!(
        temp.text("second.ron")
            == std::fs::read_to_string(support::fixture_assets().join("rooms").join("second.ron"))
                .unwrap()
    );
}

#[test]
fn a_hazard_that_breaks_a_rule_is_refused_on_save_and_nothing_is_written() {
    let temp = TempAssets::new();
    let before = temp.text("first.ron");
    let mut game = temp.game();
    game.edit_room(RoomId::First).unwrap();
    {
        let draft = &mut game.editor_mut().unwrap().draft;
        // Past the editor's own checks: no damage at all.
        let mut bad = HazardPlacement::standard(500.0, 444.0);
        bad.damage = 0;
        draft.layout.hazards.push(bad);
    }
    game.editor_mut().unwrap().dirty = true;
    let error = game.save_editor().unwrap_err();
    assert!(
        error.contains("hazard 0") && error.contains("damage"),
        "{error}"
    );
    assert_eq!(temp.text("first.ron"), before);
    assert!(temp.game().room().hazards.is_empty());
}

#[test]
fn testing_a_room_with_a_hazard_writes_nothing_and_leaves_the_saved_room_without_it() {
    let temp = TempAssets::new();
    let before = temp.text("first.ron");
    let mut game = temp.game();
    testing_with_a_hazard(&mut game);
    run(&mut game, 120, walk_right());
    game.end_test_room().unwrap();
    assert_eq!(temp.text("first.ron"), before);
    assert!(game.room().hazards.is_empty(), "the saved room has none");
    assert_eq!(game.editor().unwrap().draft.layout.hazards.len(), 1);
}

#[test]
fn moving_a_hazard_in_the_editor_moves_the_one_that_is_played() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    session.draft.enemies.clear();
    session.draft.layout.items.clear();
    session.draft.add_hazard().unwrap();
    session
        .move_selection(Selection::Hazard(0), Vec2::new(700.0, 440.0))
        .unwrap();
    game.start_test_room().unwrap();
    let rect = game.room().hazards[0].rect;
    assert_eq!(rect.center(), Vec2::new(700.0, 440.0));
}
