//! The dungeon checker with the Double Jump as the ability a gate needs: the same rules as the
//! Dash, told apart from it, and the two together.
use super::*;

/// A gate on the way in from `from` (its exit at `offset` on the right edge) to `to`.
fn gate_needing(room: RoomId, offset: f32, to: &ExitPlacement, needs: &[Ability]) -> Gate {
    Gate {
        needs: needs.to_vec(),
        ..gate_from(room, offset, to)
    }
}

/// First (with a key and `pickups`), Second (a door locked by that key to the boss, gated by
/// `needs` on the way in from First, and an exit back) and the boss: `valid()` with other gates
/// and other pickups.
fn dungeon_gated_by(needs: &[Ability], pickups: &[Item]) -> Vec<(RoomId, RoomLayout)> {
    let to_boss = locked(200.0, BOSS, 1);
    let mut second = room(SECOND, vec![exit(100.0, FIRST), to_boss.clone()], &[]);
    second.1.gates = vec![gate_needing(FIRST, 100.0, &to_boss, needs)];
    let mut items = vec![Item::Key(1)];
    items.extend_from_slice(pickups);
    dungeon(vec![
        room(FIRST, vec![exit(100.0, SECOND), exit(300.0, HUB)], &items),
        second,
        room(BOSS, vec![exit(100.0, SECOND)], &[]),
    ])
}

fn unobtainable(needs: &[Ability]) -> Problem {
    Problem::AbilityUnobtainable {
        room: "Second".into(),
        exit: "right edge exit at 200".into(),
        needs: needs.to_vec(),
    }
}

#[test]
fn each_ability_has_its_own_bit_and_the_bits_read_back_in_the_order_of_the_abilities() {
    let bits: Vec<u8> = Ability::ALL.iter().map(|a| ability_bit(*a)).collect();
    assert!(bits.iter().all(|bit| *bit != 0), "{bits:?}");
    for (at, bit) in bits.iter().enumerate() {
        assert!(
            bits[at + 1..].iter().all(|other| other & bit == 0),
            "{bits:?}"
        );
    }
    let all = bits.iter().fold(0, |mask, bit| mask | bit);
    assert_eq!(abilities_of(all), Ability::ALL);
    assert_eq!(
        abilities_of(ability_bit(Ability::DoubleJump)),
        [Ability::DoubleJump]
    );
}

#[test]
fn a_gate_that_needs_the_double_jump_passes_when_the_pickup_lies_before_it() {
    let rooms = dungeon_gated_by(&[Ability::DoubleJump], &[Item::DoubleJump]);
    assert_eq!(problems(&rooms), []);
    assert_eq!(check(&rooms, &plain_config()), [], "without a hub too");
}

#[test]
fn taking_the_pickup_away_makes_the_checker_report_the_gate_by_the_abilitys_name() {
    let mut rooms = dungeon_gated_by(&[Ability::DoubleJump], &[Item::DoubleJump]);
    assert_eq!(problems(&rooms), []);

    rooms[1]
        .1
        .items
        .retain(|placed| placed.item != Item::DoubleJump);

    let found = problems(&rooms);
    assert!(
        found.contains(&unobtainable(&[Ability::DoubleJump])),
        "{found:?}"
    );
    assert!(
        texts(&found).contains(
            &"Second: the right edge exit at 200 needs Double Jump, \
              which cannot be picked up before it"
                .to_owned()
        ),
        "{:?}",
        texts(&found)
    );
}

#[test]
fn a_double_jump_that_lies_behind_its_own_gate_is_reported() {
    // First's way into Second needs the Double Jump, which only lies in Second.
    let to_second = exit(100.0, SECOND);
    let mut first = room(FIRST, vec![to_second.clone(), exit(300.0, BOSS)], &[]);
    first.1.gates = vec![Gate {
        from: Entrance::Arrival {
            room: HUB,
            exit: ExitRef::Edge {
                edge: RoomEdge::Right,
                offset: 100.0,
            },
        },
        exit: ExitRef::of(&to_second),
        needs: vec![Ability::DoubleJump],
    }];
    let rooms = dungeon(vec![
        first,
        room(SECOND, vec![exit(100.0, FIRST)], &[Item::DoubleJump]),
        room(BOSS, vec![exit(100.0, HUB)], &[]),
    ]);
    assert_eq!(
        problems(&rooms),
        [
            Problem::Unreachable {
                room: "Second".into()
            },
            Problem::AbilityUnobtainable {
                room: "First".into(),
                exit: "right edge exit at 100".into(),
                needs: vec![Ability::DoubleJump],
            },
        ]
    );
}

#[test]
fn the_dash_does_not_open_a_double_jump_gate_and_the_double_jump_does_not_open_a_dash_gate() {
    let rooms = dungeon_gated_by(&[Ability::DoubleJump], &[Item::Dash]);
    assert!(problems(&rooms).contains(&unobtainable(&[Ability::DoubleJump])));

    let rooms = dungeon_gated_by(&[Ability::Dash], &[Item::DoubleJump]);
    assert!(problems(&rooms).contains(&unobtainable(&[Ability::Dash])));
}

