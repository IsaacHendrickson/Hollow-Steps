//! The editor's generated-dungeon preview (`docs/dungeon-gen.md`) as a model with no window: the
//! seed box and Roll next, the plan a seed builds from the pieces (the fixture pool of
//! `support::pieces`), the map with its locks, where the keys and abilities lie, and the reason
//! when there is no dungeon. The window is tested with the editor panel.
mod support;

use hollow_steps::dungeon_gen::{generate, DungeonPlan};
use hollow_steps::dungeon_map::{MapLayout, MapRoomInput};
use hollow_steps::dungeon_preview::{
    next_seed, outcome_for, DungeonPreview, Outcome, PlanView, PreviewInputs,
};
use hollow_steps::editor::RoomLayout;
use hollow_steps::game::GameState;
use hollow_steps::piece::{PieceInfo, PieceRole};
use hollow_steps::room::RoomId;
use hollow_steps::seed::format_seed;
use std::collections::HashSet;
use support::pieces::{self, BOSS_A, BOSS_B, HUB, START_A, START_B};

/// The order rooms are listed in: the first three ids, then the custom ones by number.
fn rank(id: RoomId) -> usize {
    match id {
        RoomId::First => 0,
        RoomId::Second => 1,
        RoomId::Boss => 2,
        RoomId::Custom(number) => 3 + number,
    }
}

fn catalog(pool: Vec<(RoomId, RoomLayout)>) -> Vec<(RoomId, RoomLayout)> {
    let mut rooms = pool;
    rooms.push(pieces::hub());
    rooms
}

fn inputs() -> PreviewInputs {
    PreviewInputs::of(catalog(pieces::pool()), pieces::config(), pieces::rules())
}

fn view_of(seed: u64) -> PlanView {
    match outcome_for(seed, &inputs()) {
        Outcome::Plan(view) => view,
        other => panic!("seed {seed}: {other:?}"),
    }
}

fn plan_of(seed: u64) -> DungeonPlan {
    let hub = pieces::hub();
    generate(
        &pieces::pool(),
        &hub,
        &pieces::config(),
        &pieces::rules(),
        seed,
    )
    .unwrap()
}

fn piece_name(id: RoomId) -> String {
    pieces::pool()
        .into_iter()
        .find(|(piece, _)| *piece == id)
        .and_then(|(_, layout)| layout.name)
        .unwrap()
}

#[test]
fn the_inputs_are_the_pieces_in_id_order_and_the_hub() {
    let inputs = inputs();
    let pool = pieces::pool();
    assert_eq!(inputs.pieces.len(), pool.len());
    let order: Vec<usize> = inputs.pieces.iter().map(|(id, _)| rank(*id)).collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
    assert!(inputs
        .pieces
        .iter()
        .all(|(_, layout)| layout.piece.is_some()));
    assert_eq!(inputs.hub.as_ref().map(|(id, _)| *id), Ok(HUB));
    // The hub is not a piece, and a room list in another order gives the same inputs.
    assert!(inputs.pieces.iter().all(|(id, _)| *id != HUB));
    let mut reversed = catalog(pieces::pool());
    reversed.reverse();
    assert_eq!(
        PreviewInputs::of(reversed, pieces::config(), pieces::rules()),
        inputs
    );
}

#[test]
fn the_preview_shows_what_the_generator_builds_for_the_seed() {
    for seed in 0..12 {
        let plan = plan_of(seed);
        let view = view_of(seed);
        assert_eq!(view.seed, seed);
        assert_eq!(view.rooms, plan.rooms.len());
        assert_eq!(view.attempts, plan.attempts);
        let drawn: HashSet<_> = view.map.rooms.iter().map(|room| room.id).collect();
        let planned: HashSet<_> = plan.catalog().into_iter().map(|(id, _)| id).collect();
        assert_eq!(drawn, planned, "seed {seed}: the hub and the plan's rooms");
        assert_eq!(view.map.rooms.len(), plan.rooms.len() + 1);
        assert!(view.map.rooms.iter().all(|room| !room.current));
        assert!(view
            .summary()
            .starts_with(&format!("Seed {}", format_seed(seed))));
    }
}

