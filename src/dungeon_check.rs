//! The dungeon checker (`docs/dungeon-check.md`): reads every room's layout and the run rules and
//! lists what would stop a player, or what makes the marks that say so (`docs/gates.md`) wrong.
//! Pure: no files, no game, no editor. The editor's "Check dungeon" and the shipped-content test
//! both call `check`.
//!
//! The graph's nodes are `(room, entrance)` pairs (`gates::entrances_of`); taking an exit of a
//! room from one of its entrances leads to the arrival node of the room it opens into. The walk
//! starts with no abilities and no keys. It collects the pickups of every room it enters, opens
//! key locks for keys held, lets a gate through when every ability it needs is held, and repeats
//! until nothing changes. A key is looked at by its number, not counted: how many locks and keys
//! there are is its own rule, and which door spends a key is not searched.
use std::collections::{BTreeMap, HashSet};

use crate::ability::Ability;
use crate::editor::RoomLayout;
use crate::gates::{entrances_of, Entrance, ExitRef};
use crate::item::Item;
use crate::room::RoomId;
use crate::room_name::room_title;
use crate::run::RunConfig;

mod editor;
mod problem;
#[cfg(test)]
mod tests;

pub use problem::Problem;

/// Checks the dungeon made of `rooms` (every room, the hub included) under the run rules
/// `config`. Empty when nothing is wrong. The order is fixed: run rules, unreachable rooms, locks,
/// key counts, abilities, traps, gates that name no entrance.
pub fn check(rooms: &[(RoomId, RoomLayout)], config: &RunConfig) -> Vec<Problem> {
    let graph = Graph::new(rooms, config);
    let mut problems = Vec::new();
    let Some(starts) = graph.starts(&mut problems) else {
        return problems;
    };
    let walk = graph.walk(&starts, None);
    graph.unreachable(&walk, &mut problems);
    graph.locks(&walk, &starts, &mut problems);
    graph.key_counts(&mut problems);
    graph.abilities(&walk, &mut problems);
    graph.traps(&starts, &mut problems);
    graph.orphan_gates(&mut problems);
    problems
}

/// A node: the index of a room and of one of its entrances.
type Node = (usize, usize);

fn ability_bit(ability: Ability) -> u8 {
    let place = Ability::ALL.iter().position(|known| *known == ability);
    place.map_or(0, |place| 1 << place)
}

fn abilities_of(mask: u8) -> Vec<Ability> {
    Ability::ALL
        .iter()
        .copied()
        .filter(|ability| mask & ability_bit(*ability) != 0)
        .collect()
}

/// The bit of key `id`; no bit for a number outside 1 to 8, so such a key is never held.
fn key_bit(id: u8) -> u16 {
    match id {
        1..=8 => 1 << id,
        _ => 0,
    }
}

struct DungeonRoom<'a> {
    id: RoomId,
    layout: &'a RoomLayout,
    title: String,
    entrances: Vec<Entrance>,
    /// Every ability and key lying in the room; entering the room collects them.
    abilities: u8,
    keys: u16,
}

struct Graph<'a> {
    rooms: Vec<DungeonRoom<'a>>,
    /// `edges[room][exit]`: the arrival node that exit leads to, if its room exists.
    edges: Vec<Vec<Option<Node>>>,
    hub: Option<usize>,
    start: Option<usize>,
    boss: Option<usize>,
    /// The rooms `run.ron` names that do not exist, found while building.
    missing: Vec<Problem>,
}

/// What a walk found: the nodes it reached and what it picked up on the way.
struct Walk {
    reached: Vec<Vec<bool>>,
    abilities: u8,
    keys: u16,
}

impl Walk {
    fn room_reached(&self, room: usize) -> bool {
        self.reached[room].iter().any(|reached| *reached)
    }
}

