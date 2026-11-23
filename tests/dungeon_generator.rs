//! The dungeon generator (`docs/dungeon-gen.md`) on a pool of pieces built in code: the same
//! seed gives the same plan, every seed builds a dungeon that follows the rules and passes the
//! checker, seeds differ, and a pool or rules that cannot work say why.
mod support;

use hollow_steps::dungeon_gen::{generate, DungeonPlan, GenError};
use hollow_steps::dungeon_map::{MapLayout, MapRoomInput};
use hollow_steps::editor::RoomLayout;
use hollow_steps::piece::PieceRole;
use hollow_steps::room::RoomId;
use hollow_steps::variety::generator::CountRange;
use hollow_steps::variety::GeneratorRules;
use std::collections::HashSet;
use support::pieces::{
    self, ABILITY_ENDS, BOSS_A, BOSS_B, CORRIDORS, DASH_CROSSINGS, FORKS, KEY_ENDS, START_A,
    START_B,
};
use support::plan_rules::assert_plan_valid;

type Pool = Vec<(RoomId, RoomLayout)>;

fn plan_of(pool: &Pool, rules: &GeneratorRules, seed: u64) -> Result<DungeonPlan, GenError> {
    generate(pool, &pieces::hub(), &pieces::config(), rules, seed)
}

fn plan(seed: u64) -> DungeonPlan {
    plan_of(&pieces::pool(), &pieces::rules(), seed)
        .unwrap_or_else(|error| panic!("seed {seed}: {error}"))
}

fn without(pool: Pool, ids: &[RoomId]) -> Pool {
    pool.into_iter()
        .filter(|(id, _)| !ids.contains(id))
        .collect()
}

fn order(plan: &DungeonPlan) -> Vec<RoomId> {
    plan.rooms.iter().map(|room| room.piece).collect()
}

#[test]
fn the_fixture_pool_is_what_the_tests_say_it_is() {
    let pool = pieces::pool();
    let count = |role| {
        let marked = pool
            .iter()
            .filter(|(_, layout)| layout.piece.is_some_and(|p| p.role == role));
        marked.count()
    };
    assert_eq!(
        (
            count(PieceRole::Start),
            count(PieceRole::Middle),
            count(PieceRole::Boss)
        ),
        (2, 15, 2)
    );
    let ids: HashSet<_> = pool.iter().map(|(id, _)| format!("{id:?}")).collect();
    assert_eq!(ids.len(), pool.len(), "every piece has its own id");
}

#[test]
fn the_same_seed_gives_the_same_plan_whatever_order_the_pieces_come_in() {
    let pool = pieces::pool();
    let mut reversed = pool.clone();
    reversed.reverse();
    let mut rotated = pool.clone();
    rotated.rotate_left(7);
    for seed in [0, 1, 7, 42, 1999] {
        let first = plan_of(&pool, &pieces::rules(), seed).unwrap();
        assert_eq!(
            first,
            plan_of(&pool, &pieces::rules(), seed).unwrap(),
            "seed {seed}"
        );
        assert_eq!(
            first,
            plan_of(&reversed, &pieces::rules(), seed).unwrap(),
            "seed {seed}"
        );
        assert_eq!(
            first,
            plan_of(&rotated, &pieces::rules(), seed).unwrap(),
            "seed {seed}"
        );
    }
}

#[test]
fn different_seeds_give_different_dungeons() {
    let plans: Vec<_> = (0..20).map(plan).collect();
    let distinct: HashSet<_> = plans
        .iter()
        .map(|plan| format!("{:?}", order(plan)))
        .collect();
    assert!(
        distinct.len() > 10,
        "{} different orders in 20 seeds",
        distinct.len()
    );
}

