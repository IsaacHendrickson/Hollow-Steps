//! Growing the tree of rooms for one attempt: the spine from the Start piece to the Boss piece,
//! the locked links, the rooms that make room for keys and abilities, and the side branches.
//!
//! Everything random is drawn from `Ctx::rng` streams, so an attempt is a pure function of the
//! seed and its number. Where a step has no choice left it gives up with a sentence, and the
//! attempt ends (the next one draws again).
use super::attempt::Ctx;
use super::demand::{assign, demands, Placed};
use super::pool::Port;
use super::tree::Tree;
use crate::particles::Rng;
use crate::piece::PieceRole;
use crate::room_name::room_title;

/// A finished tree with its locks and where each key and ability lies.
pub(super) struct Grown {
    pub tree: Tree,
    /// The spine links that are locked, in order: lock `i + 1` is on link `locks[i]`, between
    /// spine rooms `locks[i]` and `locks[i] + 1`.
    pub locks: Vec<usize>,
    pub placed: Vec<Placed>,
}

/// A way to join a new room: its exit `exit` to the free exit `host_exit` of `host`.
#[derive(Clone, Copy)]
struct Combo {
    host: usize,
    host_exit: usize,
    piece: usize,
    exit: usize,
}

/// What the room being joined has to leave possible.
#[derive(Clone, Copy)]
enum Next {
    /// Nothing: it may be a dead end.
    Anything,
    /// Another room joins after it, so it needs a second exit.
    Continues,
    /// The Boss piece (by index) joins after it, so it needs a second exit that pairs with one of
    /// the boss's.
    ReachesBoss(usize),
}

fn below(rng: &mut Rng, count: usize) -> usize {
    (rng.next_u64() % count.max(1) as u64) as usize
}

/// A number from `low` to `high`, both included.
fn between(rng: &mut Rng, low: usize, high: usize) -> usize {
    low + below(rng, high.saturating_sub(low) + 1)
}

/// The index in `items` (index, weight) picked with probability by weight.
fn weighted(rng: &mut Rng, items: &[(usize, u8)]) -> usize {
    let total: u64 = items.iter().map(|(_, weight)| u64::from(*weight)).sum();
    let mut roll = rng.next_u64() % total.max(1);
    for (place, (_, weight)) in items.iter().enumerate() {
        if roll < u64::from(*weight) {
            return place;
        }
        roll -= u64::from(*weight);
    }
    items.len().saturating_sub(1)
}

fn title(ctx: &Ctx, piece: usize) -> String {
    let piece = &ctx.pool.pieces[piece];
    room_title(piece.id, piece.layout.name.as_deref())
}

fn exit_count(ctx: &Ctx, piece: usize) -> usize {
    ctx.oriented[piece].layout.exits.len()
}

fn next_allows(ctx: &Ctx, piece: usize, exit: usize, next: Next) -> bool {
    let exits = &ctx.oriented[piece].layout.exits;
    match next {
        Next::Anything => true,
        Next::Continues => exits.len() >= 2,
        Next::ReachesBoss(boss) => {
            let boss_exits = &ctx.oriented[boss].layout.exits;
            (0..exits.len())
                .filter(|other| *other != exit)
                .any(|other| {
                    let port = Port::of(&exits[other]);
                    boss_exits.iter().any(|y| port.pairs_with(Port::of(y)))
                })
        }
    }
}

/// Every way to join one of `pieces` (not yet in the tree) to a free exit of one of `hosts`. A
/// piece whose gates all cross outward from one port is joined at that port when it can be, so
/// the gate stands in the way of the way on, not of the way back.
fn combos(ctx: &Ctx, tree: &Tree, hosts: &[usize], pieces: &[usize], next: Next) -> Vec<Combo> {
    let mut found = Vec::new();
    for &piece in pieces.iter().filter(|piece| !tree.uses(**piece)) {
        let exits = &ctx.oriented[piece].layout.exits;
        let mut mine = Vec::new();
        for &host in hosts {
            let host_exits = &ctx.oriented[tree.nodes[host].piece].layout.exits;
            for host_exit in tree.free_exits(host) {
                let side = Port::of(&host_exits[host_exit]);
                for (exit, placement) in exits.iter().enumerate() {
                    if side.pairs_with(Port::of(placement)) && next_allows(ctx, piece, exit, next) {
                        mine.push(Combo {
                            host,
                            host_exit,
                            piece,
                            exit,
                        });
                    }
                }
            }
        }
        let ports = ctx.oriented[piece].gate_ports();
        if mine.iter().any(|combo| ports.contains(&combo.exit)) {
            mine.retain(|combo| ports.contains(&combo.exit));
        }
        found.extend(mine);
    }
    found
}