impl<'a> Graph<'a> {
    fn new(catalog: &'a [(RoomId, RoomLayout)], config: &RunConfig) -> Self {
        let rooms: Vec<DungeonRoom> = catalog
            .iter()
            .map(|(id, layout)| {
                let mut abilities = 0;
                let mut keys = 0;
                for placed in &layout.items {
                    abilities |= placed.item.grants().map_or(0, ability_bit);
                    keys |= match placed.item {
                        Item::Key(id) => key_bit(id),
                        _ => 0,
                    };
                }
                DungeonRoom {
                    id: *id,
                    layout,
                    title: room_title(*id, layout.name.as_deref()),
                    entrances: entrances_of(*id, catalog)
                        .into_iter()
                        .map(|(entrance, _)| entrance)
                        .collect(),
                    abilities,
                    keys,
                }
            })
            .collect();
        let index_of = |id: RoomId| rooms.iter().position(|room| room.id == id);
        let edges = rooms
            .iter()
            .map(|room| {
                room.layout
                    .exits
                    .iter()
                    .map(|exit| {
                        let target = index_of(exit.target_room)?;
                        let arrival = Entrance::Arrival {
                            room: room.id,
                            exit: ExitRef::of(exit),
                        };
                        let entrance =
                            rooms[target].entrances.iter().position(|e| *e == arrival)?;
                        Some((target, entrance))
                    })
                    .collect()
            })
            .collect();
        let mut missing = Vec::new();
        let mut named = |field, id: Option<RoomId>| {
            let id = id?;
            let found = index_of(id);
            if found.is_none() {
                missing.push(Problem::MissingRoom { field, room: id });
            }
            found
        };
        let hub = named("hub_room", config.hub_room);
        let start = named("start_room", Some(config.start_room));
        let boss = named("final_boss_room", Some(config.final_boss_room));
        Self {
            rooms,
            edges,
            hub,
            start,
            boss,
            missing,
        }
    }

    fn title(&self, room: usize) -> String {
        self.rooms[room].title.clone()
    }

    fn is_hub(&self, room: usize) -> bool {
        self.hub == Some(room)
    }

    /// Where a run begins: through each exit of the hub that leads to the start room, or at the
    /// start room's spawn when there is no hub (or no such exit, which is reported). `None`
    /// when the start room does not exist: nothing can be checked then.
    fn starts(&self, problems: &mut Vec<Problem>) -> Option<Vec<Node>> {
        problems.extend(self.missing.iter().cloned());
        let start = self.start?;
        let mut starts = Vec::new();
        if let Some(hub) = self.hub {
            for target in self.edges[hub].iter().flatten() {
                if target.0 == start {
                    starts.push(*target);
                }
            }
            if starts.is_empty() {
                problems.push(Problem::HubDoesNotLeadToStart {
                    hub: self.title(hub),
                    start: self.title(start),
                });
            }
        }
        if starts.is_empty() {
            let spawn = self.rooms[start]
                .entrances
                .iter()
                .position(|entrance| *entrance == Entrance::Spawn)?;
            starts.push((start, spawn));
        }
        Some(starts)
    }

    /// What going from entrance `entrance` of `room` through its exit `exit` asks for.
    fn needs(&self, room: usize, entrance: usize, exit: usize) -> u8 {
        let layout = self.rooms[room].layout;
        let from = &self.rooms[room].entrances[entrance];
        layout
            .gate_needs(from, &ExitRef::of(&layout.exits[exit]))
            .iter()
            .fold(0, |mask, ability| mask | ability_bit(*ability))
    }

    /// Whether that exit can be taken from that entrance holding `abilities` and `keys`;
    /// `unlocked` is one lock to treat as open (to ask what lies behind it).
    fn can_take(
        &self,
        (room, entrance): Node,
        exit: usize,
        (abilities, keys): (u8, u16),
        unlocked: Option<(usize, usize)>,
    ) -> bool {
        let locked = self.rooms[room].layout.exits[exit].locked_by;
        let key_ok = locked.is_none_or(|id| {
            unlocked == Some((room, exit)) || (key_bit(id) != 0 && keys & key_bit(id) != 0)
        });
        key_ok && self.needs(room, entrance, exit) & !abilities == 0
    }

