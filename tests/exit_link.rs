//! Linking an exit to another room: the chooser, opening B, connecting, cancelling at every
//! step, and saving every room the flow touched.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::editor_session::EditorSession;
use hollow_steps::exit_arrival::arrival_error;
use hollow_steps::exit_link::{LinkCandidate, Step};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEdge, RoomEnemies, RoomId};
use macroquad::math::Vec2;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-link-{}-{}",
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

    /// Every file under `rooms/`, with its bytes, sorted by name.
    fn room_files(&self) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = std::fs::read_dir(self.0.join("rooms"))
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (
                    entry.file_name().into_string().unwrap(),
                    std::fs::read(entry.path()).unwrap(),
                )
            })
            .collect();
        files.sort();
        files
    }

    fn document(&self, file: &str) -> RoomEnemies {
        ron::de::from_bytes(&std::fs::read(self.0.join("rooms").join(file)).unwrap()).unwrap()
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn editing(temp: &TempAssets) -> GameState {
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    game
}

fn editor(game: &mut GameState) -> &mut EditorSession {
    game.editor_mut().unwrap()
}

fn titles(candidates: &[LinkCandidate]) -> Vec<String> {
    candidates.iter().map(|c| c.title.clone()).collect()
}

/// The saved layout of a room, from the session's catalog.
fn layout_of(session: &EditorSession, id: RoomId) -> RoomLayout {
    session
        .catalog()
        .into_iter()
        .find(|(room, _)| *room == id)
        .unwrap()
        .1
}

/// Opens the first room's first exit in the chooser.
fn start(game: &mut GameState) {
    editor(game).begin_link(0).unwrap();
}

/// First's exit 0 linked to `Second`, with Second's exit `exit` chosen.
fn open_second(game: &mut GameState) {
    start(game);
    editor(game).link_choose(RoomId::Second).unwrap();
}

#[test]
fn the_chooser_lists_saved_rooms_by_name_and_marks_the_room_being_edited() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    assert!(matches!(
        session.link.as_ref().unwrap().step(),
        Step::Choosing
    ));
    let candidates = session.link_candidates();
    assert_eq!(titles(&candidates), ["First", "Second", "Boss"]);
    assert!(candidates[0].is_origin && !candidates[1].is_origin);
    assert!(candidates.iter().all(|c| !c.unsaved));
}

#[test]
fn choosing_an_existing_room_opens_it_with_the_first_room_kept_aside() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let before = editor(&mut game).draft.clone();
    open_second(&mut game);
    let session = editor(&mut game);
    assert_eq!(session.id, RoomId::Second);
    assert_eq!(
        session.link_banner().as_deref(),
        Some("Linking First exit to Second")
    );
    assert!(!session.dirty, "B opens clean");
    // A waits, untouched, among the parked drafts.
    let parked = session
        .parked
        .iter()
        .find(|w| w.id == RoomId::First)
        .unwrap();
    assert_eq!(parked.draft, before);
    assert!(session.link.as_ref().unwrap().is_connecting());
    assert!(session.link_two_way(), "Two-way is on by default");
}

#[test]
fn a_new_room_is_created_named_and_opened() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    let prompt = session.link_prompt().unwrap();
    assert_eq!(prompt.text, "Room 1", "the default name is suggested");
    session.link_name_room("  Cellar ").unwrap();

    assert_eq!(session.id, RoomId::Custom(0));
    assert_eq!(session.draft.layout.name.as_deref(), Some("Cellar"));
    assert!(session.new && session.dirty);
    assert!(session.draft.layout.exits.is_empty());
    assert_eq!(
        session.link_banner().as_deref(),
        Some("Linking First exit to Cellar")
    );
    // The id is reserved: another new room would not get it.
    assert_ne!(session.free_room_id(), RoomId::Custom(0));
    assert!(session.path.ends_with("room_0.ron"));
}

#[test]
fn a_bad_or_taken_name_keeps_the_name_window_open() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    for bad in ["", "Second", "boss", "a/b", &"x".repeat(33)] {
        assert!(session.link_name_room(bad).is_err(), "{bad:?}");
        assert!(session.link_prompt().is_some(), "{bad:?}");
        assert_eq!(session.id, RoomId::First);
    }
    // A name needs to differ from the unsaved draft the flow started in, too.
    session.link_cancel();
    session.link_cancel();
    session.set_room_name_text("Entrance").unwrap();
    session.begin_link(0).unwrap();
    session.link_new_room().unwrap();
    assert!(session.link_name_room("entrance").is_err());
    session.link_name_room("Cellar").unwrap();
}