#[test]
fn a_gate_that_needs_both_reports_only_what_is_missing_and_passes_with_both() {
    let both = [Ability::Dash, Ability::DoubleJump];
    let none = problems(&dungeon_gated_by(&both, &[]));
    assert!(none.contains(&unobtainable(&both)), "{none:?}");
    assert!(
        texts(&none)
            .iter()
            .any(|text| text.contains("needs Dash and Double Jump, which")),
        "{:?}",
        texts(&none)
    );

    let dash_only = problems(&dungeon_gated_by(&both, &[Item::Dash]));
    assert!(
        dash_only.contains(&unobtainable(&[Ability::DoubleJump])),
        "{dash_only:?}"
    );
    assert!(!dash_only.contains(&unobtainable(&both)), "{dash_only:?}");

    let double_only = problems(&dungeon_gated_by(&both, &[Item::DoubleJump]));
    assert!(
        double_only.contains(&unobtainable(&[Ability::Dash])),
        "{double_only:?}"
    );

    assert_eq!(
        problems(&dungeon_gated_by(&both, &[Item::Dash, Item::DoubleJump])),
        []
    );
}

#[test]
fn a_gate_that_another_way_in_lets_the_player_use_is_not_reported_for_the_double_jump_either() {
    // The exit to the boss needs the Double Jump from the spawn, but coming back from Second
    // it is free. No Double Jump lies anywhere.
    let to_boss = exit(300.0, BOSS);
    let mut first = room(FIRST, vec![exit(100.0, SECOND), to_boss.clone()], &[]);
    first.1.gates = vec![Gate {
        from: Entrance::Spawn,
        exit: ExitRef::of(&to_boss),
        needs: vec![Ability::DoubleJump],
    }];
    let rooms = dungeon(vec![
        first,
        room(SECOND, vec![exit(100.0, FIRST), exit(300.0, BOSS)], &[]),
        room(BOSS, vec![exit(100.0, SECOND)], &[]),
    ]);
    assert_eq!(problems(&rooms), []);
}

/// First, with a free way to the boss, a free way into Second and one into Spare; Second's only
/// exit needs `needs` from the arrival from First. `first` and `spare` hold what lies in them.
fn pit_needing(needs: &[Ability], first: &[Item], spare: &[Item]) -> Vec<(RoomId, RoomLayout)> {
    let way_out = exit(100.0, FIRST);
    let mut second = room(SECOND, vec![way_out.clone()], &[]);
    second.1.gates = vec![gate_needing(FIRST, 100.0, &way_out, needs)];
    dungeon(vec![
        room(
            FIRST,
            vec![exit(100.0, SECOND), exit(300.0, SPARE), exit(500.0, BOSS)],
            first,
        ),
        second,
        room(BOSS, vec![exit(100.0, FIRST)], &[]),
        room(SPARE, vec![exit(100.0, FIRST)], spare),
    ])
}

fn trap(holding: &[Ability]) -> Problem {
    Problem::Trap {
        room: "Second".into(),
        entrance: "the arrival from First (right edge exit at 100)".into(),
        holding: holding.to_vec(),
    }
}

#[test]
fn arriving_in_a_pit_before_the_double_jump_is_a_trap() {
    let rooms = pit_needing(&[Ability::DoubleJump], &[], &[Item::DoubleJump]);
    assert_eq!(problems(&rooms), [trap(&[])]);
    assert_eq!(
        problems(&rooms)[0].text(),
        "Second: arriving at the arrival from First (right edge exit at 100) with no abilities, \
         no exit can be taken"
    );
}

#[test]
fn a_trap_names_the_ability_the_player_holds_in_it_and_it_is_the_one_that_does_not_help() {
    // The Dash lies in First, so every arrival in Second holds it; the pit needs the Double Jump.
    let dash_in_the_pit = pit_needing(&[Ability::DoubleJump], &[Item::Dash], &[Item::DoubleJump]);
    assert_eq!(problems(&dash_in_the_pit), [trap(&[Ability::Dash])]);
    assert!(problems(&dash_in_the_pit)[0]
        .text()
        .contains("with Dash, no exit can be taken"));

    // And the other way round: the Double Jump lies in First, the pit needs the Dash.
    let double_in_the_pit = pit_needing(&[Ability::Dash], &[Item::DoubleJump], &[Item::Dash]);
    assert_eq!(problems(&double_in_the_pit), [trap(&[Ability::DoubleJump])]);
    assert!(problems(&double_in_the_pit)[0]
        .text()
        .contains("with Double Jump, no exit can be taken"));
}

#[test]
fn a_pit_the_double_jump_lies_in_is_no_trap() {
    let rooms = pit_needing(&[Ability::DoubleJump], &[], &[]);
    let mut with_it = rooms.clone();
    with_it[2].1.items.push(ItemPlacement {
        item: Item::DoubleJump,
        position: (100.0, 440.0),
    });
    assert_eq!(
        problems(&with_it),
        [],
        "picked up in the pit before the climb"
    );
    assert!(
        !problems(&rooms).is_empty(),
        "and without it there is something to say"
    );
}
