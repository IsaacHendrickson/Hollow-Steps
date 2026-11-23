//! `assert_plan_valid`: checks a generated dungeon plan against the generator's rules 1 to 11
//! (`docs/dungeon-gen.md`) from the plan and the pool alone, without the generator's own code.
use hollow_steps::ability::Ability;
use hollow_steps::dungeon_check::check;
use hollow_steps::dungeon_gen::DungeonPlan;
use hollow_steps::editor::{ExitPlacement, RoomLayout};
use hollow_steps::exit_arrival::{arrival_error, arrival_next_to};
use hollow_steps::gates::{Entrance, ExitRef};
use hollow_steps::item::Item;
use hollow_steps::mirror::mirror_layout;
use hollow_steps::piece::{PieceRole, SlotKind};
use hollow_steps::room::{RoomEdge, RoomId};
use hollow_steps::run::RunConfig;
use hollow_steps::variety::GeneratorRules;
use std::collections::HashMap;

/// What the checks found out, for tests that want more than "valid".
pub struct PlanFacts {
    /// The rooms from the start room to the boss room, as instance ids.
    pub spine: Vec<RoomId>,
    /// How many exits are locked.
    pub locks: usize,
}

type Rooms = [(RoomId, RoomLayout)];

fn opposite(edge: RoomEdge) -> RoomEdge {
    match edge {
        RoomEdge::Left => RoomEdge::Right,
        RoomEdge::Right => RoomEdge::Left,
        RoomEdge::Top => RoomEdge::Bottom,
        RoomEdge::Bottom => RoomEdge::Top,
    }
}

fn pairs(a: &ExitPlacement, b: &ExitPlacement) -> bool {
    if a.door.is_some() || b.door.is_some() {
        return a.door.is_some() && b.door.is_some();
    }
    b.edge == opposite(a.edge)
}

/// The piece as the plan placed it.
fn oriented(pieces: &Rooms, id: RoomId, mirrored: bool) -> RoomLayout {
    let (_, layout) = pieces
        .iter()
        .find(|(known, _)| *known == id)
        .unwrap_or_else(|| panic!("{id:?} is not in the pool"));
    if mirrored {
        mirror_layout(layout)
    } else {
        layout.clone()
    }
}

/// Rooms reached from the start room, without crossing an exit `blocked` says no to. `blocked`
/// gets the room being left, the exit and the room the player came from.
fn reach(
    plan: &DungeonPlan,
    blocked: &dyn Fn(&RoomLayout, &ExitPlacement, Option<RoomId>) -> bool,
    start: RoomId,
) -> Vec<RoomId> {
    let mut seen = vec![(start, None)];
    let mut next = 0;
    while next < seen.len() {
        let (id, from) = seen[next];
        next += 1;
        let room = plan.room(id).expect("a room of the plan");
        for exit in &room.layout.exits {
            if Some(exit.target_room) == from || blocked(&room.layout, exit, from) {
                continue;
            }
            seen.push((exit.target_room, Some(id)));
        }
    }
    seen.into_iter().map(|(id, _)| id).collect()
}

/// Whether leaving `layout` by `exit`, having come from `from`, needs `ability`.
fn gated_by(
    layout: &RoomLayout,
    exit: &ExitPlacement,
    from: Option<RoomId>,
    ability: Ability,
) -> bool {
    layout.gates.iter().any(|gate| {
        let entered = match (gate.from, from) {
            (Entrance::Spawn, None) => true,
            (Entrance::Arrival { room, .. }, Some(came)) => room == came,
            _ => false,
        };
        entered && gate.exit == ExitRef::of(exit) && gate.needs.contains(&ability)
    })
}