#[test]
fn the_room_the_exit_is_in_cannot_be_chosen() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    let before = session.draft.clone();
    let error = session.link_choose(RoomId::First).unwrap_err();
    assert!(
        error.contains("cannot lead to the room it is in"),
        "{error}"
    );
    assert_eq!(session.message, error, "the editor shows the reason");
    assert!(matches!(
        session.link.as_ref().unwrap().step(),
        Step::Choosing
    ));
    assert_eq!(session.id, RoomId::First);
    assert_eq!(session.draft, before);
}

#[test]
fn connecting_to_an_existing_exit_sets_both_sides() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let linked = session.link_connect(0).unwrap();
    assert!(linked.two_way);
    assert_eq!((linked.from, linked.to), (RoomId::First, RoomId::Second));

    // The editor is back in A, with A's exit leading to B next to the chosen exit.
    assert_eq!(session.id, RoomId::First);
    assert!(session.link.is_none());
    assert_eq!(session.message, "Linked First <-> Second");
    let a_exit = session.draft.layout.exits[0].clone();
    assert_eq!(a_exit.target_room, RoomId::Second);
    let second = layout_of(session, RoomId::Second);
    assert_eq!(arrival_error(&second, Vec2::from(a_exit.spawn)), None);
    // Second's chosen exit is its left one; the arrival is just inside its inner face.
    let zone = second.exits[0].exit().zone(second.rect());
    assert!(
        (a_exit.spawn.0 - 16.0 - zone.right()).abs() < 12.0,
        "{a_exit:?} {zone:?}"
    );

    // B's exit leads back to A, arriving next to A's exit.
    let b_exit = second.exits[0].clone();
    assert_eq!(b_exit.target_room, RoomId::First);
    assert_eq!(
        arrival_error(&session.draft.layout, Vec2::from(b_exit.spawn)),
        None
    );
    let a_zone = session.draft.layout.exits[0]
        .exit()
        .zone(session.draft.layout.rect());
    assert!(
        (b_exit.spawn.0 + 16.0 - a_zone.x).abs() < 12.0,
        "{b_exit:?} {a_zone:?}"
    );
    // A and B are both unsaved; B waits in the parked drafts.
    assert!(session.dirty);
    let parked = session
        .parked
        .iter()
        .find(|w| w.id == RoomId::Second)
        .unwrap();
    assert!(parked.dirty && !parked.new);
}

#[test]
fn a_new_exit_is_placed_in_the_target_and_connected_two_way() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let exits_before = session.draft.layout.exits.len();
    // Near Second's top edge, in the middle.
    let index = session.link_place_exit(Vec2::new(480.0, 44.0)).unwrap();
    assert_eq!(index, exits_before);
    assert_eq!(session.link_placed_exit(), Some(index));
    let placed = session.draft.layout.exits[index].clone();
    assert_eq!(placed.edge, RoomEdge::Top);
    assert_eq!(placed.target_room, RoomId::First, "it leads back to A");
    assert!(session.link_two_way(), "forced on for a new exit");
    session.link_set_two_way(false);
    assert!(session.link_two_way(), "a new exit cannot be one-way");

    session.link_connect(index).unwrap();
    assert_eq!(session.id, RoomId::First);
    let second = layout_of(session, RoomId::Second);
    assert_eq!(second.exits.len(), exits_before + 1);
    let top = &second.exits[index];
    assert_eq!((top.edge, top.target_room), (RoomEdge::Top, RoomId::First));
    assert_eq!(
        arrival_error(&session.draft.layout, Vec2::from(top.spawn)),
        None
    );
    let a_exit = &session.draft.layout.exits[0];
    assert_eq!(a_exit.target_room, RoomId::Second);
    assert_eq!(arrival_error(&second, Vec2::from(a_exit.spawn)), None);
    // The arrival is under the new exit, not inside it.
    assert!(a_exit.spawn.1 > 40.0 + 32.0, "{a_exit:?}");
}

