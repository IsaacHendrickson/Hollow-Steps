//! The path replay helper (`tests/support/path.rs`), on rooms built in code: a floor walk, a gap
//! the real jump crosses, a gap it cannot, a gap only a dash crosses, a hazard, a locked exit,
//! and the failure text. Every number that depends on the player is taken from the real reach
//! code at the shipped tuning, so a retuned player does not break these.
mod support;

use hollow_steps::ability::Ability;
use hollow_steps::editor::{ExitPlacement, RoomLayout};
use hollow_steps::hazard::HazardPlacement;
use hollow_steps::player::PlayerConfig;
use hollow_steps::reach::Reach;
use hollow_steps::room::{RoomEdge, RoomId};
use hollow_steps::timestep::FIXED_STEP_SECONDS;
use support::path::*;

/// Where the platform the player starts on ends.
const EDGE: f32 = 400.0;
/// The top of the platforms; a standing player's center is half its height above.
const TOP: f32 = 600.0;
const START_X: f32 = 100.0;
const STAND_Y: f32 = TOP - 24.0;

fn exit(edge: RoomEdge, offset: f32, length: f32) -> ExitPlacement {
    ExitPlacement {
        edge,
        offset,
        length,
        target_room: RoomId::First,
        spawn: (START_X, STAND_Y),
        door: None,
        locked_by: None,
    }
}

/// A room 1600 px wide and 1000 tall (a pit 400 px deep, more than a jump rises) with an exit on
/// the right wall, standing height, and these solids.
fn room_with(terrain: &[(f32, f32, f32, f32)]) -> RoomLayout {
    let mut room: RoomLayout = ron::from_str(&format!(
        "(bounds: (0.0, 0.0, 1600.0, 1000.0), floor_y: 1000.0, spawn: ({START_X}, {STAND_Y}), \
         items: [], exits: [])"
    ))
    .unwrap();
    room.terrain = terrain.to_vec();
    room.exits = vec![exit(RoomEdge::Right, 500.0, 96.0)];
    room.validate_geometry().expect("the fixture room is valid");
    room
}

fn flat_room() -> RoomLayout {
    room_with(&[(0.0, TOP, 1600.0, 400.0)])
}

/// Two platforms with a gap of `gap` px between them.
fn gap_room(gap: f32) -> RoomLayout {
    room_with(&[
        (0.0, TOP, EDGE, 400.0),
        (EDGE + gap, TOP, 1600.0 - EDGE - gap, 400.0),
    ])
}

/// Standing anywhere on the far platform, well past the gap.
fn far_side(gap: f32) -> Target {
    Target::area(EDGE + gap + 100.0, STAND_Y - 24.0, 600.0, 48.0)
}

fn ticks_for(distance: f32) -> usize {
    let config = PlayerConfig::default();
    (distance / (config.move_speed * FIXED_STEP_SECONDS)).floor() as usize
}

/// Run to the edge, jump (the key held through the whole flight), run on and settle.
fn crossing() -> PathScript {
    PathScript::new([
        run_right(ticks_for(EDGE - START_X)),
        jump(170),
        run_right(200),
        wait(40),
    ])
}

/// The same, with a dash near the top of the jump, where it keeps the most air time.
fn dash_crossing() -> PathScript {
    PathScript::new([
        run_right(ticks_for(EDGE - START_X)),
        jump(170),
        run_right(70),
        dash(),
        run_right(200),
        wait(40),
    ])
    .with_ability(Ability::Dash)
}

fn reach() -> Reach {
    Reach::of(&PlayerConfig::default())
}

#[test]
fn a_walk_along_a_floor_gets_where_the_distance_says() {
    let room = flat_room();
    // 60 ticks at the move speed: about 110 px from the spawn.
    let walk = PathScript::new([run_right(60)]);
    assert_reaches(
        &room,
        Arrival::Spawn,
        &walk,
        Target::area(190.0, STAND_Y - 24.0, 40.0, 48.0),
    );
    assert_blocked(
        &room,
        Arrival::Spawn,
        &PathScript::new([run_right(20)]),
        Target::area(190.0, 552.0, 40.0, 48.0),
    );
    // All the way to the exit on the right wall.
    let tick = check_reaches(
        &room,
        Arrival::Spawn,
        &PathScript::new([run_right(900)]),
        Target::Exit(0),
    )
    .expect("a floor walk reaches the exit");
    assert!((780..=800).contains(&tick), "{tick}");
    // Walking the other way never does.
    assert_blocked(
        &room,
        Arrival::Spawn,
        &PathScript::new([run_left(900)]),
        Target::Exit(0),
    );
}