#[test]
fn every_seed_builds_a_valid_dungeon_within_the_attempts() {
    let (pool, hub, config, rules) = (
        pieces::pool(),
        pieces::hub(),
        pieces::config(),
        pieces::rules(),
    );
    let mut attempts = 0u64;
    let mut worst = 0;
    for seed in 0..2000 {
        let plan = generate(&pool, &hub, &config, &rules, seed)
            .unwrap_or_else(|error| panic!("seed {seed}: {error}"));
        assert!(
            plan.attempts >= 1 && plan.attempts <= rules.attempts as u32,
            "seed {seed}"
        );
        assert_eq!(plan.seed, seed);
        assert_plan_valid(&plan, &pool, &hub, &config, &rules);
        attempts += u64::from(plan.attempts);
        worst = worst.max(plan.attempts);
    }
    let mean = attempts as f64 / 2000.0;
    eprintln!("mean attempts over 2000 seeds: {mean:.3}, worst {worst}");
    assert!(mean < 3.0, "mean attempts {mean}");
}

#[test]
fn seeds_vary_the_rooms_the_order_and_the_ends() {
    let plans: Vec<_> = (0..200).map(plan).collect();
    let orders: HashSet<_> = plans.iter().map(order).collect();
    eprintln!("different room orders over 200 seeds: {}", orders.len());
    assert!(
        orders.len() >= 150,
        "{} different orders in 200 seeds",
        orders.len()
    );

    let appears = |id: RoomId| {
        plans
            .iter()
            .filter(|p| p.rooms.iter().any(|r| r.piece == id))
            .count()
    };
    let middles = CORRIDORS
        .iter()
        .chain(&FORKS)
        .chain(&KEY_ENDS)
        .chain(&ABILITY_ENDS)
        .chain(&DASH_CROSSINGS);
    for id in middles {
        let count = appears(*id);
        assert!(count > 0, "{id:?} never appears");
        assert!(count < plans.len(), "{id:?} is in every plan");
    }
    for id in [START_A, START_B, BOSS_A, BOSS_B] {
        assert!(appears(id) > 0, "{id:?} never appears");
    }
}

#[test]
fn a_dash_gated_piece_always_comes_with_a_dash_placed_before_it() {
    let (pool, hub, config, rules) = (
        pieces::pool(),
        pieces::hub(),
        pieces::config(),
        pieces::rules(),
    );
    let mut seen = 0;
    for seed in 0..200 {
        let plan = plan(seed);
        let gated = plan.rooms.iter().any(|room| !room.layout.gates.is_empty());
        if gated {
            seen += 1;
            let dash = plan.rooms.iter().any(|room| {
                room.layout.items.iter().any(|placed| {
                    placed.item.grants() == Some(hollow_steps::ability::Ability::Dash)
                })
            });
            assert!(dash, "seed {seed}: a Dash gate and no Dash");
            assert_plan_valid(&plan, &pool, &hub, &config, &rules);
        }
    }
    assert!(seen >= 10, "only {seen} of 200 plans have a gate");
}

#[test]
fn locks_and_keys_appear_and_stay_within_the_rules() {
    let mut with_locks = 0;
    for seed in 0..200 {
        let plan = plan(seed);
        let locked = plan
            .rooms
            .iter()
            .flat_map(|room| &room.layout.exits)
            .filter(|exit| exit.locked_by.is_some())
            .count();
        assert!(locked <= 3, "seed {seed}: {locked} locks");
        with_locks += usize::from(locked > 0);
    }
    assert!(
        with_locks >= 20,
        "only {with_locks} of 200 plans have a lock"
    );
}

#[test]
fn every_plan_puts_each_room_in_its_own_map_cell() {
    for seed in 0..300 {
        let plan = plan(seed);
        let inputs: Vec<_> = plan
            .catalog()
            .iter()
            .map(|(id, layout)| MapRoomInput::from_layout(*id, layout))
            .collect();
        let map = MapLayout::build(&inputs);
        assert!(map.conflicts.is_empty(), "seed {seed}: {:?}", map.conflicts);
        let cells: HashSet<_> = map
            .rooms
            .iter()
            .map(|room| format!("{:?}", room.cell))
            .collect();
        assert_eq!(
            cells.len(),
            inputs.len(),
            "seed {seed}: two rooms in one cell"
        );
    }
}