/// One of `options` (grouped by piece): a piece by weight, then one of its ways uniformly.
fn choose(ctx: &Ctx, rng: &mut Rng, options: &[Combo]) -> Option<Combo> {
    let mut pieces: Vec<(usize, u8)> = Vec::new();
    for option in options {
        if pieces
            .last()
            .is_none_or(|(piece, _)| *piece != option.piece)
        {
            pieces.push((option.piece, ctx.pool.pieces[option.piece].weight));
        }
    }
    if pieces.is_empty() {
        return None;
    }
    let piece = pieces[weighted(rng, &pieces)].0;
    let mine: Vec<&Combo> = options.iter().filter(|o| o.piece == piece).collect();
    Some(*mine[below(rng, mine.len())])
}

fn attach(ctx: &Ctx, tree: &mut Tree, combo: Combo) -> usize {
    let exits = exit_count(ctx, combo.piece);
    tree.attach(combo.host, combo.host_exit, combo.piece, combo.exit, exits)
}

/// Whether every key and ability can be placed in this tree.
fn feasible(ctx: &Ctx, tree: &Tree, locks: &[usize]) -> bool {
    let wanted = demands(ctx, tree, locks);
    assign(ctx, tree, &wanted, &mut ctx.rng("assign", 0)).is_ok()
}

/// The spine rooms but the boss.
fn hosts(tree: &Tree) -> Vec<usize> {
    tree.spine[..tree.spine.len() - 1].to_vec()
}

