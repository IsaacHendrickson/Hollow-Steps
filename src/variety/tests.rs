//! The rule of optional placements with no `GameState`: which enemies a seed leaves in a room.
//!
//! The room is `SCENE`: three enemies in group 1, one with chance 50, one that is always there,
//! and one with chance 0 that has the same patrol as the first (a twin, so the identities have
//! to tell it apart). An enemy is named here by the left end of its patrol.
use super::*;
use crate::player::WorldBounds;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

/// (patrol left, patrol right, group, chance), in file order.
const SCENE: [(f32, f32, Option<u8>, u8); 6] = [
    (100.0, 200.0, Some(1), 100),
    (220.0, 320.0, Some(1), 100),
    (340.0, 440.0, Some(1), 100),
    (460.0, 540.0, None, 50),
    (560.0, 800.0, None, 100),
    (100.0, 200.0, None, 0),
];

fn enemies(scene: &[(f32, f32, Option<u8>, u8)]) -> Vec<Enemy> {
    let mut list: Vec<Enemy> = scene
        .iter()
        .map(|(left, right, group, chance)| {
            Enemy::new(*left, *right, 0.0, 0.0, BOUNDS).with_variety(*group, *chance)
        })
        .collect();
    mark_identities(&mut list);
    list
}

/// The patrol lefts of what `seed` leaves of `scene` in `room`.
fn stays(scene: &[(f32, f32, Option<u8>, u8)], room: RoomId, seed: u64) -> Vec<f32> {
    let mut list = enemies(scene);
    thin(&mut list, room, seed);
    list.iter().map(|enemy| enemy.placement_key().1).collect()
}

fn scene(seed: u64) -> Vec<f32> {
    stays(&SCENE, RoomId::First, seed)
}

#[test]
fn these_seeds_leave_exactly_these_enemies() {
    // Worked out apart from this code (FNV-1a identities, streams, the picks by hand), so a
    // change to the keys, the mixing or the rule fails here: it would change every run.
    assert_eq!(scene(1), [340.0, 460.0, 560.0]);
    assert_eq!(scene(2), [340.0, 560.0]);
    assert_eq!(scene(3), [220.0, 460.0, 560.0]);
    assert_eq!(scene(0xABC), [220.0, 460.0, 560.0]);
    assert_eq!(scene(u64::MAX), [340.0, 560.0]);
}

#[test]
fn the_same_seed_gives_the_same_enemies_and_the_seeds_do_not_all_agree() {
    for seed in [0, 1, 7, u64::MAX] {
        assert_eq!(scene(seed), scene(seed));
    }
    let outcomes: BTreeSet<String> = (1..=200).map(|seed| format!("{:?}", scene(seed))).collect();
    assert!(outcomes.len() >= 4, "200 seeds gave only {outcomes:?}");
}

#[test]
fn over_200_seeds_a_group_of_three_shows_all_three_and_a_chance_50_enemy_a_middle_share() {
    let mut picked = BTreeSet::new();
    let mut present = 0;
    for seed in 1..=200 {
        let left = scene(seed);
        picked.extend(
            left.iter()
                .copied()
                .filter(|x| [100.0, 220.0, 340.0].contains(x))
                .map(|x| x as u32),
        );
        present += usize::from(left.contains(&460.0));
    }
    assert_eq!(
        picked.len(),
        3,
        "every member of the group is picked sometimes: {picked:?}"
    );
    assert!(
        (70..=130).contains(&present),
        "chance 50 was present in {present} of 200"
    );
}

#[test]
fn a_group_keeps_exactly_one_member_and_the_rest_keep_their_order() {
    for seed in 1..=200 {
        let left = scene(seed);
        let members = left
            .iter()
            .filter(|x| [100.0, 220.0, 340.0].contains(x))
            .count();
        assert_eq!(members, 1, "seed {seed}: {left:?}");
        assert!(
            left.windows(2).all(|pair| pair[0] < pair[1]),
            "file order: {left:?}"
        );
        assert!(
            left.contains(&560.0),
            "an ungrouped chance-100 enemy is always there"
        );
    }
}

#[test]
fn chance_0_never_stays_and_chance_100_always_does() {
    // The twin of the first placement has chance 0: the other enemy with that patrol is the
    // grouped one, so count the ones that are not.
    for seed in 1..=200 {
        let mut list = enemies(&SCENE);
        thin(&mut list, RoomId::First, seed);
        assert!(
            list.iter()
                .all(|enemy| !(enemy.group().is_none() && enemy.chance() == 0)),
            "seed {seed}: chance 0 stayed"
        );
        assert!(list
            .iter()
            .any(|enemy| enemy.chance() == 100 && enemy.group().is_none()));
    }
}