#[test]
fn mirrored_pieces_pair_on_right_edges_and_unpaired_exits_are_gone() {
    let (pool, hub, config) = (pieces::pool(), pieces::hub(), pieces::config());
    let all = GeneratorRules {
        mirror_percent: 100.0,
        ..pieces::rules()
    };
    let none = GeneratorRules {
        mirror_percent: 0.0,
        ..pieces::rules()
    };
    for seed in 0..100 {
        let mirrored = plan_of(&pool, &all, seed).unwrap();
        assert!(
            mirrored.rooms.iter().all(|room| room.mirrored),
            "seed {seed}"
        );
        assert_plan_valid(&mirrored, &pool, &hub, &config, &all);
        let plain = plan_of(&pool, &none, seed).unwrap();
        assert!(plain.rooms.iter().all(|room| !room.mirrored), "seed {seed}");
        assert_plan_valid(&plain, &pool, &hub, &config, &none);

        for plan in [&mirrored, &plain] {
            let pieces_used: HashSet<_> = plan
                .rooms
                .iter()
                .map(|room| format!("{:?}", room.piece))
                .collect();
            assert_eq!(
                pieces_used.len(),
                plan.rooms.len(),
                "seed {seed}: a piece twice"
            );
            for room in &plan.rooms {
                let source = &pool.iter().find(|(id, _)| *id == room.piece).unwrap().1;
                // No room keeps more exits than its piece had, and each one it keeps is joined.
                assert!(room.layout.exits.len() <= source.exits.len(), "seed {seed}");
                for exit in &room.layout.exits {
                    let neighbour = plan.room(exit.target_room).expect("a joined exit");
                    let back = neighbour
                        .layout
                        .exits
                        .iter()
                        .find(|e| e.target_room == room.instance);
                    assert!(back.is_some(), "seed {seed}: a one-way exit");
                }
            }
        }
    }
}

#[test]
fn ids_follow_the_rules_whichever_start_and_boss_are_picked() {
    let mut starts = HashSet::new();
    let mut bosses = HashSet::new();
    for seed in 0..100 {
        let plan = plan(seed);
        let config = pieces::config();
        assert_eq!(plan.rooms[0].instance, config.start_room);
        starts.insert(format!("{:?}", plan.rooms[0].piece));
        let boss = plan.room(config.final_boss_room).expect("a boss room");
        bosses.insert(format!("{:?}", boss.piece));
        assert!([START_A, START_B].contains(&plan.rooms[0].piece));
        assert!([BOSS_A, BOSS_B].contains(&boss.piece));
    }
    assert_eq!((starts.len(), bosses.len()), (2, 2));
}

// ---- errors ----

#[test]
fn a_pool_with_no_boss_piece_says_so() {
    let pool = without(pieces::pool(), &[BOSS_A, BOSS_B]);
    let error = plan_of(&pool, &pieces::rules(), 1).unwrap_err();
    assert!(
        matches!(
            error,
            GenError::NoPieceWithRole {
                role: PieceRole::Boss
            }
        ),
        "{error:?}"
    );
    assert_eq!(error.to_string(), "the pool has no Boss piece");
}

#[test]
fn a_pool_with_no_start_piece_says_so() {
    let pool = without(pieces::pool(), &[START_A, START_B]);
    let error = plan_of(&pool, &pieces::rules(), 1).unwrap_err();
    assert_eq!(error.to_string(), "the pool has no Start piece");
}

#[test]
fn asking_for_more_keys_than_key_slots_says_so() {
    let pool = without(pieces::pool(), &[KEY_ENDS[1], KEY_ENDS[2]]);
    let rules = GeneratorRules {
        keys: CountRange { min: 2, max: 3 },
        ..pieces::rules()
    };
    let error = plan_of(&pool, &rules, 1).unwrap_err();
    assert!(
        matches!(error, GenError::NotEnoughKeySlots { need: 2, have: 1 }),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .starts_with("keys.min is 2 but only 1 Key slot exists"),
        "{error}"
    );
}

#[test]
fn fewer_pieces_than_rooms_min_says_so() {
    let pool = without(
        pieces::pool(),
        &[
            FORKS[0],
            FORKS[1],
            FORKS[2],
            FORKS[3],
            KEY_ENDS[0],
            KEY_ENDS[1],
            KEY_ENDS[2],
            DASH_CROSSINGS[0],
            DASH_CROSSINGS[1],
        ],
    );
    let rules = GeneratorRules {
        rooms: CountRange { min: 12, max: 13 },
        keys: CountRange { min: 0, max: 0 },
        ..pieces::rules()
    };
    let error = plan_of(&pool, &rules, 1).unwrap_err();
    assert!(matches!(error, GenError::TooFewPieces { .. }), "{error}");
    assert!(error.to_string().contains("rooms.min"), "{error}");
}