#[test]
fn placing_again_replaces_the_earlier_new_exit_and_choosing_another_drops_it() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let count = session.draft.layout.exits.len();
    session.link_place_exit(Vec2::new(480.0, 44.0)).unwrap();
    let index = session.link_place_exit(Vec2::new(300.0, 396.0)).unwrap();
    assert_eq!(
        session.draft.layout.exits.len(),
        count + 1,
        "only one placed exit"
    );
    assert_eq!(session.draft.layout.exits[index].edge, RoomEdge::Bottom);
    // Choosing an existing exit instead leaves the placed one out of the result.
    session.link_connect(1).unwrap();
    let second = layout_of(session, RoomId::Second);
    assert_eq!(second.exits.len(), count);
    assert_eq!(second.exits[1].target_room, RoomId::First);
}

#[test]
fn a_one_way_link_leaves_the_other_exit_alone() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let second_before = session.draft.clone();
    session.link_set_two_way(false);
    assert!(!session.link_two_way());
    let linked = session.link_connect(1).unwrap();
    assert!(!linked.two_way);
    assert_eq!(session.message, "Linked First -> Second");
    assert_eq!(session.draft.layout.exits[0].target_room, RoomId::Second);
    // B's chosen exit is untouched: it still leads where it did.
    let second = layout_of(session, RoomId::Second);
    assert_eq!(second.exits, second_before.layout.exits);
    assert_eq!(second.exits[1].target_room, RoomId::Boss);
    // A's exit arrives next to B's exit 1 (its right one).
    let zone = second.exits[1].exit().zone(second.rect());
    let spawn = session.draft.layout.exits[0].spawn;
    assert!(
        spawn.0 < zone.x && zone.x - spawn.0 < 40.0,
        "{spawn:?} {zone:?}"
    );
}

#[test]
fn replacing_a_link_names_the_old_target() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    // Boss's one exit leads to Second; link it to First's exit, which leads to Second too.
    game.edit_room(RoomId::Boss).unwrap();
    editor(&mut game).begin_link(0).unwrap();
    let session = editor(&mut game);
    session.link_choose(RoomId::Second).unwrap();
    // Second's right exit led to Boss; pointing it at Boss again changes nothing to name.
    let linked = session.link_connect(0).unwrap();
    assert_eq!(
        linked.replaced,
        Some(RoomId::First),
        "Second's left exit led to First"
    );
    assert_eq!(
        session.message,
        "Linked Boss <-> Second (Second's exit no longer leads to First)"
    );
    let second = layout_of(session, RoomId::Second);
    assert_eq!(second.exits[0].target_room, RoomId::Boss);
}

#[test]
fn an_exit_that_already_led_to_the_same_room_is_not_reported_as_replaced() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    // Second's left exit already leads to First.
    let linked = session.link_connect(0).unwrap();
    assert_eq!(linked.replaced, None);
    assert_eq!(session.message, "Linked First <-> Second");
}

#[test]
fn a_new_exit_never_reports_a_replaced_target() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let index = session.link_place_exit(Vec2::new(480.0, 44.0)).unwrap();
    let linked = session.link_connect(index).unwrap();
    assert_eq!(linked.replaced, None);
}

#[test]
fn connecting_without_an_exit_is_refused_and_the_flow_stays_open() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let error = session.link_connect(9).unwrap_err();
    assert!(error.contains("Select an exit in Second"), "{error}");
    assert_eq!(session.id, RoomId::Second);
    assert!(session.link.is_some());
    assert!(session.parked.iter().all(|w| !w.dirty));
}

#[test]
fn an_exit_placed_where_it_does_not_fit_is_refused() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    // Second has a solid block right at its left edge, where the click will land.
    session.rooms[1].1.terrain = vec![(80.0, 100.0, 60.0, 200.0)];
    session.link_choose(RoomId::Second).unwrap();
    let before = session.draft.clone();
    let error = session.link_place_exit(Vec2::new(84.0, 200.0)).unwrap_err();
    assert!(error.contains("inside terrain"), "{error}");
    assert_eq!(session.draft, before, "a refused exit changes nothing");
    assert_eq!(session.link_placed_exit(), None);
    assert!(!session.dirty);
    // The same click away from the block works.
    assert!(session.link_place_exit(Vec2::new(84.0, 360.0)).is_ok());
}