#[test]
fn rooms_are_named_by_their_piece() {
    for seed in 0..12 {
        let plan = plan_of(seed);
        let view = view_of(seed);
        let named = |id: RoomId| {
            view.map
                .rooms
                .iter()
                .find(|room| room.id == id)
                .map(|room| room.name.clone())
                .unwrap()
        };
        assert_eq!(named(HUB), "Hub");
        for room in &plan.rooms {
            assert_eq!(
                named(room.instance),
                piece_name(room.piece),
                "seed {seed}: {:?} is the piece {:?}",
                room.instance,
                room.piece
            );
        }
        // Whichever Start and Boss piece were drawn, the dungeon has one of each.
        let names: Vec<_> = view
            .map
            .rooms
            .iter()
            .map(|room| room.name.clone())
            .collect();
        let starts = [START_A, START_B].map(piece_name);
        let bosses = [BOSS_A, BOSS_B].map(piece_name);
        assert_eq!(names.iter().filter(|n| starts.contains(n)).count(), 1);
        assert_eq!(names.iter().filter(|n| bosses.contains(n)).count(), 1);
    }
}

#[test]
fn the_map_is_the_one_a_run_of_that_seed_would_show() {
    for seed in 0..12 {
        let plan = plan_of(seed);
        let rooms: Vec<MapRoomInput> = plan
            .catalog()
            .iter()
            .map(|(id, layout)| MapRoomInput::from_layout(*id, layout))
            .collect();
        let run_map = MapLayout::build(&rooms);
        let view = view_of(seed);
        let cells: Vec<_> = view
            .map
            .rooms
            .iter()
            .map(|room| (room.id, room.cell))
            .collect();
        let expected: Vec<_> = run_map
            .rooms
            .iter()
            .map(|room| (room.id, room.cell))
            .collect();
        assert_eq!(cells, expected, "seed {seed}");
        assert_eq!(view.map.links, run_map.links, "seed {seed}");
        assert!(
            run_map.conflicts.is_empty(),
            "seed {seed}: no two rooms share a cell"
        );
        assert!(view
            .map
            .rooms
            .iter()
            .all(|room| !room.conflict && !room.no_entrance));
    }
}

#[test]
fn locks_carry_their_key_numbers_and_each_key_is_listed_where_it_lies() {
    let (mut locked, mut with_ability) = (0, 0);
    for seed in 0..40 {
        let plan = plan_of(seed);
        let view = view_of(seed);
        let lines: Vec<String> = view.pickups.iter().map(|pickup| pickup.line()).collect();
        for link in view.map.links.iter().filter_map(|link| link.lock) {
            locked += 1;
            assert!(
                lines
                    .iter()
                    .any(|line| line.starts_with(&format!("Key {link} in "))),
                "seed {seed}: Key {link} locks an exit, so it lies somewhere: {lines:?}"
            );
        }
        // Every item that opens or crosses something is listed once, in a room of the dungeon,
        // by its piece's name.
        let lying: usize = plan
            .rooms
            .iter()
            .map(|room| {
                room.layout
                    .items
                    .iter()
                    .filter(|placed| placed.item.name() != "Amber Shard")
                    .count()
            })
            .sum();
        assert_eq!(view.pickups.len(), lying, "seed {seed}: {lines:?}");
        for pickup in &view.pickups {
            let rooms: Vec<_> = view.map.rooms.iter().map(|room| &room.name).collect();
            assert!(rooms.contains(&&pickup.room), "seed {seed}: {pickup:?}");
        }
        with_ability += usize::from(lines.iter().any(|line| line.starts_with("Dash")));
    }
    assert!(locked > 0, "some of these seeds lock the spine");
    assert!(
        with_ability > 0,
        "some of these seeds place the Dash ability"
    );
}

#[test]
fn the_same_seed_shows_the_same_plan_and_other_seeds_show_other_ones() {
    assert_eq!(view_of(7), view_of(7));
    let maps: HashSet<String> = (0..20)
        .map(|seed| format!("{:?}", view_of(seed).map))
        .collect();
    assert!(
        maps.len() >= 15,
        "only {} different maps in 20 seeds",
        maps.len()
    );
}