/// Panics, naming the seed, unless `plan` follows every rule. Returns what it found.
pub fn assert_plan_valid(
    plan: &DungeonPlan,
    pieces: &Rooms,
    hub: &(RoomId, RoomLayout),
    config: &RunConfig,
    rules: &GeneratorRules,
) -> PlanFacts {
    let seed = plan.seed;
    let role_of = |id: RoomId| {
        let layout = &pieces.iter().find(|(known, _)| *known == id).unwrap().1;
        layout.piece.expect("a piece").role
    };

    // 1. Instance ids, and a piece used at most once.
    let mut used: Vec<RoomId> = plan.rooms.iter().map(|room| room.piece).collect();
    used.sort_by_key(|id| format!("{id:?}"));
    used.dedup();
    assert_eq!(used.len(), plan.rooms.len(), "seed {seed}: a piece twice");
    let mut instances: Vec<RoomId> = plan.rooms.iter().map(|room| room.instance).collect();
    instances.sort_by_key(|id| format!("{id:?}"));
    instances.dedup();
    assert_eq!(
        instances.len(),
        plan.rooms.len(),
        "seed {seed}: an id twice"
    );
    let mut starts = 0;
    let mut bosses = 0;
    for room in &plan.rooms {
        match role_of(room.piece) {
            PieceRole::Start => {
                starts += 1;
                assert_eq!(room.instance, config.start_room, "seed {seed}: start id");
            }
            PieceRole::Boss => {
                bosses += 1;
                assert_eq!(
                    room.instance, config.final_boss_room,
                    "seed {seed}: boss id"
                );
            }
            PieceRole::Middle => {
                assert_eq!(
                    room.instance, room.piece,
                    "seed {seed}: a middle keeps its id"
                );
            }
        }
    }
    assert_eq!((starts, bosses), (1, 1), "seed {seed}: one start, one boss");
    assert_eq!(plan.rooms[0].instance, config.start_room, "seed {seed}");
    assert!(plan.room(config.final_boss_room).is_some(), "seed {seed}");

    // 2, 4, 11. Each room is its piece (mirrored or not), with unpaired exits, their gates and
    // the slots gone, and the keys and abilities lying where slots were.
    let mut extras: HashMap<RoomId, Vec<Item>> = HashMap::new();
    for room in &plan.rooms {
        let source = oriented(pieces, room.piece, room.mirrored);
        let here = &room.layout;
        assert!(
            here.slots.is_empty(),
            "seed {seed}: {:?} kept slots",
            room.instance
        );
        assert!(
            here.piece.is_none(),
            "seed {seed}: {:?} kept its mark",
            room.instance
        );
        assert_eq!(
            here.terrain, source.terrain,
            "seed {seed}: {:?}",
            room.instance
        );
        assert_eq!(here.spawn, source.spawn, "seed {seed}: {:?}", room.instance);
        let mut last = 0;
        for exit in &here.exits {
            let at = source.exits[last..]
                .iter()
                .position(|known| ExitRef::of(known) == ExitRef::of(exit))
                .unwrap_or_else(|| panic!("seed {seed}: {:?} has a new exit", room.instance));
            last += at + 1;
        }
        let authored = source.items.len();
        assert_eq!(
            here.items[..authored],
            source.items[..],
            "seed {seed}: items"
        );
        let mut free: Vec<_> = source.slots.iter().collect();
        for placed in &here.items[authored..] {
            let slot = free
                .iter()
                .position(|slot| slot.position == placed.position)
                .unwrap_or_else(|| panic!("seed {seed}: an item off its slot"));
            let slot = free.remove(slot);
            let fits = match placed.item {
                Item::Key(_) => matches!(slot.kind, SlotKind::Key | SlotKind::Any),
                item if item.grants().is_some() => {
                    matches!(slot.kind, SlotKind::Ability | SlotKind::Any)
                }
                _ => false,
            };
            assert!(
                fits,
                "seed {seed}: {:?} in a {:?} slot",
                placed.item, slot.kind
            );
            extras.entry(room.instance).or_default().push(placed.item);
        }
    }

    // 3. Two-way links, paired ports, arrivals.
    let mut links = 0;
    for room in &plan.rooms {
        for exit in &room.layout.exits {
            let there = plan
                .room(exit.target_room)
                .unwrap_or_else(|| panic!("seed {seed}: an exit to nowhere"));
            let back: Vec<usize> = there
                .layout
                .exits
                .iter()
                .enumerate()
                .filter(|(_, other)| other.target_room == room.instance)
                .map(|(place, _)| place)
                .collect();
            assert_eq!(back.len(), 1, "seed {seed}: not one link back");
            let other = &there.layout.exits[back[0]];
            assert!(pairs(exit, other), "seed {seed}: ports do not pair");
            let spot = arrival_next_to(&there.layout, back[0]).expect("an arrival");
            assert_eq!(exit.spawn, (spot.x, spot.y), "seed {seed}: spawn");
            assert_eq!(arrival_error(&there.layout, spot), None, "seed {seed}");
            links += 1;
        }
    }
    assert_eq!(links, 2 * (plan.rooms.len() - 1), "seed {seed}: not a tree");
    let everyone = reach(plan, &|_, _, _| false, config.start_room);
    assert_eq!(everyone.len(), plan.rooms.len(), "seed {seed}: cut off");

    // 5. Gates: each one a piece had, whose entry and exit are both still joined, is carried.
    for room in &plan.rooms {
        let source = oriented(pieces, room.piece, room.mirrored);
        let mut carried = 0;
        for gate in &source.gates {
            let Some(exit) = source.exits.iter().find(|e| ExitRef::of(e) == gate.exit) else {
                continue;
            };
            let kept = room
                .layout
                .exits
                .iter()
                .any(|e| ExitRef::of(e) == ExitRef::of(exit));
            let entry_joined = match gate.from {
                Entrance::Spawn => true,
                Entrance::Arrival { room: target, .. } => source
                    .exits
                    .iter()
                    .filter(|e| e.target_room == target)
                    .all(|e| {
                        room.layout
                            .exits
                            .iter()
                            .any(|k| ExitRef::of(k) == ExitRef::of(e))
                    }),
            };
            carried += usize::from(kept && entry_joined);
        }
        assert_eq!(
            room.layout.gates.len(),
            carried,
            "seed {seed}: gates of {:?}",
            room.instance
        );
        for gate in &room.layout.gates {
            assert!(
                room.layout
                    .exits
                    .iter()
                    .any(|e| ExitRef::of(e) == gate.exit),
                "seed {seed}: a gate names a missing exit"
            );
            if let Entrance::Arrival { room: from, exit } = gate.from {
                let neighbour = plan.room(from).expect("a neighbour");
                let towards = neighbour
                    .layout
                    .exits
                    .iter()
                    .find(|e| e.target_room == room.instance);
                assert_eq!(
                    towards.map(ExitRef::of),
                    Some(exit),
                    "seed {seed}: a gate arrives from the wrong exit"
                );
            }
        }
    }

    // 6. The hub keeps its exits; the ones to the start room arrive at its spawn.
    assert_eq!(plan.hub_room, hub.0);
    assert_eq!(plan.hub.exits.len(), hub.1.exits.len());
    for (new, old) in plan.hub.exits.iter().zip(&hub.1.exits) {
        assert_eq!(new.target_room, old.target_room, "seed {seed}: hub target");
        assert_eq!(ExitRef::of(new), ExitRef::of(old));
        if new.target_room == config.start_room {
            assert_eq!(
                new.spawn, plan.rooms[0].layout.spawn,
                "seed {seed}: hub spawn"
            );
        } else {
            assert_eq!(new.spawn, old.spawn);
        }
    }

    // 7. The spine: the way from the start room to the boss room.
    let spine = spine_of(plan, config);
    let (low, high) = (rules.spine.min as usize, rules.spine.max as usize);
    assert!(
        (low..=high).contains(&spine.len()),
        "seed {seed}: spine of {}",
        spine.len()
    );
    let (low, high) = (rules.rooms.min as usize, rules.rooms.max as usize);
    assert!(
        (low..=high).contains(&plan.rooms.len()),
        "seed {seed}: {} rooms",
        plan.rooms.len()
    );

    // 8. Locks: on spine links, leading away from the start, numbered in spine order.
    let mut locks = Vec::new();
    for room in &plan.rooms {
        for exit in &room.layout.exits {
            if let Some(number) = exit.locked_by {
                let at = spine.iter().position(|id| *id == room.instance);
                let away = at.and_then(|at| spine.get(at + 1)) == Some(&exit.target_room);
                assert!(
                    away,
                    "seed {seed}: a lock off the spine or toward the start"
                );
                locks.push((at.unwrap(), number));
            }
        }
    }
    locks.sort_unstable();
    let numbers: Vec<u8> = locks.iter().map(|(_, number)| *number).collect();
    let expected: Vec<u8> = (1..=locks.len() as u8).collect();
    assert_eq!(numbers, expected, "seed {seed}: lock numbers");
    let (low, high) = (rules.keys.min as usize, rules.keys.max as usize);
    assert!(
        (low..=high).contains(&locks.len()),
        "seed {seed}: {} locks",
        locks.len()
    );

    // 9. Key i lies once, in a room the player reaches without crossing lock i or later.
    for number in 1..=locks.len() as u8 {
        let holders: Vec<RoomId> = extras
            .iter()
            .filter(|(_, items)| items.contains(&Item::Key(number)))
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(
            holders.len(),
            1,
            "seed {seed}: key {number} lies {} times",
            holders.len()
        );
        let open = reach(
            plan,
            &|_, exit, _| exit.locked_by.is_some_and(|lock| lock >= number),
            config.start_room,
        );
        assert!(
            open.contains(&holders[0]),
            "seed {seed}: key {number} is behind lock {number}"
        );
    }

    // 10. Each ability a gate needs lies in the dungeon, reachable before the first such gate.
    for ability in Ability::ALL {
        let needed = plan.rooms.iter().any(|room| {
            room.layout
                .gates
                .iter()
                .any(|gate| gate.needs.contains(ability))
        });
        if !needed {
            continue;
        }
        let free = reach(
            plan,
            &|layout, exit, from| gated_by(layout, exit, from, *ability),
            config.start_room,
        );
        let item_in = |room: &hollow_steps::dungeon_gen::PlannedRoom| {
            room.layout
                .items
                .iter()
                .any(|placed| placed.item.grants() == Some(*ability))
        };
        assert!(
            plan.rooms
                .iter()
                .any(|room| free.contains(&room.instance) && item_in(room)),
            "seed {seed}: {ability:?} is needed and not found before its first gate"
        );
        let placed = extras
            .values()
            .flatten()
            .filter(|item| item.grants() == Some(*ability));
        assert!(placed.count() <= 1, "seed {seed}: {ability:?} placed twice");
    }

    // The checker has the last word.
    let problems = check(&plan.catalog(), config);
    assert!(
        problems.is_empty(),
        "seed {seed}: {:?}",
        problems
            .iter()
            .map(|problem| problem.text())
            .collect::<Vec<_>>()
    );
    PlanFacts {
        spine,
        locks: locks.len(),
    }
}

/// The rooms on the way from the start room to the boss room.
fn spine_of(plan: &DungeonPlan, config: &RunConfig) -> Vec<RoomId> {
    let mut path = vec![config.start_room];
    let mut came = None;
    while *path.last().unwrap() != config.final_boss_room {
        let here = plan.room(*path.last().unwrap()).unwrap();
        // Follow the exit whose side of the tree holds the boss room.
        let step = here
            .layout
            .exits
            .iter()
            .filter(|exit| Some(exit.target_room) != came)
            .find(|exit| holds_boss(plan, exit.target_room, here.instance, config))
            .expect("the boss room is in the tree");
        came = Some(here.instance);
        path.push(step.target_room);
    }
    path
}

/// Whether the boss room lies on the side of `room` away from `parent`.
fn holds_boss(plan: &DungeonPlan, room: RoomId, parent: RoomId, config: &RunConfig) -> bool {
    if room == config.final_boss_room {
        return true;
    }
    let layout = &plan.room(room).unwrap().layout;
    layout
        .exits
        .iter()
        .filter(|exit| exit.target_room != parent)
        .any(|exit| holds_boss(plan, exit.target_room, room, config))
}