#[test]
fn a_group_of_one_keeps_its_member_and_two_groups_choose_apart() {
    let one = [(100.0, 200.0, Some(4), 100)];
    assert!((1..=50).all(|seed| stays(&one, RoomId::First, seed) == [100.0]));

    let two = [
        (100.0, 200.0, Some(1), 100),
        (220.0, 320.0, Some(1), 100),
        (340.0, 440.0, Some(2), 100),
        (460.0, 540.0, Some(2), 100),
    ];
    let mut combos = BTreeSet::new();
    for seed in 1..=200 {
        let left = stays(&two, RoomId::First, seed);
        assert_eq!(left.len(), 2, "one of each group: {left:?}");
        assert!(left[0] < 300.0 && left[1] > 300.0);
        combos.insert(format!("{left:?}"));
    }
    assert_eq!(combos.len(), 4, "the two draws are independent: {combos:?}");
}

#[test]
fn another_room_draws_apart_from_this_one() {
    let differs = (1..=200)
        .any(|seed| stays(&SCENE, RoomId::First, seed) != stays(&SCENE, RoomId::Custom(3), seed));
    assert!(differs, "the room is part of every key");
    // And a room still gives the same answer for the same seed.
    assert_eq!(
        stays(&SCENE, RoomId::Custom(3), 9),
        stays(&SCENE, RoomId::Custom(3), 9)
    );
}

#[test]
fn identical_placements_are_told_apart_and_roll_apart() {
    let twins = [(300.0, 400.0, None, 50), (300.0, 400.0, None, 50)];
    let list = enemies(&twins);
    assert_ne!(list[0].identity(), list[1].identity());
    let counts: BTreeSet<usize> = (1..=200)
        .map(|seed| stays(&twins, RoomId::First, seed).len())
        .collect();
    assert_eq!(
        counts,
        BTreeSet::from([0, 1, 2]),
        "they do not always roll together"
    );
}

#[test]
fn an_identity_is_the_same_wherever_the_room_is_rebuilt_and_ignores_the_tier() {
    let first = enemies(&SCENE);
    let again = enemies(&SCENE);
    let identities = |list: &[Enemy]| list.iter().map(Enemy::identity).collect::<Vec<_>>();
    assert_eq!(identities(&first), identities(&again));
    assert_eq!(
        identities(&first).iter().collect::<BTreeSet<_>>().len(),
        SCENE.len(),
        "six placements, six identities"
    );
    // A tier that leaves a placement out happens after the numbering and moves nobody's number.
    let mut left_out = enemies(&SCENE);
    let kept = left_out[5].identity();
    left_out.remove(0);
    assert_eq!(left_out[4].identity(), kept);
}

#[test]
fn an_enemy_with_no_placement_data_is_always_there() {
    let mut plain = vec![Enemy::new(300.0, 400.0, 0.0, 0.0, BOUNDS)];
    mark_identities(&mut plain);
    assert_eq!((plain[0].group(), plain[0].chance()), (None, FULL_CHANCE));
    for seed in 1..=50 {
        let mut list = plain.clone();
        thin(&mut list, RoomId::First, seed);
        assert_eq!(list.len(), 1);
    }
    let mut none: Vec<Enemy> = Vec::new();
    thin(&mut none, RoomId::First, 1);
    assert!(none.is_empty());
}

#[test]
fn a_group_number_is_1_to_9_and_a_chance_0_to_100() {
    assert!(check_group(0).unwrap_err().contains("group 0"));
    assert!((1..=MAX_GROUP).all(|group| check_group(group).is_ok()));
    assert!(check_group(10).unwrap_err().contains("1 to 9"));
    assert!(check_chance(0).is_ok() && check_chance(100).is_ok());
    assert!(check_chance(101).unwrap_err().contains("101"));
    assert!(check_chance(255).is_err());
}

#[test]
fn a_placement_is_checked_by_rule_and_the_error_names_it() {
    let placement = |group, chance| EnemyPlacement {
        group,
        chance,
        ..EnemyPlacement::new("chase", 300.0, 400.0)
    };
    assert!(placement(None, 100).check_variety(0).is_ok());
    assert!(placement(None, 0).check_variety(0).is_ok());
    assert!(placement(Some(9), 100).check_variety(0).is_ok());

    let error = placement(Some(2), 50).check_variety(3).unwrap_err();
    assert!(
        error.contains("enemy 3") && error.contains("chase"),
        "{error}"
    );
    assert!(
        error.contains("group 2") && error.contains("chance 100"),
        "{error}"
    );

    let error = placement(Some(0), 100).check_variety(1).unwrap_err();
    assert!(
        error.contains("enemy 1") && error.contains("1 to 9"),
        "{error}"
    );
    let error = placement(Some(10), 100).check_variety(1).unwrap_err();
    assert!(error.contains("group 10"), "{error}");
    let error = placement(None, 101).check_variety(2).unwrap_err();
    assert!(
        error.contains("enemy 2") && error.contains("101"),
        "{error}"
    );
}