fn typed(text: &str) -> DungeonPreview {
    let mut preview = DungeonPreview::default();
    *preview.text_mut() = text.to_owned();
    preview.refresh(inputs());
    preview
}

#[test]
fn the_seed_box_takes_hex_with_or_without_0x_and_ignores_spaces_around_it() {
    for (text, seed) in [
        ("0", 0),
        ("ab", 0xab),
        ("AB", 0xab),
        ("0xab", 0xab),
        ("  00000000000000ab  ", 0xab),
        ("ffffffffffffffff", u64::MAX),
    ] {
        let preview = typed(text);
        assert_eq!(preview.seed(), Ok(seed), "{text:?}");
        assert!(
            matches!(preview.outcome(), Some(Outcome::Plan(view)) if view.seed == seed),
            "{text:?}: {:?}",
            preview.outcome()
        );
    }
}

#[test]
fn a_box_that_is_not_a_seed_says_why_and_draws_no_plan_until_it_is_fixed() {
    for text in ["", "0x", "xyz", "12 34", "11111111111111111", "-1"] {
        let mut preview = typed(text);
        let Some(Outcome::BadSeed(reason)) = preview.outcome() else {
            panic!("{text:?}: {:?}", preview.outcome());
        };
        assert!(!reason.is_empty(), "{text:?}");
        assert!(preview.seed().is_err(), "{text:?}");
        *preview.text_mut() = "2a".to_owned();
        preview.refresh(inputs());
        assert!(
            matches!(preview.outcome(), Some(Outcome::Plan(_))),
            "{text:?}"
        );
    }
}

#[test]
fn roll_next_puts_the_next_seed_in_the_box_and_the_plan_follows() {
    let mut preview = DungeonPreview::with_seed(5);
    preview.refresh(inputs());
    assert_eq!(preview.text(), format_seed(5));

    preview.roll_next();
    assert_eq!(preview.seed(), Ok(next_seed(5)));
    assert_ne!(next_seed(5), 5);
    assert_eq!(
        preview.text().len(),
        16,
        "sixteen digits, as a result screen shows them"
    );
    preview.refresh(inputs());
    assert!(matches!(
        preview.outcome(),
        Some(Outcome::Plan(view)) if view.seed == next_seed(5)
    ));

    // The same start rolls to the same next seed, and a long roll never comes back round.
    let mut seen = HashSet::new();
    let mut seed = 5;
    for _ in 0..200 {
        seed = next_seed(seed);
        assert!(seen.insert(seed));
    }
    assert_eq!(next_seed(5), next_seed(5));
}

#[test]
fn roll_next_from_a_box_that_holds_no_seed_starts_from_the_last_good_one() {
    let mut preview = DungeonPreview::with_seed(9);
    preview.refresh(inputs());
    *preview.text_mut() = "zz".to_owned();
    preview.refresh(inputs());
    assert!(matches!(preview.outcome(), Some(Outcome::BadSeed(_))));
    preview.roll_next();
    assert_eq!(preview.seed(), Ok(next_seed(9)));
}

#[test]
fn a_pool_that_cannot_build_says_what_the_generator_says() {
    let without_boss: Vec<_> = catalog(pieces::pool())
        .into_iter()
        .filter(|(id, _)| *id != BOSS_A && *id != BOSS_B)
        .collect();
    let inputs = PreviewInputs::of(without_boss, pieces::config(), pieces::rules());
    let expected = generate(
        &inputs.pieces,
        inputs.hub.as_ref().unwrap(),
        &inputs.config,
        &inputs.rules,
        3,
    )
    .unwrap_err()
    .to_string();
    assert_eq!(expected, "the pool has no Boss piece");
    assert_eq!(outcome_for(3, &inputs), Outcome::Failed(expected));

    let empty = PreviewInputs::of(vec![pieces::hub()], pieces::config(), pieces::rules());
    assert!(matches!(outcome_for(3, &empty), Outcome::Failed(text) if text.contains("Start")));
}