#[test]
fn an_exit_next_to_no_free_spot_cannot_be_connected() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    // A wall of solid in front of Second's left exit, deeper than the search reaches.
    session.rooms[1].1.terrain = vec![(112.0, 40.0, 600.0, 360.0)];
    session.link_choose(RoomId::Second).unwrap();
    let error = session.link_connect(0).unwrap_err();
    assert!(error.contains("no free spot"), "{error}");
    assert_eq!(session.id, RoomId::Second, "still in B, nothing linked");
    assert!(session.link.is_some());
    session.link_cancel();
    assert_eq!(session.draft.layout.exits[0].target_room, RoomId::Second);
}

#[test]
fn an_arrival_is_never_inside_an_exit_wherever_the_exit_is_placed() {
    let temp = TempAssets::new();
    for (x, y) in [
        (90.0, 120.0),
        (90.0, 396.0),
        (870.0, 60.0),
        (870.0, 380.0),
        (200.0, 42.0),
        (700.0, 42.0),
        (200.0, 398.0),
        (700.0, 398.0),
        (480.0, 220.0),
    ] {
        let mut game = editing(&temp);
        open_second(&mut game);
        let session = editor(&mut game);
        let index = session.link_place_exit(Vec2::new(x, y)).unwrap();
        session.link_connect(index).unwrap();
        let second = layout_of(session, RoomId::Second);
        let a_spawn = Vec2::from(session.draft.layout.exits[0].spawn);
        assert_eq!(
            arrival_error(&second, a_spawn),
            None,
            "({x}, {y}) into Second"
        );
        let b_spawn = Vec2::from(second.exits[index].spawn);
        assert_eq!(
            arrival_error(&session.draft.layout, b_spawn),
            None,
            "({x}, {y}) into First"
        );
    }
}

/// The flow can be cancelled at any step with every draft as it was.
#[test]
fn cancelling_the_chooser_changes_nothing() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = session.draft.clone();
    session.begin_link(0).unwrap();
    session.link_cancel();
    assert!(session.link.is_none());
    assert_eq!(session.draft, before);
    assert!(!session.dirty && session.parked.is_empty());
    assert_eq!(session.id, RoomId::First);
}

#[test]
fn cancelling_the_chooser_removes_a_just_placed_exit_and_restores_the_unsaved_flag() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = session.draft.clone();
    let index = session.begin_link_new_exit(RoomEdge::Top).unwrap();
    assert_eq!(index, before.layout.exits.len());
    assert_eq!(
        session.draft.layout.exits.len(),
        before.layout.exits.len() + 1
    );
    assert!(session.dirty);
    session.link_cancel();
    assert_eq!(session.draft, before, "the placed exit is gone");
    assert!(!session.dirty, "and the session is as clean as it was");

    // An already unsaved session stays unsaved.
    session.set_room_name_text("Entrance").unwrap();
    let before = session.draft.clone();
    session.begin_link_new_exit(RoomEdge::Left).unwrap();
    session.link_cancel();
    assert_eq!(session.draft, before);
    assert!(session.dirty);
}

#[test]
fn cancelling_the_name_window_goes_back_to_the_chooser_and_a_second_cancel_ends_the_flow() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = session.draft.clone();
    session.begin_link_new_exit(RoomEdge::Top).unwrap();
    session.link_new_room().unwrap();
    assert!(session.link_prompt().is_some());
    session.link_cancel();
    assert!(matches!(
        session.link.as_ref().unwrap().step(),
        Step::Choosing
    ));
    assert!(session.link_prompt().is_none());
    // The chooser can still be used after backing out of the name window.
    session.link_cancel();
    assert!(session.link.is_none());
    assert_eq!(session.draft, before);
    assert!(session.parked.is_empty());
}

#[test]
fn cancelling_in_the_target_room_returns_to_the_first_room_with_nothing_changed() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let before = editor(&mut game).draft.clone();
    open_second(&mut game);
    let session = editor(&mut game);
    let index = session.link_place_exit(Vec2::new(480.0, 44.0)).unwrap();
    assert!(session.draft.layout.exits.len() > index);
    session.link_cancel();
    assert_eq!(session.id, RoomId::First);
    assert_eq!(session.draft, before);
    assert!(!session.dirty);
    assert!(session.parked.is_empty(), "Second's placed exit is dropped");
    assert!(session.link.is_none());
    assert_eq!(layout_of(session, RoomId::Second).exits.len(), 2);
}

