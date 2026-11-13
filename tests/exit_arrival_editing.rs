//! Editing where an exit arrives without leaving the room: the arrivals of other rooms'
//! exits that lead into the room being edited (listed, moved, saved), and the arrival the
//! link step lets the user drag before pressing Connect.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{RoomLayout, Selection};
use hollow_steps::editor_inbound::InboundArrival;
use hollow_steps::editor_session::EditorSession;
use hollow_steps::exit_arrival::{arrival_error, arrival_next_to};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEdge, RoomEnemies, RoomId};
use macroquad::math::Vec2;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-arrival-{}-{}",
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

    fn layout(&self, file: &str) -> RoomLayout {
        let document: RoomEnemies =
            ron::de::from_bytes(&std::fs::read(self.0.join("rooms").join(file)).unwrap()).unwrap();
        document.layout.unwrap()
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

fn at(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

/// Second's exit 0 leads to First and arrives at (860, 436); editing First, that is the
/// only arrival from another room.
const SECOND_TO_FIRST: Selection = Selection::Inbound {
    room: RoomId::Second,
    exit: 0,
};

/// A solid over the arrival of Second's exit 0 in First.
fn cover_the_arrival(session: &mut EditorSession) {
    session
        .draft
        .layout
        .terrain
        .push((820.0, 400.0, 80.0, 60.0));
}

/// First's exit 0 linked to Second, with Second open and nothing chosen yet.
fn open_second(game: &mut GameState) {
    editor(game).begin_link(0).unwrap();
    editor(game).link_choose(RoomId::Second).unwrap();
}

// ---- the arrivals of other rooms' exits -------------------------------------------------

#[test]
fn the_exits_of_other_rooms_that_lead_here_are_listed_with_their_room() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    assert_eq!(
        editor(&mut game).inbound_arrivals(),
        [InboundArrival {
            room: RoomId::Second,
            exit: 0,
            position: at(860.0, 436.0),
            title: "Second".into(),
            error: None,
        }]
    );
}

#[test]
fn every_exit_into_the_room_is_listed_and_the_rooms_own_exits_are_not() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.edit_room(RoomId::Second).unwrap();
    let found: Vec<_> = editor(&mut game)
        .inbound_arrivals()
        .into_iter()
        .map(|inbound| (inbound.room, inbound.exit, inbound.position))
        .collect();
    assert_eq!(
        found,
        [
            (RoomId::First, 0, at(160.0, 376.0)),
            (RoomId::Boss, 0, at(780.0, 376.0)),
        ]
    );
}

#[test]
fn an_exit_that_leads_back_into_its_own_room_is_not_listed() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    session
        .draft
        .add_exit(RoomEdge::Top, RoomId::First, at(480.0, 200.0));
    assert_eq!(session.inbound_arrivals().len(), 1);
}

#[test]
fn an_arrival_that_does_not_fit_here_any_more_says_why() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    cover_the_arrival(session);
    let inbound = session.inbound_arrivals();
    assert_eq!(inbound[0].error, Some("inside a solid"));
}

#[test]
fn a_rooms_unsaved_draft_is_listed_in_place_of_its_saved_exit() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    session
        .move_inbound_arrival(RoomId::Second, 0, at(700.0, 436.0))
        .unwrap();
    // The saved Second still says (860, 436); the list follows the parked draft.
    assert_eq!(
        session
            .rooms
            .iter()
            .find(|(id, _)| *id == RoomId::Second)
            .unwrap()
            .1
            .exits[0]
            .spawn,
        (860.0, 436.0)
    );
    assert_eq!(session.inbound_arrivals().len(), 1);
    assert_eq!(session.inbound_arrivals()[0].position, at(700.0, 436.0));
}

// ---- moving one --------------------------------------------------------------------------