/// Builds the rooms of one attempt.
pub(super) fn grow(ctx: &Ctx) -> Result<Grown, Vec<String>> {
    let limits = ctx.limits;
    let middles = ctx.pool.with_role(PieceRole::Middle);
    let mut shape = ctx.rng("shape", 0);
    let spine_len = between(&mut shape, limits.spine.0, limits.spine.1);
    let total = between(&mut shape, limits.rooms.0.max(spine_len), limits.rooms.1);
    let lock_count = between(&mut shape, limits.keys.0, limits.keys.1.min(spine_len - 1));

    let weights = |role| -> Vec<(usize, u8)> {
        ctx.pool
            .with_role(role)
            .into_iter()
            .map(|piece| (piece, ctx.pool.pieces[piece].weight))
            .collect()
    };
    let starts = weights(PieceRole::Start);
    let bosses = weights(PieceRole::Boss);
    let start = starts[weighted(&mut ctx.rng("start", 0), &starts)].0;
    let boss = bosses[weighted(&mut ctx.rng("boss", 0), &bosses)].0;

    // The spine: the start, middles each joined to the newest room, then the boss.
    let mut tree = Tree::new(start, exit_count(ctx, start));
    for step in 0..spine_len - 2 {
        let host = tree.boss();
        let next = if step + 2 == spine_len - 1 {
            Next::ReachesBoss(boss)
        } else {
            Next::Continues
        };
        let options = combos(ctx, &tree, &[host], &middles, next);
        let pick = choose(ctx, &mut ctx.rng("spine", step as u64), &options).ok_or_else(|| {
            let host = tree.nodes[host].piece;
            vec![format!(
                "no piece can follow {} on the spine",
                title(ctx, host)
            )]
        })?;
        let node = attach(ctx, &mut tree, pick);
        tree.spine.push(node);
    }
    let host = tree.boss();
    let options = combos(ctx, &tree, &[host], &[boss], Next::Anything);
    let pick = choose(ctx, &mut ctx.rng("boss-link", 0), &options).ok_or_else(|| {
        let host = tree.nodes[host].piece;
        vec![format!(
            "the boss piece {} has no exit that pairs with a free exit of {}",
            title(ctx, boss),
            title(ctx, host)
        )]
    })?;
    let node = attach(ctx, &mut tree, pick);
    tree.spine.push(node);

    // The locks: some of the spine's links, in spine order.
    let mut links: Vec<usize> = (0..spine_len - 1).collect();
    let mut rng = ctx.rng("locks", 0);
    for place in (1..links.len()).rev() {
        links.swap(place, below(&mut rng, place + 1));
    }
    let mut locks: Vec<usize> = links.into_iter().take(lock_count).collect();
    locks.sort_unstable();

    // Rooms for what has no slot yet: a room with a fitting slot, joined where it can be reached.
    let mut added = 0;
    loop {
        let wanted = demands(ctx, &tree, &locks);
        let Err(unmatched) = assign(ctx, &tree, &wanted, &mut ctx.rng("assign", 0)) else {
            break;
        };
        let demand = &wanted[unmatched[0]];
        let name = demand.want.describe();
        if tree.len() >= limits.rooms.1 {
            return Err(vec![format!("no room is left to hold {name}")]);
        }
        let reachable: Vec<usize> = hosts(&tree)
            .into_iter()
            .filter(|room| demand.region[*room])
            .collect();
        let holders: Vec<usize> = middles
            .iter()
            .copied()
            .filter(|piece| ctx.oriented[*piece].has_slot_for(demand.want.is_key()))
            .collect();
        let options = combos(ctx, &tree, &reachable, &holders, Next::Anything);
        let pick = choose(ctx, &mut ctx.rng("support", added), &options).ok_or_else(|| {
            vec![format!(
                "no room with a slot for {name} can be joined before it"
            )]
        })?;
        attach(ctx, &mut tree, pick);
        added += 1;
    }

    // Side branches of one to three rooms, until the room count is reached.
    let mut tries = 0u64;
    while tree.len() < total && tries < 4 * total as u64 {
        tries += 1;
        let mut rng = ctx.rng("fill", tries);
        let open: Vec<usize> = hosts(&tree)
            .into_iter()
            .filter(|room| !tree.free_exits(*room).is_empty())
            .collect();
        if open.is_empty() {
            break;
        }
        let mut at = open[below(&mut rng, open.len())];
        let length = (1 + below(&mut rng, 3)).min(total - tree.len());
        let mut refused: Vec<usize> = Vec::new();
        for step in 0..length {
            let next = if step + 1 < length {
                Next::Continues
            } else {
                Next::Anything
            };
            let mut joined = None;
            loop {
                let pieces: Vec<usize> = middles
                    .iter()
                    .copied()
                    .filter(|piece| !refused.contains(piece))
                    .collect();
                let options = combos(ctx, &tree, &[at], &pieces, next);
                let Some(pick) = choose(ctx, &mut rng, &options) else {
                    break;
                };
                let node = attach(ctx, &mut tree, pick);
                if feasible(ctx, &tree, &locks) {
                    joined = Some(node);
                    break;
                }
                // A gate that leaves an ability with no place to lie in: take the room back.
                tree.detach_last();
                refused.push(pick.piece);
            }
            match joined {
                Some(node) => at = node,
                None => break,
            }
        }
    }
    if tree.len() < limits.rooms.0 {
        return Err(vec![format!(
            "only {} rooms could be joined, and rooms.min is {}",
            tree.len(),
            limits.rooms.0
        )]);
    }

    let wanted = demands(ctx, &tree, &locks);
    let placed = assign(ctx, &tree, &wanted, &mut ctx.rng("place", 0)).map_err(|missing| {
        missing
            .into_iter()
            .map(|demand| format!("no slot is left for {}", wanted[demand].want.describe()))
            .collect::<Vec<_>>()
    })?;
    Ok(Grown {
        tree,
        locks,
        placed,
    })
}
