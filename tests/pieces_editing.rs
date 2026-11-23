//! Dungeon pieces and pickup slots end to end in the editor: mark a room and add slots, Save,
//! restart from the files; the Test room does not show a slot as a pickup; the unsaved-changes
//! guard. Rooms come from fixtures, never the shipped assets.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{RoomLayout, Selection};
use hollow_steps::editor_guard::{GuardStep, GuardedAction, UnsavedGuard};
use hollow_steps::game::GameState;
use hollow_steps::piece::{PickupSlot, PieceInfo, PieceRole, SlotKind};
use hollow_steps::room::{RoomEnemies, RoomId};
use macroquad::math::Vec2;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-pieces-{}-{}",
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

/// The first room marked as a Boss piece of weight 4 with two slots, a Key and an Ability one.
fn mark_and_slot(game: &mut GameState) -> (PieceInfo, Vec<PickupSlot>) {
    game.edit_room(RoomId::First).unwrap();
    let session = game.editor_mut().unwrap();
    let piece = PieceInfo {
        role: PieceRole::Boss,
        weight: 4,
    };
    session.draft.set_piece(Some(piece)).unwrap();
    session.draft.add_slot().unwrap();
    session.draft.add_slot().unwrap();
    session.draft.set_slot_kind(0, SlotKind::Key).unwrap();
    session.draft.set_slot_kind(1, SlotKind::Ability).unwrap();
    session.dirty = true;
    (piece, session.draft.layout.slots.clone())
}

#[test]
fn a_room_in_a_fixture_is_no_piece_and_has_no_slots() {
    let game = GameState::try_new(&support::fixture_assets()).unwrap();
    let layout = RoomLayout::from_room(game.room());
    assert_eq!(layout.piece, None);
    assert!(layout.slots.is_empty());
}

#[test]
fn a_mark_and_slots_are_saved_with_the_room_and_load_again() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    let (piece, slots) = mark_and_slot(&mut game);
    game.save_editor().unwrap();

    let text = temp.text("first.ron");
    assert!(text.contains("piece") && text.contains("slots"), "{text}");
    let document: RoomEnemies = ron::from_str(&text).unwrap();
    let saved = document.layout.unwrap();
    assert_eq!(saved.piece, Some(piece));
    assert_eq!(saved.slots, slots);

    // A restart reads it back into the room, and the editor draft shows the same.
    let mut fresh = temp.game();
    let layout = RoomLayout::from_room(fresh.room());
    assert_eq!(layout.piece, Some(piece));
    assert_eq!(layout.slots, slots);
    fresh.edit_room(RoomId::First).unwrap();
    assert_eq!(fresh.editor().unwrap().draft.layout, layout);
    // Other rooms are untouched.
    assert_eq!(
        temp.text("second.ron"),
        std::fs::read_to_string(support::fixture_assets().join("rooms").join("second.ron"))
            .unwrap()
    );
}

#[test]
fn unmarking_a_saved_piece_keeps_its_slots_in_the_file() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    let (_, slots) = mark_and_slot(&mut game);
    game.save_editor().unwrap();
    let session = game.editor_mut().unwrap();
    session.draft.set_piece(None).unwrap();
    session.dirty = true;
    game.save_editor().unwrap();

    let text = temp.text("first.ron");
    assert!(!text.contains("piece"), "{text}");
    let layout = temp.game();
    let layout = RoomLayout::from_room(layout.room());
    assert_eq!(layout.piece, None);
    assert_eq!(layout.slots, slots);
}

#[test]
fn a_slot_that_breaks_a_rule_is_refused_on_save_and_nothing_is_written() {
    let temp = TempAssets::new();
    let before = temp.text("first.ron");
    let mut game = temp.game();
    game.edit_room(RoomId::First).unwrap();
    {
        let session = game.editor_mut().unwrap();
        // Past the editor's own checks: a slot outside the room.
        session
            .draft
            .layout
            .slots
            .push(PickupSlot::new((-500.0, 100.0), SlotKind::Any));
        session.dirty = true;
    }
    let error = game.save_editor().unwrap_err();
    assert!(error.contains("pickup slot 1"), "{error}");
    assert_eq!(temp.text("first.ron"), before);
    assert!(RoomLayout::from_room(temp.game().room()).slots.is_empty());
}

#[test]
fn the_test_room_shows_no_slot_as_a_pickup_and_writes_nothing() {
    let temp = TempAssets::new();
    let before = temp.text("first.ron");
    let mut game = temp.game();
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    let items = game.items().to_vec();
    game.end_test_room().unwrap();

    let (_, slots) = mark_and_slot(&mut game);
    assert_eq!(slots.len(), 2);
    game.start_test_room().unwrap();
    assert_eq!(
        game.items(),
        items,
        "the same pickups with or without slots"
    );
    assert!(game.items().iter().all(|item| slots
        .iter()
        .all(|slot| Vec2::from(slot.position) != item.position)));
    game.end_test_room().unwrap();
    assert_eq!(temp.text("first.ron"), before, "testing writes nothing");
    assert_eq!(game.editor().unwrap().draft.layout.slots, slots);
}

#[test]
fn marking_a_room_or_adding_a_slot_trips_the_unsaved_changes_guard_until_saved() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.edit_room(RoomId::First).unwrap();
    let mut guard = UnsavedGuard::default();
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Perform(GuardedAction::Close),
        "a clean session closes at once"
    );

    for edit in [
        |game: &mut GameState| {
            let draft = &mut game.editor_mut().unwrap().draft;
            draft
                .set_piece(Some(PieceInfo::new(PieceRole::Start)))
                .unwrap();
        },
        |game: &mut GameState| {
            game.editor_mut().unwrap().draft.add_slot().unwrap();
        },
    ] {
        let mut game = temp.game();
        game.edit_room(RoomId::First).unwrap();
        edit(&mut game);
        // The panel marks the room unsaved with every accepted edit.
        game.editor_mut().unwrap().dirty = true;
        let mut guard = UnsavedGuard::default();
        assert_eq!(
            guard.request_for(GuardedAction::Close, game.editor()),
            GuardStep::Wait
        );
        assert!(guard.is_open(), "the dialog asks before closing");
        assert_eq!(game.editor().unwrap().unsaved_rooms().len(), 1);
        game.save_editor().unwrap();
        let mut guard = UnsavedGuard::default();
        assert_eq!(
            guard.request_for(GuardedAction::Close, game.editor()),
            GuardStep::Perform(GuardedAction::Close),
            "saved, so it closes at once"
        );
    }
}

#[test]
fn a_selected_slot_is_part_of_the_room_being_edited() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    mark_and_slot(&mut game);
    let session = game.editor_mut().unwrap();
    session.dirty = false;
    session
        .drag_selection(Selection::Slot(0), Vec2::new(500.0, 300.0))
        .unwrap();
    assert!(session.dirty, "dragging a slot marks the room unsaved");
    assert_eq!(session.draft.slot(0).unwrap().position, (500.0, 300.0));
}