#[test]
fn a_replay_starts_from_an_exits_arrival_point_too() {
    let room = flat_room();
    let arrival = Arrival::of(&ExitPlacement {
        spawn: (1200.0, STAND_Y),
        ..exit(RoomEdge::Left, 0.0, 10.0)
    });
    assert_eq!(
        arrival,
        Arrival::At(macroquad::math::Vec2::new(1200.0, STAND_Y))
    );
    let tick = check_reaches(
        &room,
        arrival,
        &PathScript::new([run_right(900)]),
        Target::Exit(0),
    )
    .unwrap();
    assert!(tick < 250, "from x 1200 the exit is 350 px away: {tick}");
    let played = replay(&room, arrival, &PathScript::new([wait(5)]));
    assert_eq!(played.samples[0].position.x, 1200.0);
    assert!(
        played.samples[0].grounded,
        "the arrival settles onto the floor"
    );
}

#[test]
fn a_jump_over_a_gap_the_real_jump_crosses_reaches_the_far_side() {
    let gap = reach().jump_distance * 0.7;
    let room = gap_room(gap);
    let tick = check_reaches(&room, Arrival::Spawn, &crossing(), far_side(gap))
        .unwrap_or_else(|m| panic!("{m}"));
    assert!(tick > 0);
    // The same script only walking would fall into the pit.
    let walk = PathScript::new([run_right(ticks_for(EDGE - START_X) + 200), wait(40)]);
    assert_blocked(&room, Arrival::Spawn, &walk, far_side(gap));
}

#[test]
fn a_gap_wider_than_the_reach_is_not_crossed_by_the_same_script() {
    let gap = reach().jump_distance * 1.3;
    let room = gap_room(gap);
    assert_blocked(&room, Arrival::Spawn, &crossing(), far_side(gap));
    let message = check_reaches(&room, Arrival::Spawn, &crossing(), far_side(gap)).unwrap_err();
    assert!(message.contains("does not reach"), "{message}");
}

#[test]
fn a_dash_gap_needs_the_dash() {
    let reach = reach();
    assert!(
        reach.dash_jump_distance > reach.jump_distance + 50.0,
        "a dash adds distance"
    );
    let gap = (reach.jump_distance + reach.dash_jump_distance) / 2.0;
    let room = gap_room(gap);
    assert_reaches(&room, Arrival::Spawn, &dash_crossing(), far_side(gap));
    assert_blocked_without(
        &room,
        Arrival::Spawn,
        &dash_crossing(),
        far_side(gap),
        Ability::Dash,
    );
    // A gap the plain jump crosses is not a dash gap: the proof says so.
    let easy = gap_room(reach.jump_distance * 0.7);
    let message = check_blocked_without(
        &easy,
        Arrival::Spawn,
        &dash_crossing(),
        far_side(reach.jump_distance * 0.7),
        Ability::Dash,
    )
    .unwrap_err();
    assert!(message.contains("Dash is not needed"), "{message}");
}

#[test]
fn a_script_that_never_gets_there_proves_nothing_about_a_gate() {
    let gap = (reach().jump_distance + reach().dash_jump_distance) / 2.0;
    let room = gap_room(gap);
    let idle = PathScript::new([wait(100)]).with_ability(Ability::Dash);
    let message = check_blocked_without(&room, Arrival::Spawn, &idle, far_side(gap), Ability::Dash)
        .unwrap_err();
    assert!(message.contains("proves nothing about Dash"), "{message}");
}

#[test]
fn a_locked_exit_needs_its_key() {
    let mut room = flat_room();
    room.exits[0].locked_by = Some(2);
    let walk = PathScript::new([run_right(900)]).with_key(2);
    assert_reaches(&room, Arrival::Spawn, &walk, Target::Exit(0));
    assert_blocked_without(&room, Arrival::Spawn, &walk, Target::Exit(0), Lack::Key(2));
    // Another key does not open it.
    let wrong = PathScript::new([run_right(900)]).with_key(3);
    assert_blocked(&room, Arrival::Spawn, &wrong, Target::Exit(0));
}