#[test]
fn rules_no_dungeon_can_meet_say_so() {
    let mut rules = pieces::rules();
    rules.keys.min = 9;
    rules.keys.max = 9;
    let inputs = PreviewInputs::of(catalog(pieces::pool()), pieces::config(), rules);
    let Outcome::Failed(text) = outcome_for(1, &inputs) else {
        panic!("nine keys cannot be built");
    };
    assert!(!text.is_empty());
}

#[test]
fn a_missing_hub_says_so_and_builds_nothing() {
    let mut no_hub = pieces::config();
    no_hub.hub_room = None;
    let inputs = PreviewInputs::of(catalog(pieces::pool()), no_hub, pieces::rules());
    let Outcome::Failed(text) = outcome_for(1, &inputs) else {
        panic!("no hub, no dungeon");
    };
    assert!(text.contains("hub_room"), "{text}");

    let gone: Vec<_> = pieces::pool();
    let inputs = PreviewInputs::of(gone, pieces::config(), pieces::rules());
    let Outcome::Failed(text) = outcome_for(1, &inputs) else {
        panic!("the hub room is not there");
    };
    assert!(text.contains("no such room"), "{text}");
}

#[test]
fn refreshing_builds_again_only_when_the_seed_or_the_rooms_changed() {
    let mut preview = DungeonPreview::with_seed(11);
    preview.refresh(inputs());
    let first = preview.outcome().cloned();
    assert!(matches!(first, Some(Outcome::Plan(_))));
    preview.refresh(inputs());
    assert_eq!(preview.outcome().cloned(), first, "nothing changed");

    // A piece taken out of the pool changes the answer, and putting it back gives it again.
    let without_boss: Vec<_> = catalog(pieces::pool())
        .into_iter()
        .filter(|(id, _)| *id != BOSS_A && *id != BOSS_B)
        .collect();
    preview.refresh(PreviewInputs::of(
        without_boss,
        pieces::config(),
        pieces::rules(),
    ));
    assert!(matches!(preview.outcome(), Some(Outcome::Failed(_))));
    preview.refresh(inputs());
    assert_eq!(preview.outcome().cloned(), first);
}

#[test]
fn the_note_appears_only_while_the_generator_is_switched_off() {
    let mut preview = DungeonPreview::with_seed(1);
    assert_eq!(preview.note(), None, "nothing built yet");
    preview.refresh(inputs());
    assert_eq!(preview.note(), None);

    let mut off = pieces::rules();
    off.enabled = false;
    preview.refresh(PreviewInputs::of(
        catalog(pieces::pool()),
        pieces::config(),
        off,
    ));
    assert!(preview
        .note()
        .is_some_and(|note| note.contains("generator.enabled")));
    // It still builds: the preview answers "what would this seed make", whatever the switch says.
    assert!(matches!(preview.outcome(), Some(Outcome::Plan(_))));
}

#[test]
fn the_session_gives_unsaved_drafts_and_the_game_gives_the_rules() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    let rules = game.variety_rules().generator;
    let config = *game.run_config();
    let session = game.editor_mut().unwrap();
    assert_eq!(session.generator, rules, "the game's generator rules");
    assert_eq!(session.run, config);

    let before = session.catalog();
    assert!(session.preview_inputs().pieces.is_empty(), "no piece yet");
    // The open room is marked as a piece and not saved: the preview still sees it.
    session
        .draft
        .set_piece(Some(PieceInfo::new(PieceRole::Middle)))
        .unwrap();
    let marked = session.preview_inputs();
    assert_eq!(marked.pieces.len(), 1);
    assert_eq!(marked.pieces[0].0, session.id);
    assert!(
        session
            .rooms
            .iter()
            .all(|(_, layout)| layout.piece.is_none()),
        "the saved rooms are as they were"
    );

    // Building from it reads, and changes, nothing in the session.
    let mut preview = DungeonPreview::default();
    preview.refresh(marked);
    preview.roll_next();
    preview.refresh(session.preview_inputs());
    assert!(matches!(preview.outcome(), Some(Outcome::Failed(_))));
    assert!(!session.dirty, "the preview does not mark the room unsaved");
    assert_ne!(
        session.catalog(),
        before,
        "only the draft's own mark differs"
    );
}