    /// The fixpoint walk from `starts`. Taking an exit into the hub ends a run, so the walk never
    /// goes on from there. `unlocked` is one lock to treat as open.
    fn walk(&self, starts: &[Node], unlocked: Option<(usize, usize)>) -> Walk {
        let mut walk = Walk {
            reached: self
                .rooms
                .iter()
                .map(|room| vec![false; room.entrances.len()])
                .collect(),
            abilities: 0,
            keys: 0,
        };
        for node in starts {
            self.reach(&mut walk, *node);
        }
        loop {
            let mut found = Vec::new();
            for (room, entrances) in walk.reached.iter().enumerate() {
                for (entrance, _) in entrances.iter().enumerate().filter(|(_, r)| **r) {
                    for (exit, target) in self.edges[room].iter().enumerate() {
                        let held = (walk.abilities, walk.keys);
                        let Some(target) = target.filter(|node| !self.is_hub(node.0)) else {
                            continue;
                        };
                        if !walk.reached[target.0][target.1]
                            && self.can_take((room, entrance), exit, held, unlocked)
                        {
                            found.push(target);
                        }
                    }
                }
            }
            if found.is_empty() {
                return walk;
            }
            for node in found {
                self.reach(&mut walk, node);
            }
        }
    }

    /// Marks a node reached and picks up what lies in its room.
    fn reach(&self, walk: &mut Walk, (room, entrance): Node) {
        walk.reached[room][entrance] = true;
        walk.abilities |= self.rooms[room].abilities;
        walk.keys |= self.rooms[room].keys;
    }

    // ---- the rules ----

    fn unreachable(&self, walk: &Walk, problems: &mut Vec<Problem>) {
        for (room, _) in self.rooms.iter().enumerate() {
            if self.is_hub(room) || walk.room_reached(room) {
                continue;
            }
            problems.push(if self.boss == Some(room) {
                Problem::BossUnreachable {
                    room: self.title(room),
                }
            } else {
                Problem::Unreachable {
                    room: self.title(room),
                }
            });
        }
    }

    /// Every lock in a reachable room whose key the walk never picks up: no such key anywhere
    /// the player can go, or only behind that same lock (told apart by walking again with that
    /// one lock open).
    fn locks(&self, walk: &Walk, starts: &[Node], problems: &mut Vec<Problem>) {
        for (room, dungeon) in self.rooms.iter().enumerate() {
            if self.is_hub(room) || !walk.room_reached(room) {
                continue;
            }
            for (exit, placement) in dungeon.layout.exits.iter().enumerate() {
                let Some(id) = placement.locked_by else {
                    continue;
                };
                if walk.keys & key_bit(id) != 0 && key_bit(id) != 0 {
                    continue;
                }
                let opened = self.walk(starts, Some((room, exit)));
                let (title, exit_name) = (self.title(room), ExitRef::of(placement).describe());
                problems.push(if opened.keys & key_bit(id) != 0 {
                    Problem::KeyBehindItsOwnLock {
                        room: title,
                        exit: exit_name,
                        key: id,
                    }
                } else {
                    Problem::KeyUnreachable {
                        room: title,
                        exit: exit_name,
                        key: id,
                    }
                });
            }
        }
    }

    /// A key is used up by the exit it opens, so each number needs a key per lock.
    fn key_counts(&self, problems: &mut Vec<Problem>) {
        for id in 1..=8u8 {
            let locks = self
                .rooms
                .iter()
                .flat_map(|room| &room.layout.exits)
                .filter(|exit| exit.locked_by == Some(id))
                .count();
            let keys = self
                .rooms
                .iter()
                .flat_map(|room| &room.layout.items)
                .filter(|placed| placed.item == Item::Key(id))
                .count();
            if locks > keys {
                problems.push(Problem::TooManyLocks {
                    key: id,
                    locks,
                    keys,
                });
            }
        }
    }

