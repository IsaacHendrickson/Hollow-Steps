//! What has to be placed in slots, where each thing may go, and one way to place them all.
//!
//! A **key** `i` may lie in any room on the start side of lock `i`: every room but the ones
//! behind that lock (rooms behind earlier locks are fine, their keys come first). An **ability**
//! a gate needs may lie in any room the player reaches from the start without crossing a lock or
//! a gate: a zone with nothing in front of it, so no ability waits on a key or on another
//! ability. The boss room's slots are never used. Placing is a matching: each thing in its own
//! slot, which `assign` finds, or says which thing has none.
use super::attempt::Ctx;
use super::pool::{ability_bit, slot_fits, Entry};
use super::tree::Tree;
use crate::ability::Ability;
use crate::item::Item;
use crate::particles::Rng;

/// One thing to place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Want {
    Key(u8),
    Ability(Ability),
}

impl Want {
    pub fn is_key(self) -> bool {
        matches!(self, Self::Key(_))
    }

    pub fn describe(self) -> String {
        match self {
            Self::Key(id) => format!("Key {id}"),
            Self::Ability(ability) => ability.name().to_owned(),
        }
    }

    /// The item that is placed: the key itself, or the pickup that grants the ability.
    pub fn item(self) -> Option<Item> {
        match self {
            Self::Key(id) => Some(Item::Key(id)),
            Self::Ability(ability) => Item::ALL
                .iter()
                .copied()
                .find(|item| item.grants() == Some(ability)),
        }
    }
}

/// A thing to place and the rooms it may go in (one flag per room of the tree).
pub(super) struct Demand {
    pub want: Want,
    pub region: Vec<bool>,
}

/// A thing placed: the room and the index of its slot in the room's piece.
#[derive(Clone, Copy, Debug)]
pub(super) struct Placed {
    pub want: Want,
    pub node: usize,
    pub slot: usize,
}

/// Whether the spine link `link` (between spine rooms `link` and `link + 1`) is locked.
fn is_locked(tree: &Tree, locks: &[usize], from: usize, to: usize) -> bool {
    tree.spine
        .windows(2)
        .enumerate()
        .any(|(link, pair)| locks.contains(&link) && pair[0] == from && pair[1] == to)
}

/// The abilities the gates of the rooms in the tree need, those that survive pairing: a gate on
/// an exit that is joined, from the spawn or from a port that is joined.
fn gates_needing(ctx: &Ctx, tree: &Tree) -> u8 {
    let mut needed = 0;
    for node in &tree.nodes {
        for gate in &ctx.oriented[node.piece].gates {
            let exit_joined = gate.exit.is_some_and(|exit| node.links[exit].is_some());
            let from_joined = match gate.from {
                Entry::Spawn => true,
                Entry::Port(port) => node.links[port].is_some(),
                Entry::Lost => false,
            };
            if exit_joined && from_joined {
                needed |= gate.needs;
            }
        }
    }
    needed
}

/// Whether a piece in the tree already holds a pickup of `ability`: it counts as placed.
fn lies_in_a_piece(ctx: &Ctx, tree: &Tree, ability: Ability) -> bool {
    tree.nodes.iter().any(|node| {
        ctx.oriented[node.piece]
            .layout
            .items
            .iter()
            .any(|placed| placed.item.grants() == Some(ability))
    })
}

/// The rooms reached from the start without crossing a lock or a gate. Going down from `room`
/// through its exit `exit` is stopped by a lock on that spine link, or by a gate of `room` that
/// goes from the exit leading up to that exit.
fn free_zone(ctx: &Ctx, tree: &Tree, locks: &[usize]) -> Vec<bool> {
    let mut zone = vec![false; tree.len()];
    zone[0] = true;
    let mut pending = vec![0];
    while let Some(room) = pending.pop() {
        let up = tree.parent_exit(room);
        let gates = &ctx.oriented[tree.nodes[room].piece].gates;
        for (exit, child) in tree.children(room) {
            let gated = up.is_some_and(|up| {
                gates.iter().any(|gate| {
                    gate.from == Entry::Port(up) && gate.exit == Some(exit) && gate.needs != 0
                })
            });
            if zone[child] || gated || is_locked(tree, locks, room, child) {
                continue;
            }
            zone[child] = true;
            pending.push(child);
        }
    }
    zone
}