#[test]
fn moving_an_inbound_arrival_changes_the_other_room_and_leaves_this_one_alone() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = session.draft.clone();
    session
        .drag_selection(SECOND_TO_FIRST, at(700.0, 436.0))
        .unwrap();

    assert_eq!(session.draft, before, "this room's draft is untouched");
    assert!(!session.dirty, "and it is not marked unsaved");
    assert_eq!(session.parked.len(), 1);
    let second = &session.parked[0];
    assert_eq!(second.id, RoomId::Second);
    assert!(second.dirty && !second.new);
    assert_eq!(second.draft.layout.exits[0].spawn, (700.0, 436.0));
    assert_eq!(second.draft.layout.exits[0].target_room, RoomId::First);
    // Nothing else about Second changed.
    assert_eq!(second.draft.layout.exits[1].spawn, (160.0, 436.0));
    assert!(session.has_unsaved());
    assert_eq!(session.unsaved_rooms(), ["Second"]);
}

#[test]
fn moving_it_again_keeps_the_one_parked_copy() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    for x in [700.0, 650.0, 600.0] {
        session
            .move_selection(SECOND_TO_FIRST, at(x, 436.0))
            .unwrap();
    }
    assert_eq!(session.parked.len(), 1);
    assert_eq!(
        session.parked[0].draft.layout.exits[0].spawn,
        (600.0, 436.0)
    );
}

#[test]
fn a_spot_the_player_does_not_fit_is_refused_and_nothing_is_parked() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    cover_the_arrival(session);
    for (spot, reason) in [
        (at(5000.0, 5000.0), "outside the room"),
        (at(860.0, 436.0), "inside a solid"),
        (at(940.0, 410.0), "inside an exit"),
    ] {
        let error = session.move_selection(SECOND_TO_FIRST, spot).unwrap_err();
        assert!(error.contains(reason), "{spot}: {error}");
        assert!(session.parked.is_empty(), "{spot}: nothing was parked");
    }
    assert!(!session.has_unsaved() || session.dirty);
    let error = session
        .move_selection(SECOND_TO_FIRST, at(f32::NAN, 100.0))
        .unwrap_err();
    assert!(error.contains("finite"), "{error}");
    assert!(session.parked.is_empty());
}

#[test]
fn only_an_exit_that_leads_here_can_be_moved() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let spot = at(700.0, 436.0);
    // Boss's exit 0 leads to Second, Second's exit 1 to Boss, and exit 7 does not exist.
    for (room, exit) in [
        (RoomId::Boss, 0),
        (RoomId::Second, 1),
        (RoomId::Second, 7),
        (RoomId::First, 0),
        (RoomId::Custom(9), 0),
    ] {
        assert!(
            session
                .move_selection(Selection::Inbound { room, exit }, spot)
                .is_err(),
            "{room:?} exit {exit}"
        );
    }
    assert!(session.parked.is_empty() && !session.has_unsaved());
}

#[test]
fn an_inbound_arrival_cannot_be_moved_while_a_link_is_open() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    editor(&mut game).begin_link(0).unwrap();
    let session = editor(&mut game);
    let error = session
        .move_inbound_arrival(RoomId::Second, 0, at(700.0, 436.0))
        .unwrap_err();
    assert!(error.contains("link"), "{error}");
    assert!(session.parked.is_empty());
}

#[test]
fn dragging_marks_this_room_unsaved_only_for_what_belongs_to_it() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    session
        .drag_selection(SECOND_TO_FIRST, at(700.0, 436.0))
        .unwrap();
    assert!(!session.dirty);
    session
        .drag_selection(Selection::Spawn, at(200.0, 436.0))
        .unwrap();
    assert!(session.dirty, "the room's own spawn is this room's edit");
}

#[test]
fn the_draft_itself_does_not_move_arrivals_it_does_not_own() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = session.draft.clone();
    for selection in [SECOND_TO_FIRST, Selection::LinkArrival(0)] {
        assert!(session
            .draft
            .move_to(selection, at(700.0, 436.0), &session.definitions)
            .is_err());
    }
    assert_eq!(session.draft, before);
}

// ---- saving ------------------------------------------------------------------------------

