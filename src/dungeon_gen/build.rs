//! Turning a grown tree into rooms: instance ids, exits paired and aimed at their neighbours,
//! gates carried, locks, keys and abilities lying in the slots, and the hub rewired.
use super::attempt::Ctx;
use super::grow::Grown;
use super::plan::PlannedRoom;
use super::pool::Entry;
use crate::editor::{ItemPlacement, RoomLayout};
use crate::exit_arrival::{arrival_error, arrival_next_to};
use crate::gates::{Entrance, ExitRef, Gate};
use crate::room::RoomId;
use crate::room_name::room_title;
use crate::run::RunConfig;

/// The rooms of the tree as they play, and the hub aimed at the start room. An error says what
/// no room can be built from (a spot where nobody can arrive).
pub(super) fn build(
    ctx: &Ctx,
    grown: &Grown,
    hub: &(RoomId, RoomLayout),
    config: &RunConfig,
) -> Result<(Vec<PlannedRoom>, RoomLayout), Vec<String>> {
    let tree = &grown.tree;
    let boss = tree.boss();
    let instance = |node: usize| -> RoomId {
        if node == 0 {
            config.start_room
        } else if node == boss {
            config.final_boss_room
        } else {
            ctx.pool.pieces[tree.nodes[node].piece].id
        }
    };
    let oriented = |node: usize| &ctx.oriented[tree.nodes[node].piece];

    // 1. Each room keeps the exits that are joined, aimed at the room they lead to, and the
    //    gates that still make sense. `moved[node][old]` is where an exit went in the list.
    let mut layouts: Vec<RoomLayout> = Vec::new();
    let mut moved: Vec<Vec<Option<usize>>> = Vec::new();
    for (index, node) in tree.nodes.iter().enumerate() {
        let source = oriented(index);
        let mut layout = source.layout.clone();
        let mut exits = Vec::new();
        let mut places = vec![None; source.layout.exits.len()];
        for (old, exit) in source.layout.exits.iter().enumerate() {
            if let Some(link) = node.links[old] {
                let mut kept = exit.clone();
                kept.target_room = instance(link.to);
                kept.locked_by = None;
                places[old] = Some(exits.len());
                exits.push(kept);
            }
        }
        layout.exits = exits;
        layout.gates = source
            .layout
            .gates
            .iter()
            .zip(&source.gates)
            .filter_map(|(gate, info)| {
                node.links[info.exit?]?;
                let from = match info.from {
                    Entry::Spawn => gate.from,
                    Entry::Lost => return None,
                    Entry::Port(port) => {
                        let link = node.links[port]?;
                        let neighbour = &oriented(link.to).layout.exits[link.to_exit];
                        Entrance::Arrival {
                            room: instance(link.to),
                            exit: ExitRef::of(neighbour),
                        }
                    }
                };
                Some(Gate {
                    from,
                    exit: gate.exit,
                    needs: gate.needs.clone(),
                })
            })
            .collect();
        // A planned room is a room of a dungeon, not a piece of a pool.
        layout.slots.clear();
        layout.piece = None;
        layout.map_cell = None;
        layouts.push(layout);
        moved.push(places);
    }

    // 2. Where each exit lands: next to the exit of the room it leads to.
    let mut arrivals = Vec::new();
    for (index, node) in tree.nodes.iter().enumerate() {
        for (old, link) in node.links.iter().enumerate() {
            let (Some(link), Some(here)) = (link, moved[index][old]) else {
                continue;
            };
            let there = moved[link.to][link.to_exit].unwrap_or(0);
            let spot = arrival_next_to(&layouts[link.to], there).map_err(|why| {
                let room = &layouts[link.to];
                vec![format!(
                    "{}: {why}",
                    room_title(instance(link.to), room.name.as_deref())
                )]
            })?;
            arrivals.push((index, here, spot));
        }
    }
    for (index, exit, spot) in arrivals {
        layouts[index].exits[exit].spawn = spot.into();
    }

    // 3. The locks, on the way away from the start.
    for (number, link) in grown.locks.iter().enumerate() {
        let (from, to) = (tree.spine[*link], tree.spine[*link + 1]);
        let exit = tree
            .exit_toward(from, to)
            .and_then(|old| moved[from][old])
            .ok_or_else(|| vec!["a locked link has no exit".to_owned()])?;
        layouts[from].exits[exit].locked_by = u8::try_from(number + 1).ok();
    }

    // 4. Keys and abilities where their slots were.
    for placed in &grown.placed {
        let slot = oriented(placed.node).layout.slots[placed.slot];
        let item = placed
            .want
            .item()
            .ok_or_else(|| vec![format!("no pickup grants {}", placed.want.describe())])?;
        layouts[placed.node].items.push(ItemPlacement {
            item,
            position: slot.position,
        });
    }

    // 5. The hub: each exit to the start room arrives at the start room's spawn.
    let start = &layouts[0];
    if let Some(why) = arrival_error(start, start.spawn.into()) {
        return Err(vec![format!(
            "{}: the spawn is not a place to arrive ({why})",
            room_title(instance(0), start.name.as_deref())
        )]);
    }
    let mut hub_layout = hub.1.clone();
    for exit in &mut hub_layout.exits {
        if exit.target_room == config.start_room {
            exit.spawn = start.spawn;
        }
    }

    let rooms = layouts
        .into_iter()
        .enumerate()
        .map(|(index, layout)| PlannedRoom {
            instance: instance(index),
            piece: ctx.pool.pieces[tree.nodes[index].piece].id,
            mirrored: oriented(index).mirrored,
            layout,
        })
        .collect();
    Ok((rooms, hub_layout))
}