#[test]
fn a_hazard_kills_the_replay_and_the_report_says_which() {
    let mut room = flat_room();
    let mut spikes = HazardPlacement::standard(0.0, 0.0);
    spikes.rect = (500.0, TOP - 16.0, 64.0, 16.0);
    room.hazards = vec![spikes];
    room.validate_geometry().unwrap();
    let walk = PathScript::new([run_right(900)]);
    let played = replay(&room, Arrival::Spawn, &walk);
    let last = played.last();
    assert_eq!(last.hazard, Some(0));
    assert!(
        last.tick < 250,
        "the spikes are about 200 px away: {}",
        last.tick
    );
    assert!(
        played.samples.len() < walk.length(),
        "the replay stopped at the hit"
    );
    assert_eq!(played.planned, 900);
    assert_eq!(
        played.verdict(&room, Target::Exit(0), &walk.holds),
        Verdict::Died {
            tick: last.tick,
            hazard: 0
        }
    );
    let message = check_reaches(&room, Arrival::Spawn, &walk, Target::Exit(0)).unwrap_err();
    assert!(
        message.contains("hit hazard #0") && message.contains("replay stopped"),
        "{message}"
    );
    // Jumping them is fine.
    let over = PathScript::new([
        run_right(ticks_for(440.0 - START_X)),
        jump(120),
        run_right(900),
    ]);
    assert_reaches(&room, Arrival::Spawn, &over, Target::Exit(0));
}

#[test]
fn a_failure_prints_a_compact_trace_and_the_last_tick() {
    let gap = reach().jump_distance * 1.3;
    let room = gap_room(gap);
    let message = check_reaches(&room, Arrival::Spawn, &crossing(), far_side(gap)).unwrap_err();
    let rows: Vec<&str> = message
        .lines()
        .filter(|line| {
            line.trim_start()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
        })
        .collect();
    assert!(
        message.contains("tick") && message.contains("nearest terrain edge"),
        "{message}"
    );
    assert!(
        message.contains("from the spawn, holding nothing"),
        "{message}"
    );
    let tick = |row: &&str| {
        row.split_whitespace()
            .next()
            .unwrap()
            .parse::<usize>()
            .unwrap()
    };
    let ticks: Vec<usize> = rows.iter().map(tick).collect();
    assert_eq!(&ticks[..3], &[0, 10, 20], "a row every tenth tick");
    let played = replay(&room, Arrival::Spawn, &crossing());
    assert_eq!(
        *ticks.last().unwrap(),
        played.last().tick,
        "and the last tick"
    );
    assert!(ticks.windows(2).all(|pair| pair[0] < pair[1]));
    assert!(
        rows.iter().any(|row| row.contains("terrain #0")),
        "the nearest edge is named: {message}"
    );
    assert!(
        rows.iter().any(|row| row.contains("grounded"))
            && rows.iter().any(|row| row.contains("air"))
    );
    // It ended in the pit, on the floor.
    assert!(message.contains("ended at"), "{message}");
}

#[test]
fn a_replay_is_deterministic() {
    let gap = reach().jump_distance * 0.7;
    let room = gap_room(gap);
    let first = replay(&room, Arrival::Spawn, &crossing());
    let second = replay(&room, Arrival::Spawn, &crossing());
    assert_eq!(first.samples, second.samples);
    assert_eq!(
        first.samples.len(),
        crossing().length() + 1,
        "the arrival, then every tick"
    );
    let sparse: Vec<usize> = first.every(50).map(|sample| sample.tick).collect();
    assert_eq!(sparse[..3], [0, 50, 100]);
}

#[test]
fn a_script_reads_as_its_steps() {
    let script = PathScript::new([run_right(2), jump(3), wait(1), dash(), run_left(1)]);
    let ticks = script.ticks();
    assert_eq!(script.length(), 6);
    let axes: Vec<f32> = ticks.iter().map(|tick| tick.axis).collect();
    assert_eq!(
        axes,
        [1.0, 1.0, 1.0, 0.0, 0.0, -1.0],
        "a jump keeps the heading, a wait clears it"
    );
    let pressed: Vec<bool> = ticks.iter().map(|tick| tick.jump_pressed).collect();
    assert_eq!(
        pressed,
        [false, false, true, false, false, false],
        "the press is one tick"
    );
    let held: Vec<bool> = ticks.iter().map(|tick| tick.jump_held).collect();
    assert_eq!(
        held,
        [false, false, true, true, true, false],
        "held for 3 ticks, the press included"
    );
    assert!(ticks[4].dash_pressed && ticks.iter().filter(|tick| tick.dash_pressed).count() == 1);
    let without = PathScript::new([])
        .with_ability(Ability::Dash)
        .with_key(1)
        .without(Ability::Dash.into());
    assert!(without.holds.abilities.is_empty() && without.holds.keys == [1]);
}