#[test]
fn a_pool_with_no_opposite_ports_says_so() {
    let (start, boss, middle) = (
        |id, tag| {
            pieces::room(id, tag, "S")
                .piece(PieceRole::Start, 1)
                .exits(&[pieces::Side::Left])
                .build()
        },
        |id, tag| {
            pieces::room(id, tag, "B")
                .piece(PieceRole::Boss, 1)
                .exits(&[pieces::Side::Left])
                .build()
        },
        |id, tag| {
            pieces::room(id, tag, "M")
                .piece(PieceRole::Middle, 1)
                .exits(&[pieces::Side::Left])
                .build()
        },
    );
    let pool = vec![
        start(START_A, 1),
        boss(BOSS_A, 2),
        middle(CORRIDORS[0], 3),
        middle(CORRIDORS[1], 4),
    ];
    let rules = GeneratorRules {
        rooms: CountRange { min: 3, max: 4 },
        spine: CountRange { min: 2, max: 4 },
        keys: CountRange { min: 0, max: 0 },
        mirror_percent: 0.0,
        ..pieces::rules()
    };
    let error = plan_of(&pool, &rules, 1).unwrap_err();
    assert!(matches!(error, GenError::NoPairablePorts), "{error}");
}

#[test]
fn bad_rules_are_refused_naming_the_field() {
    let rules = GeneratorRules {
        attempts: 0,
        ..pieces::rules()
    };
    let error = plan_of(&pieces::pool(), &rules, 1).unwrap_err();
    assert!(error.to_string().contains("generator.attempts"), "{error}");
}

#[test]
fn a_room_without_a_piece_mark_is_refused() {
    let mut pool = pieces::pool();
    pool[0].1.piece = None;
    let error = plan_of(&pool, &pieces::rules(), 1).unwrap_err();
    assert!(matches!(error, GenError::NotAPiece { .. }), "{error}");
}

// ---- the rule check itself must be able to fail ----

fn first_plan_with_a_lock() -> DungeonPlan {
    (0..200)
        .map(plan)
        .find(|plan| {
            let exits = plan.rooms.iter().flat_map(|room| &room.layout.exits);
            exits.into_iter().any(|exit| exit.locked_by.is_some())
        })
        .expect("a plan with a lock in 200 seeds")
}

fn check_rules(plan: &DungeonPlan) {
    let (pool, hub, config, rules) = (
        pieces::pool(),
        pieces::hub(),
        pieces::config(),
        pieces::rules(),
    );
    assert_plan_valid(plan, &pool, &hub, &config, &rules);
}

#[test]
fn the_rule_check_accepts_an_untouched_plan() {
    check_rules(&first_plan_with_a_lock());
}

#[test]
#[should_panic(expected = "spawn")]
fn the_rule_check_notices_an_exit_aimed_at_the_wrong_spot() {
    let mut plan = first_plan_with_a_lock();
    plan.rooms[0].layout.exits[0].spawn.0 += 40.0;
    check_rules(&plan);
}

#[test]
#[should_panic(expected = "key 1 lies 0 times")]
fn the_rule_check_notices_a_key_that_is_missing() {
    let mut plan = first_plan_with_a_lock();
    for room in &mut plan.rooms {
        room.layout
            .items
            .retain(|placed| placed.item != hollow_steps::item::Item::Key(1));
    }
    check_rules(&plan);
}

#[test]
#[should_panic(expected = "lock numbers")]
fn the_rule_check_notices_locks_numbered_out_of_order() {
    let mut plan = first_plan_with_a_lock();
    for room in &mut plan.rooms {
        for exit in &mut room.layout.exits {
            if exit.locked_by == Some(1) {
                exit.locked_by = Some(2);
            }
        }
    }
    check_rules(&plan);
}
