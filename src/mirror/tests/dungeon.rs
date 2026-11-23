//! The dungeon checker gives the same verdict on a dungeon and on its flip.
use std::mem::{discriminant, Discriminant};

use crate::dungeon_check::{check, Problem};
use crate::mirror::fixture::{self, BOSS, CLOSET, HUB, SECOND, START, TOWER};
use crate::mirror::flip_world;
use crate::room::RoomId;

fn verdict(good: bool, flipped: impl Fn(RoomId) -> bool) -> Vec<Problem> {
    let mut documents = fixture::dungeon(good);
    flip_world(&mut documents, flipped);
    check(&fixture::catalog(&documents), &fixture::run_config())
}

fn kinds(problems: &[Problem]) -> Vec<Discriminant<Problem>> {
    problems.iter().map(discriminant).collect()
}

#[test]
fn a_good_dungeon_has_no_problems_and_still_none_after_the_flip() {
    assert_eq!(verdict(true, |_| false), []);
    assert_eq!(verdict(true, |id| id != HUB), []);
}

#[test]
fn a_bad_dungeon_has_the_same_number_and_kinds_of_problems_after_the_flip() {
    let before = verdict(false, |_| false);
    let after = verdict(false, |id| id != HUB);
    assert!(
        before.len() >= 3,
        "made bad on purpose: {:?}",
        before.iter().map(Problem::text).collect::<Vec<_>>()
    );
    assert_eq!(after.len(), before.len());
    assert_eq!(kinds(&after), kinds(&before));
    // More than one kind, so the comparison says something.
    let distinct: std::collections::HashSet<_> = kinds(&before).into_iter().collect();
    assert!(distinct.len() >= 2);
}

#[test]
fn the_verdict_is_the_same_whichever_rooms_are_flipped() {
    let rooms = [START, SECOND, BOSS, TOWER, CLOSET];
    for good in [true, false] {
        let expected = verdict(good, |_| false);
        for bits in 0..32u32 {
            let flipped = |id: RoomId| {
                rooms
                    .iter()
                    .position(|r| *r == id)
                    .is_some_and(|i| bits & (1 << i) != 0)
            };
            let found = verdict(good, flipped);
            assert_eq!(found.len(), expected.len(), "good {good}, set {bits:05b}");
            assert_eq!(
                kinds(&found),
                kinds(&expected),
                "good {good}, set {bits:05b}"
            );
        }
    }
}
