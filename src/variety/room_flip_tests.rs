//! Which flippable rooms a seed mirrors, with no game: the pure choice (`RoomFlips`), what a run
//! decides from F4's choice and the rules, and the `room_flip_percent` field of `variety.ron`.
//! The draws are pinned against a separate implementation of the stream mixing (FNV-1a keys,
//! SplitMix64 steps), so a change to the keys or the mixing fails here, not in a player's run.
use super::*;
use crate::room::RoomId;

/// Every room id the fixtures use, in `RoomId::index` order 0 to 5.
const ROOMS: [RoomId; 6] = [
    RoomId::First,
    RoomId::Second,
    RoomId::Boss,
    RoomId::Custom(0),
    RoomId::Custom(1),
    RoomId::Custom(2),
];

fn chance(seed: u64, percent: f32) -> RoomFlips {
    RoomFlips::Chance { seed, percent }
}

fn flipped(flips: RoomFlips) -> Vec<bool> {
    ROOMS.iter().map(|room| flips.flips(*room)).collect()
}

#[test]
fn off_flips_nothing_and_all_flips_everything() {
    assert!(flipped(RoomFlips::Off).iter().all(|flip| !flip));
    assert!(flipped(RoomFlips::All).iter().all(|flip| *flip));
    assert_eq!(RoomFlips::default(), RoomFlips::Off);
}

#[test]
fn these_seeds_flip_exactly_these_rooms_at_fifty_percent() {
    // Draws, in percent, worked out apart from this code (rooms First to Custom(2)):
    //   seed 1: 60.22 93.36 25.23 82.19  2.70 21.64
    //   seed 3: 92.99 24.83 48.88  1.74 54.53 95.02
    //   seed 0xABC: 13.04 83.12 26.91 23.27 48.59 71.96
    assert_eq!(
        flipped(chance(1, 50.0)),
        [false, false, true, false, true, true]
    );
    assert_eq!(
        flipped(chance(3, 50.0)),
        [false, true, true, true, false, false]
    );
    assert_eq!(
        flipped(chance(0xABC, 50.0)),
        [true, false, true, true, true, false]
    );
}

#[test]
fn a_room_flips_when_its_draw_is_under_the_percent_and_not_when_it_is_over() {
    // Seed 1, Boss draws 25.23 percent.
    assert!(chance(1, 25.5).flips(RoomId::Boss));
    assert!(!chance(1, 25.0).flips(RoomId::Boss));
}

#[test]
fn zero_percent_never_flips_and_one_hundred_always_does() {
    for seed in (0..200).chain([u64::MAX]) {
        assert!(
            flipped(chance(seed, 0.0)).iter().all(|flip| !flip),
            "{seed}"
        );
        assert!(
            flipped(chance(seed, 100.0)).iter().all(|flip| *flip),
            "{seed}"
        );
    }
}

#[test]
fn the_same_seed_gives_the_same_answer_and_each_room_draws_apart() {
    for seed in [0, 1, 7, u64::MAX] {
        assert_eq!(flipped(chance(seed, 50.0)), flipped(chance(seed, 50.0)));
    }
    // Seed 7: 53.9 34.5 16.8 86.8 84.9 73.7 (First to Custom(2)), then 49.8 36.8 for 6 and 7.
    let rooms: Vec<bool> = (0..8)
        .map(|index| chance(7, 50.0).flips(room_at(index)))
        .collect();
    assert_eq!(rooms, [false, true, true, false, false, false, true, true]);
}

fn room_at(index: usize) -> RoomId {
    match index {
        0 => RoomId::First,
        1 => RoomId::Second,
        2 => RoomId::Boss,
        n => RoomId::Custom(n - 3),
    }
}

#[test]
fn over_200_seeds_a_room_is_flipped_in_a_middle_share_of_them() {
    for room in ROOMS {
        let flips = (1..=200)
            .filter(|seed| chance(*seed, 50.0).flips(room))
            .count();
        assert!(
            (80..=120).contains(&flips),
            "{room:?} flipped in {flips} of 200 seeds"
        );
    }
}

#[test]
fn the_room_draw_does_not_share_numbers_with_the_dungeon_flip_or_the_enemy_picks() {
    // Seed 1: the dungeon's own draw is 54.50 percent and room First's is 60.22.
    let dungeon = f64::from(crate::seed::stream(1, "flip", 0).unit()) * 100.0;
    assert!((dungeon - 54.50).abs() < 0.01, "{dungeon}");
    assert!(!chance(1, 55.0).flips(RoomId::First));
}

#[test]
fn a_run_decides_by_f4s_choice_and_the_rules() {
    let rules = VarietyRules {
        room_flip_percent: 30.0,
        ..VarietyRules::default()
    };
    assert_eq!(
        decide_room_flips(FlipChoice::Never, &rules, 5),
        RoomFlips::Off
    );
    assert_eq!(
        decide_room_flips(FlipChoice::Always, &rules, 5),
        RoomFlips::All
    );
    assert_eq!(
        decide_room_flips(FlipChoice::Auto, &rules, 5),
        chance(5, 30.0)
    );
}

#[test]
fn the_flag_is_written_only_when_it_is_set() {
    assert!(is_false(&false));
    assert!(!is_false(&true));
}

// ---- room_flip_percent in the file ----

#[test]
fn a_file_without_room_flip_percent_gets_fifty() {
    let rules = VarietyRules::from_ron("(flip_percent: 10.0)").unwrap();
    assert_eq!(rules.flip_percent, 10.0);
    assert_eq!(rules.room_flip_percent, 50.0);
    assert_eq!(VarietyRules::default().room_flip_percent, 50.0);
}

#[test]
fn a_file_can_say_room_flip_percent() {
    let rules = VarietyRules::from_ron("(flip_percent: 0.0, room_flip_percent: 12.5)").unwrap();
    assert_eq!(rules.room_flip_percent, 12.5);
    for edge in ["0.0", "100.0"] {
        let text = format!("(flip_percent: 0.0, room_flip_percent: {edge})");
        assert!(VarietyRules::from_ron(&text).is_ok(), "{edge}");
    }
}

#[test]
fn room_flip_percent_outside_0_to_100_is_refused_and_named() {
    for bad in ["100.5", "-1.0", "inf", "NaN"] {
        let error =
            VarietyRules::from_ron(&format!("(flip_percent: 0.0, room_flip_percent: {bad})"))
                .unwrap_err();
        assert!(error.contains("room_flip_percent"), "{bad}: {error}");
    }
    // The other field is still checked, and still named.
    let error = VarietyRules::from_ron("(flip_percent: 101.0)").unwrap_err();
    assert!(error.contains("flip_percent"), "{error}");
}

#[test]
fn an_unknown_field_is_still_refused() {
    assert!(VarietyRules::from_ron("(flip_percent: 0.0, room_percent: 5.0)").is_err());
}