#[test]
fn save_writes_the_moved_arrival_into_the_other_rooms_file() {
    let temp = TempAssets::new();
    let before = temp.room_files();
    let mut game = editing(&temp);
    editor(&mut game)
        .move_selection(SECOND_TO_FIRST, at(700.0, 436.0))
        .unwrap();
    assert_eq!(temp.room_files(), before, "nothing is written by the drag");
    game.save_editor().unwrap();

    let second = temp.layout("second.ron");
    assert_eq!(second.exits[0].spawn, (700.0, 436.0));
    assert_eq!(second.exits[1].spawn, (160.0, 436.0));
    let after = temp.room_files();
    let boss =
        |files: &[(String, Vec<u8>)]| files.iter().find(|(name, _)| name == "boss.ron").cloned();
    assert_eq!(
        boss(&before),
        boss(&after),
        "an unrelated room is not rewritten"
    );
    let session = game.editor().unwrap();
    assert!(!session.has_unsaved() && session.parked.is_empty());
    assert_eq!(session.inbound_arrivals()[0].position, at(700.0, 436.0));

    // And it is still there after a restart.
    drop(game);
    let mut again = temp.game();
    again.toggle_editor().unwrap();
    assert_eq!(
        editor(&mut again).inbound_arrivals()[0].position,
        at(700.0, 436.0)
    );
}

#[test]
fn save_refuses_everything_when_an_exit_was_drawn_over_the_moved_arrival() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    editor(&mut game)
        .move_selection(SECOND_TO_FIRST, at(700.0, 436.0))
        .unwrap();
    // An exit is added along the floor, right where the player would arrive.
    let session = editor(&mut game);
    session
        .draft
        .add_exit(RoomEdge::Bottom, RoomId::Second, at(160.0, 376.0));
    let exit = session.draft.layout.exits.last_mut().unwrap();
    (exit.offset, exit.length) = (600.0, 200.0);
    session.dirty = true;
    assert_eq!(
        session.inbound_arrivals()[0].error,
        Some("inside an exit"),
        "the marker already says so"
    );
    let before = temp.room_files();
    let error = game.save_editor().unwrap_err();
    assert!(
        error.contains("second.ron") && error.contains("overlaps an exit"),
        "names the room file with the problem: {error}"
    );
    assert_eq!(temp.room_files(), before, "neither room was written");
    assert!(game.editor().unwrap().has_unsaved(), "and the work is kept");

    // Moving the arrival clear of the exit lets the save through.
    editor(&mut game)
        .move_selection(SECOND_TO_FIRST, at(400.0, 436.0))
        .unwrap();
    game.save_editor().unwrap();
    assert_eq!(temp.layout("second.ron").exits[0].spawn, (400.0, 436.0));
}

// ---- the arrival in the link step --------------------------------------------------------

#[test]
fn without_dragging_the_arrival_is_the_automatic_spot_next_to_the_exit() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let automatic = arrival_next_to(&session.draft.layout, 1).unwrap();
    assert_eq!(session.link_arrival(1), Some(automatic));
    session.link_connect(1).unwrap();
    assert_eq!(
        Vec2::from(session.draft.layout.exits[0].spawn),
        automatic,
        "Connect arrives where the marker showed"
    );
}

#[test]
fn a_dragged_arrival_is_where_connect_sends_the_player() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    session
        .drag_selection(Selection::LinkArrival(1), at(600.0, 352.0))
        .unwrap();
    assert_eq!(session.link_arrival(1), Some(at(600.0, 352.0)));
    assert!(
        !session.dirty,
        "dragging it is not an edit of the open room yet"
    );
    session.link_connect(1).unwrap();
    assert_eq!(session.draft.layout.exits[0].spawn, (600.0, 352.0));
    assert_eq!(session.draft.layout.exits[0].target_room, RoomId::Second);

    game.save_editor().unwrap();
    assert_eq!(temp.layout("first.ron").exits[0].spawn, (600.0, 352.0));
}