/// Everything to place for this tree: each key, then each ability a gate needs that no piece
/// already holds. `locks` are the spine links that are locked, in order: lock `i + 1` is on
/// link `locks[i]`.
pub(super) fn demands(ctx: &Ctx, tree: &Tree, locks: &[usize]) -> Vec<Demand> {
    let mut found = Vec::new();
    for (index, link) in locks.iter().enumerate() {
        let behind = tree.subtree(tree.spine[link + 1]);
        found.push(Demand {
            want: Want::Key(u8::try_from(index + 1).unwrap_or(u8::MAX)),
            region: behind.iter().map(|inside| !inside).collect(),
        });
    }
    let needed = gates_needing(ctx, tree);
    if needed != 0 {
        let zone = free_zone(ctx, tree, locks);
        for ability in Ability::ALL.iter().copied() {
            if needed & ability_bit(ability) != 0 && !lies_in_a_piece(ctx, tree, ability) {
                found.push(Demand {
                    want: Want::Ability(ability),
                    region: zone.clone(),
                });
            }
        }
    }
    found
}

/// Puts every demand in a slot of its own, or returns the demands (by index) left without one.
/// The slots tried for each demand are shuffled by `rng`, so the result varies with the stream
/// but whether a placing exists never does.
pub(super) fn assign(
    ctx: &Ctx,
    tree: &Tree,
    demands: &[Demand],
    rng: &mut Rng,
) -> Result<Vec<Placed>, Vec<usize>> {
    let boss = tree.boss();
    let mut slots = Vec::new();
    for (node, room) in tree
        .nodes
        .iter()
        .enumerate()
        .filter(|(node, _)| *node != boss)
    {
        for (slot, placed) in ctx.oriented[room.piece].layout.slots.iter().enumerate() {
            slots.push((node, slot, placed.kind));
        }
    }
    let candidates: Vec<Vec<usize>> = demands
        .iter()
        .map(|demand| {
            let mut fits: Vec<usize> = (0..slots.len())
                .filter(|index| {
                    let (node, _, kind) = slots[*index];
                    demand.region[node] && slot_fits(kind, demand.want.is_key())
                })
                .collect();
            for place in (1..fits.len()).rev() {
                let other = (rng.next_u64() % (place as u64 + 1)) as usize;
                fits.swap(place, other);
            }
            fits
        })
        .collect();
    let mut owner: Vec<Option<usize>> = vec![None; slots.len()];
    for demand in 0..demands.len() {
        let mut seen = vec![false; slots.len()];
        augment(demand, &candidates, &mut owner, &mut seen);
    }
    let unmatched: Vec<usize> = (0..demands.len())
        .filter(|demand| !owner.contains(&Some(*demand)))
        .collect();
    if !unmatched.is_empty() {
        return Err(unmatched);
    }
    Ok(owner
        .iter()
        .enumerate()
        .filter_map(|(index, demand)| {
            let (node, slot, _) = slots[index];
            Some(Placed {
                want: demands[(*demand)?].want,
                node,
                slot,
            })
        })
        .collect())
}

/// One step of a bipartite matching: gives `demand` a slot, moving another demand to a different
/// slot if it has to. False when `demand` cannot have one.
fn augment(
    demand: usize,
    candidates: &[Vec<usize>],
    owner: &mut [Option<usize>],
    seen: &mut [bool],
) -> bool {
    for &slot in &candidates[demand] {
        if std::mem::replace(&mut seen[slot], true) {
            continue;
        }
        let free = match owner[slot] {
            None => true,
            Some(other) => augment(other, candidates, owner, seen),
        };
        if free {
            owner[slot] = Some(demand);
            return true;
        }
    }
    false
}