#[test]
fn cancelling_in_the_target_room_takes_out_the_exit_that_started_the_flow() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = session.draft.clone();
    session.begin_link_new_exit(RoomEdge::Bottom).unwrap();
    session.link_choose(RoomId::Second).unwrap();
    session.link_cancel();
    assert_eq!(session.id, RoomId::First);
    assert_eq!(session.draft, before);
    assert!(!session.dirty);
}

#[test]
fn cancelling_discards_a_room_the_flow_created_and_frees_its_id() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    assert_eq!(session.id, RoomId::Custom(0));
    session.link_cancel();
    assert_eq!(session.id, RoomId::First);
    assert!(session.parked.is_empty(), "the new room is gone");
    assert_eq!(
        session.free_room_id(),
        RoomId::Custom(0),
        "its id is free again"
    );
    assert_eq!(
        session.suggested_room_name(),
        "Room 1",
        "and so is its name"
    );
    let candidates = session.link_candidates();
    assert_eq!(titles(&candidates), ["First", "Second", "Boss"]);
    assert!(
        candidates.iter().all(|c| !c.is_origin && !c.unsaved),
        "the discarded room is not offered and nothing is marked as the origin"
    );
    session.set_room_name_text("Cellar").unwrap();
}

#[test]
fn cancelling_puts_back_a_target_that_had_unsaved_changes_of_its_own() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    // First link: First <-> Second. Second is now an unsaved draft.
    open_second(&mut game);
    editor(&mut game).link_connect(0).unwrap();
    let second_before = editor(&mut game).parked[0].clone();
    // Second link, to the same room, then cancelled in it.
    editor(&mut game).begin_link(0).unwrap();
    let session = editor(&mut game);
    session.link_choose(RoomId::Second).unwrap();
    assert_eq!(session.id, RoomId::Second);
    session.link_place_exit(Vec2::new(480.0, 44.0)).unwrap();
    session.link_cancel();
    assert_eq!(session.id, RoomId::First);
    assert_eq!(session.parked.len(), 1);
    assert_eq!(session.parked[0].draft, second_before.draft);
    assert!(session.parked[0].dirty);
}

#[test]
fn a_flow_cannot_start_while_another_runs_and_each_step_needs_its_own_state() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    assert!(session.begin_link(9).is_err(), "no such exit");
    assert!(session.link_choose(RoomId::Second).is_err(), "no flow yet");
    assert!(session.link_connect(0).is_err());
    assert!(session.link_new_room().is_err());
    session.begin_link(0).unwrap();
    assert!(session.begin_link(0).is_err());
    assert!(session.begin_link_new_exit(RoomEdge::Left).is_err());
    assert!(session.link_connect(0).is_err(), "still choosing");
    assert!(session.link_place_exit(Vec2::ZERO).is_err());
    assert!(session.link_name_room("Cellar").is_err(), "not naming");
    session.link_choose(RoomId::Second).unwrap();
    assert!(session.link_choose(RoomId::Boss).is_err(), "already in B");
    assert!(session.link_new_room().is_err());
}

#[test]
fn nothing_reaches_the_disk_until_save() {
    let temp = TempAssets::new();
    let before = temp.room_files();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    let index = session.link_place_exit(Vec2::new(300.0, 440.0)).unwrap();
    assert_eq!(temp.room_files(), before, "no file while B is open");
    session.link_connect(index).unwrap();
    assert_eq!(temp.room_files(), before, "none after the link either");
    assert!(session.has_unsaved());
    drop(game);
    assert_eq!(
        temp.room_files(),
        before,
        "and none when the editor is left unsaved"
    );
}