#[test]
fn the_dragged_arrival_belongs_to_the_exit_it_was_set_for() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let automatic = session.link_arrival(0).unwrap();
    session.link_set_arrival(1, at(600.0, 352.0)).unwrap();
    assert_eq!(session.link_arrival(0), Some(automatic));
    session.link_connect(0).unwrap();
    assert_eq!(Vec2::from(session.draft.layout.exits[0].spawn), automatic);
}

#[test]
fn a_spot_the_player_does_not_fit_cannot_be_set_and_the_old_one_stays() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    session.link_set_arrival(1, at(600.0, 352.0)).unwrap();
    for spot in [at(5000.0, 5000.0), at(860.0, 352.0), at(f32::INFINITY, 0.0)] {
        assert!(session.link_set_arrival(1, spot).is_err(), "{spot}");
    }
    assert_eq!(session.link_arrival(1), Some(at(600.0, 352.0)));
    assert!(session.link_set_arrival(9, at(600.0, 352.0)).is_err());
}

#[test]
fn an_arrival_the_exit_was_dragged_over_falls_back_to_the_automatic_spot() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    session.link_set_arrival(1, at(850.0, 150.0)).unwrap();
    assert_eq!(session.link_arrival(1), Some(at(850.0, 150.0)));
    // The exit is dragged up the wall until it covers the arrival.
    session
        .drag_selection(Selection::Exit(1), at(880.0, 150.0))
        .unwrap();
    let fallback = session.link_arrival(1).unwrap();
    assert_ne!(fallback, at(850.0, 150.0));
    assert_eq!(arrival_error(&session.draft.layout, fallback), None);
    session.link_connect(1).unwrap();
    assert_eq!(Vec2::from(session.draft.layout.exits[0].spawn), fallback);
}

#[test]
fn an_unchosen_placed_exit_does_not_disturb_the_arrival_of_the_chosen_one() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let placed = session.link_place_exit(at(30.0, 300.0)).unwrap();
    assert_eq!(placed, 2);
    session.link_set_arrival(1, at(600.0, 352.0)).unwrap();
    assert_eq!(session.link_arrival(1), Some(at(600.0, 352.0)));
    session.link_connect(1).unwrap();
    assert_eq!(session.draft.layout.exits[0].spawn, (600.0, 352.0));
}

#[test]
fn the_arrival_next_to_a_placed_exit_can_be_dragged_too() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let placed = session.link_place_exit(at(30.0, 300.0)).unwrap();
    session.link_set_arrival(placed, at(400.0, 300.0)).unwrap();
    let linked = session.link_connect(placed).unwrap();
    assert!(linked.two_way, "a placed exit always leads back");
    assert_eq!(session.draft.layout.exits[0].spawn, (400.0, 300.0));
}

#[test]
fn placing_the_exit_again_forgets_the_arrival_set_for_the_one_it_replaces() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    let session = editor(&mut game);
    let placed = session.link_place_exit(at(30.0, 300.0)).unwrap();
    session.link_set_arrival(placed, at(400.0, 300.0)).unwrap();
    let again = session.link_place_exit(at(30.0, 200.0)).unwrap();
    assert_eq!(again, placed, "the replacement takes the same index");
    assert_eq!(
        session.link_arrival(again),
        Some(arrival_next_to(&session.draft.layout, again).unwrap())
    );
}

#[test]
fn cancelling_the_link_forgets_the_dragged_arrival() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    open_second(&mut game);
    editor(&mut game)
        .link_set_arrival(1, at(600.0, 352.0))
        .unwrap();
    editor(&mut game).link_cancel();
    open_second(&mut game);
    let session = editor(&mut game);
    assert_eq!(
        session.link_arrival(1),
        Some(arrival_next_to(&session.draft.layout, 1).unwrap())
    );
}

#[test]
fn there_is_no_link_arrival_outside_the_linking_step() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    assert_eq!(session.link_arrival(0), None);
    assert!(session.link_set_arrival(0, at(600.0, 352.0)).is_err());
    session.begin_link(0).unwrap();
    assert_eq!(session.link_arrival(0), None, "the chooser is not B");
    assert!(session.link_set_arrival(0, at(600.0, 352.0)).is_err());
}
