//! Check dungeon with dungeon pieces, through the editor session: the marks of the unsaved draft
//! are read, a pool built up room by room (each Saved) is checked as one, and unmarking a room
//! ends its part in the pool while its slots stay. Rooms come from fixtures, never the shipped
//! assets; the tests that Save work on a copy.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::dungeon_check::Problem;
use hollow_steps::editor::ItemPlacement;
use hollow_steps::game::GameState;
use hollow_steps::item::Item;
use hollow_steps::piece::{PieceInfo, PieceRole, SlotKind};
use hollow_steps::room::RoomId;

fn game() -> GameState {
    GameState::try_new(&support::fixture_assets()).unwrap()
}

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-pieces-check-{}-{}",
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
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// Opens `id` in the editor and marks it as a piece of `role`, leaving the draft unsaved.
fn mark(game: &mut GameState, id: RoomId, role: PieceRole) {
    game.edit_room(id).unwrap();
    let session = game.editor_mut().unwrap();
    session.draft.set_piece(Some(PieceInfo::new(role))).unwrap();
    session.dirty = true;
}

/// Marks `id` as a piece of `role` and Saves it, so the next room opens on top of it.
fn mark_and_save(game: &mut GameState, id: RoomId, role: PieceRole) {
    mark(game, id, role);
    game.save_editor().unwrap();
}

fn problems(game: &GameState) -> Vec<Problem> {
    game.editor().unwrap().check_dungeon()
}

fn pool_problems(game: &GameState) -> Vec<Problem> {
    problems(game)
        .into_iter()
        .filter(|problem| {
            matches!(
                problem,
                Problem::NoPieceWithRole { .. }
                    | Problem::PieceWithoutExit { .. }
                    | Problem::PieceLockedExit { .. }
                    | Problem::PieceHoldsKey { .. }
                    | Problem::PieceRoleMismatch { .. }
                    | Problem::PieceGateCannotTravel { .. }
                    | Problem::PieceAbilityWithoutSlot { .. }
            )
        })
        .collect()
}

#[test]
fn fixture_rooms_with_no_piece_have_no_pool_problem() {
    let mut game = game();
    game.edit_room(RoomId::First).unwrap();
    assert_eq!(problems(&game), []);
}

#[test]
fn marking_one_unsaved_room_asks_for_a_start_and_a_boss_piece_and_unmarking_ends_it() {
    let mut game = game();
    mark(&mut game, RoomId::Second, PieceRole::Middle);
    assert_eq!(
        pool_problems(&game),
        [
            Problem::NoPieceWithRole {
                role: PieceRole::Start
            },
            Problem::NoPieceWithRole {
                role: PieceRole::Boss
            },
        ]
    );
    game.editor_mut().unwrap().draft.set_piece(None).unwrap();
    assert_eq!(problems(&game), []);
}

#[test]
fn pieces_in_rooms_edited_one_after_another_all_count() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    mark_and_save(&mut game, RoomId::First, PieceRole::Start);
    mark_and_save(&mut game, RoomId::Second, PieceRole::Middle);
    mark(&mut game, RoomId::Boss, PieceRole::Boss);
    // With a Start, a Middle and a Boss piece the role rules are met, and the rooms the run
    // rules name have the roles they need. Each of them is a reachable room that is walked.
    let found = pool_problems(&game);
    assert!(
        found.iter().all(|problem| !matches!(
            problem,
            Problem::NoPieceWithRole { .. } | Problem::PieceRoleMismatch { .. }
        )),
        "{found:?}"
    );
}

#[test]
fn a_middle_piece_with_a_key_item_is_reported_and_the_room_is_named() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    mark_and_save(&mut game, RoomId::First, PieceRole::Start);
    mark_and_save(&mut game, RoomId::Boss, PieceRole::Boss);
    mark(&mut game, RoomId::Second, PieceRole::Middle);
    game.editor_mut()
        .unwrap()
        .draft
        .layout
        .items
        .push(ItemPlacement {
            item: Item::Key(4),
            position: (100.0, 440.0),
        });
    let found = pool_problems(&game);
    let held = found
        .iter()
        .find(|problem| matches!(problem, Problem::PieceHoldsKey { key: 4, .. }))
        .unwrap_or_else(|| panic!("no key problem in {found:?}"));
    assert!(held.text().contains("Second"), "{}", held.text());
}

#[test]
fn the_run_rules_start_room_must_be_a_start_piece_once_it_is_one() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    mark_and_save(&mut game, RoomId::Boss, PieceRole::Boss);
    mark_and_save(&mut game, RoomId::Second, PieceRole::Start);
    mark(&mut game, RoomId::First, PieceRole::Middle);
    let found = pool_problems(&game);
    assert!(
        found.iter().any(|problem| matches!(
            problem,
            Problem::PieceRoleMismatch {
                field: "start_room",
                expected: PieceRole::Start,
                ..
            }
        )),
        "{found:?}"
    );
}

#[test]
fn unmarking_a_room_keeps_its_slots_and_they_stop_counting() {
    let mut game = game();
    mark(&mut game, RoomId::First, PieceRole::Start);
    let session = game.editor_mut().unwrap();
    session.draft.add_slot().unwrap();
    session.draft.set_slot_kind(0, SlotKind::Ability).unwrap();
    assert_eq!(session.draft.layout.slots.len(), 1);
    session.draft.set_piece(None).unwrap();
    assert_eq!(session.draft.layout.slots.len(), 1);
    assert_eq!(problems(&game), []);
}