    /// Gates that never open: the entrance is reached but an ability it needs is never picked
    /// up, and no other way into the room lets the player use that exit either.
    fn abilities(&self, walk: &Walk, problems: &mut Vec<Problem>) {
        for (room, dungeon) in self.rooms.iter().enumerate() {
            if self.is_hub(room) {
                continue;
            }
            for gate in &dungeon.layout.gates {
                let entrance = dungeon.entrances.iter().position(|e| *e == gate.from);
                let exit = dungeon
                    .layout
                    .exits
                    .iter()
                    .position(|placement| ExitRef::of(placement) == gate.exit);
                let (Some(entrance), Some(exit)) = (entrance, exit) else {
                    continue;
                };
                let missing = self.needs(room, entrance, exit) & !walk.abilities;
                if !walk.reached[room][entrance] || missing == 0 {
                    continue;
                }
                let held = (walk.abilities, walk.keys);
                let usable = (0..dungeon.entrances.len()).any(|other| {
                    walk.reached[room][other] && self.can_take((room, other), exit, held, None)
                });
                let problem = Problem::AbilityUnobtainable {
                    room: self.title(room),
                    exit: gate.exit.describe(),
                    needs: abilities_of(missing),
                };
                if !usable && !problems.contains(&problem) {
                    problems.push(problem);
                }
            }
        }
    }

    /// Walks every way a run can go, keeping what it has picked up on that way (unlike the
    /// fixpoint walk, which holds everything it ever found), and reports each entrance reached
    /// where no exit can be taken. A dead end, where the way back is open, is fine, and so is the
    /// final boss room: defeating its boss ends the run. Locks are looked at by key number.
    fn traps(&self, starts: &[Node], problems: &mut Vec<Problem>) {
        let mut seen: HashSet<(usize, usize, u8, u16)> = HashSet::new();
        let mut pending = Vec::new();
        let mut enter = |node: Node, held: (u8, u16), pending: &mut Vec<_>| {
            let state = (
                node.0,
                node.1,
                held.0 | self.rooms[node.0].abilities,
                held.1 | self.rooms[node.0].keys,
            );
            if seen.insert(state) {
                pending.push(state);
            }
        };
        for node in starts {
            enter(*node, (0, 0), &mut pending);
        }
        // Per entrance, the fewest abilities held in a state that has no way out.
        let mut trapped: BTreeMap<Node, u8> = BTreeMap::new();
        while let Some((room, entrance, abilities, keys)) = pending.pop() {
            if self.is_hub(room) {
                continue;
            }
            let held = (abilities, keys);
            let mut any = false;
            for (exit, target) in self.edges[room].iter().enumerate() {
                if !self.can_take((room, entrance), exit, held, None) {
                    continue;
                }
                any = true;
                if let Some(target) = target.filter(|node| !self.is_hub(node.0)) {
                    enter(target, held, &mut pending);
                }
            }
            if !any && self.boss != Some(room) {
                let fewest = trapped.entry((room, entrance)).or_insert(abilities);
                if (abilities.count_ones(), abilities) < (fewest.count_ones(), *fewest) {
                    *fewest = abilities;
                }
            }
        }
        for ((room, entrance), abilities) in trapped {
            problems.push(Problem::Trap {
                room: self.title(room),
                entrance: self.entrance_name(&self.rooms[room].entrances[entrance]),
                holding: abilities_of(abilities),
            });
        }
    }

    fn entrance_name(&self, entrance: &Entrance) -> String {
        match entrance {
            Entrance::Spawn => "the room spawn".to_owned(),
            Entrance::Arrival { room, exit } => {
                let from = self.rooms.iter().find(|candidate| candidate.id == *room);
                let from =
                    from.map_or_else(|| room_title(*room, None), |found| found.title.clone());
                format!("the arrival from {from} ({})", exit.describe())
            }
        }
    }

    /// Gates whose entrance the room does not have (a room's exits moved or deleted elsewhere).
    fn orphan_gates(&self, problems: &mut Vec<Problem>) {
        for dungeon in &self.rooms {
            for gate in &dungeon.layout.gates {
                if !dungeon.entrances.contains(&gate.from) {
                    problems.push(Problem::OrphanGate {
                        room: dungeon.title.clone(),
                        from: self.entrance_name(&gate.from),
                        exit: gate.exit.describe(),
                    });
                }
            }
        }
    }
}