#[test]
fn save_writes_both_rooms_and_a_new_room_is_created() {
    let temp = TempAssets::new();
    let before = temp.room_files();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    let index = session.link_place_exit(Vec2::new(30.0, 300.0)).unwrap();
    session.link_connect(index).unwrap();
    game.save_editor().unwrap();

    let after = temp.room_files();
    let names: Vec<_> = after.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["boss.ron", "first.ron", "room_0.ron", "second.ron"]);
    for file in ["boss.ron", "second.ron"] {
        let old = before.iter().find(|(name, _)| name == file).unwrap();
        let new = after.iter().find(|(name, _)| name == file).unwrap();
        assert_eq!(old, new, "{file} is not part of the link");
    }
    let first = temp.document("first.ron").layout.unwrap();
    let cellar = temp.document("room_0.ron").layout.unwrap();
    assert_eq!(cellar.name.as_deref(), Some("Cellar"));
    assert_eq!(first.exits.last().unwrap().target_room, RoomId::Custom(0));
    assert_eq!(cellar.exits.len(), 1);
    assert_eq!(cellar.exits[0].target_room, RoomId::First);
    // The session is clean and holds nothing parked any more.
    let session = game.editor().unwrap();
    assert!(!session.has_unsaved() && session.parked.is_empty());
    assert!(!session.new && session.unsaved_rooms().is_empty());
}

#[test]
fn both_rooms_survive_a_restart_and_the_player_can_walk_through_both_exits() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    editor(&mut game).link_connect(1).unwrap();
    game.save_editor().unwrap();
    let a_exit = editor(&mut game).draft.layout.exits[0].clone();
    let b_exit = layout_of(editor(&mut game), RoomId::Second).exits[1].clone();
    drop(game);

    let restarted = temp.game();
    let mut again = restarted;
    again.toggle_editor().unwrap();
    let session = editor(&mut again);
    assert_eq!(session.draft.layout.exits[0], a_exit);
    assert_eq!(layout_of(session, RoomId::Second).exits[1], b_exit);
    assert_eq!(a_exit.target_room, RoomId::Second);
    assert_eq!(b_exit.target_room, RoomId::First);
    // The files themselves say the same, through RON.
    let first: RoomEnemies = temp.document("first.ron");
    assert_eq!(first.layout.unwrap().exits[0], a_exit);
    let second = temp.document("second.ron").layout.unwrap();
    assert_eq!(second.exits[1], b_exit);
}

#[test]
fn saving_stops_everything_when_one_new_room_cannot_be_written() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    let index = session.link_place_exit(Vec2::new(30.0, 300.0)).unwrap();
    session.link_connect(index).unwrap();
    // The new room's file appears behind the editor's back.
    std::fs::copy(
        temp.0.join("rooms/second.ron"),
        temp.0.join("rooms/room_0.ron"),
    )
    .unwrap();
    let before = temp.room_files();
    assert!(game.save_editor().is_err());
    assert_eq!(temp.room_files(), before, "neither room was written");
    assert!(
        game.editor().unwrap().has_unsaved(),
        "and the work is still there"
    );
}

#[test]
fn saving_validates_every_touched_room_before_writing_any() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    editor(&mut game).link_connect(0).unwrap();
    // A's draft is made invalid after the link: its spawn leaves the room.
    editor(&mut game).draft.layout.spawn = (5000.0, 5000.0);
    let before = temp.room_files();
    assert!(game.save_editor().is_err());
    assert_eq!(temp.room_files(), before, "Second was not written either");
    // Fixing it lets the save through, and both files change.
    editor(&mut game).draft.layout.spawn = (180.0, 436.0);
    game.save_editor().unwrap();
    assert_ne!(temp.room_files(), before);
}

#[test]
fn saving_is_refused_while_a_link_is_open() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let before = temp.room_files();
    let error = game.save_editor().unwrap_err();
    assert!(error.contains("exit link"), "{error}");
    editor(&mut game).link_cancel();
    game.save_editor().unwrap();
    assert_eq!(temp.room_files().len(), before.len());
}

#[test]
fn a_new_unsaved_room_can_be_chosen_by_a_later_link() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    let index = session.link_place_exit(Vec2::new(30.0, 300.0)).unwrap();
    session.link_connect(index).unwrap();

    // A second exit of First is linked to the same unsaved room, by name.
    let session = editor(&mut game);
    session.begin_link_new_exit(RoomEdge::Top).unwrap();
    let candidates = session.link_candidates();
    assert_eq!(titles(&candidates), ["First", "Second", "Boss", "Cellar"]);
    let cellar = candidates.iter().find(|c| c.title == "Cellar").unwrap();
    assert!(cellar.unsaved);
    session.link_choose(cellar.id).unwrap();
    assert_eq!(session.id, RoomId::Custom(0));
    assert_eq!(session.draft.layout.name.as_deref(), Some("Cellar"));
    assert!(session.new, "still a new room");
    // The room can be named only once: the chooser knows it as an existing room.
    session.link_connect(0).unwrap();
    assert_eq!(session.id, RoomId::First);
    game.save_editor().unwrap();
    let first = temp.document("first.ron").layout.unwrap();
    let to_cellar = first
        .exits
        .iter()
        .filter(|e| e.target_room == RoomId::Custom(0))
        .count();
    assert_eq!(to_cellar, 2);
}

#[test]
fn unsaved_rooms_are_listed_by_name() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    assert!(!session.has_unsaved());
    assert!(session.unsaved_rooms().is_empty());
    session.begin_link(0).unwrap();
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    let index = session.link_place_exit(Vec2::new(30.0, 300.0)).unwrap();
    session.link_connect(index).unwrap();
    assert!(session.has_unsaved());
    assert_eq!(session.unsaved_rooms(), ["First", "Cellar (new)"]);
}

#[test]
fn starting_a_new_room_discards_the_parked_drafts() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    editor(&mut game).link_connect(0).unwrap();
    assert!(!editor(&mut game).parked.is_empty());
    editor(&mut game).new_room_named("Attic").unwrap();
    assert!(editor(&mut game).parked.is_empty());
    assert!(editor(&mut game).link.is_none());
}

#[test]
fn names_are_checked_against_parked_drafts() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    let index = session.link_place_exit(Vec2::new(30.0, 300.0)).unwrap();
    session.link_connect(index).unwrap();
    // Cellar is a parked unsaved room; First cannot take its name.
    assert!(session.set_room_name_text("cellar").is_err());
    assert_eq!(session.title_of(RoomId::Custom(0)), "Cellar");
}

#[test]
fn the_catalog_includes_unsaved_rooms_so_exits_can_point_at_them() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let session = editor(&mut game);
    session.link_new_room().unwrap();
    session.link_name_room("Cellar").unwrap();
    // While B is open, the catalog has B's draft and A's parked draft.
    let ids: Vec<_> = session.catalog().iter().map(|(id, _)| *id).collect();
    assert!(ids.contains(&RoomId::Custom(0)) && ids.contains(&RoomId::First));
    let index = session.link_place_exit(Vec2::new(30.0, 300.0)).unwrap();
    session.link_connect(index).unwrap();
    let draft = RoomDraft {
        enemies: Vec::new(),
        boss: None,
        layout: session.draft.layout.clone(),
    };
    draft
        .validate(&session.definitions, &session.catalog())
        .expect("A's exit to the unsaved room validates against the catalog");
}

#[test]
fn a_session_that_never_links_behaves_as_before() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    assert!(session.link.is_none() && session.parked.is_empty());
    assert!(!session.has_unsaved());
    session.set_room_name_text("Entrance").unwrap();
    assert!(session.has_unsaved());
    assert_eq!(session.unsaved_rooms(), ["Entrance"]);
    game.save_editor().unwrap();
}

#[test]
fn a_finished_link_changes_only_the_linked_exit_in_the_first_room() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let before = editor(&mut game).draft.clone();
    open_second(&mut game);
    editor(&mut game).link_connect(1).unwrap();
    let after = editor(&mut game).draft.clone();
    let mut expected = before.clone();
    expected.layout.exits[0].target_room = after.layout.exits[0].target_room;
    expected.layout.exits[0].spawn = after.layout.exits[0].spawn;
    assert_eq!(after, expected, "A's draft differs only in the linked exit");
    assert_eq!(after.layout.exits[0].target_room, RoomId::Second);
    assert_ne!(after.layout.exits[0].spawn, before.layout.exits[0].spawn);
}

#[test]
fn testing_a_room_is_refused_while_a_link_is_open() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    start(&mut game);
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("exit link"), "{error}");
    assert_eq!(editor(&mut game).message, error);
    assert!(game.editor().is_some() && !game.testing_room());
    editor(&mut game).link_cancel();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
}
