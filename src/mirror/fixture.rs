//! A small dungeon built in code, for the mirror's tests and for the flip's tests in play. It has
//! a hub; a start room with a key, the Dash, a hazard, a door and edge exits; a second room with
//! a locked exit, a Dash gate, a switch and a barrier, and a top exit; a boss room; a tower
//! reached by that top exit (a bottom exit back, with its own Dash gate); and a closet behind a
//! door. Every arrival is a free spot beside the exit it comes from (`arrival_next_to`), so the
//! rooms are valid as they stand. Rooms have different bounds, so a mirror is never a no-op.
//!
//! `dungeon(false)` is the same dungeon made bad on purpose: no key and no Dash anywhere.
use crate::ability::Ability;
use crate::editor::{ExitPlacement, ItemPlacement, RoomLayout};
use crate::exit_arrival::arrival_next_to;
use crate::gates::{Entrance, ExitRef, Gate};
use crate::hazard::HazardPlacement;
use crate::item::{Item, ITEM_SIZE};
use crate::puzzle::{BarrierPlacement, SwitchPlacement};
use crate::room::{test_rooms, RoomEdge, RoomEnemies, RoomId};
use crate::run::RunConfig;

pub(crate) const HUB: RoomId = RoomId::Custom(0);
pub(crate) const START: RoomId = RoomId::First;
pub(crate) const SECOND: RoomId = RoomId::Second;
pub(crate) const BOSS: RoomId = RoomId::Boss;
pub(crate) const TOWER: RoomId = RoomId::Custom(1);
pub(crate) const CLOSET: RoomId = RoomId::Custom(2);

/// The loop's rules for the fixture dungeon.
pub(crate) fn run_config() -> RunConfig {
    RunConfig::from_ron("(hub_room: Custom(0), start_room: First, final_boss_room: Boss)").unwrap()
}

/// An empty room with these bounds and floor, and a spawn on the floor 100 in.
fn blank(bounds: (f32, f32, f32, f32), floor_y: f32) -> RoomLayout {
    let mut layout = RoomLayout::from_room(&test_rooms()[0]);
    layout.name = None;
    layout.bounds = bounds;
    layout.floor_y = floor_y;
    layout.spawn = (bounds.0 + 100.0, floor_y - 24.0);
    layout.items.clear();
    layout.charms.clear();
    layout.exits.clear();
    layout.terrain.clear();
    layout.hazards.clear();
    layout.clear_puzzles();
    layout.gates.clear();
    layout.map_cell = None;
    layout
}

/// An exit on an edge, 96 long, whose arrival `fix_spawns` fills in.
fn edge(edge: RoomEdge, offset: f32, to: RoomId) -> ExitPlacement {
    ExitPlacement::edge_exit(edge, offset, 96.0, to, (0.0, 0.0))
}

fn door(zone: (f32, f32, f32, f32), to: RoomId) -> ExitPlacement {
    ExitPlacement {
        door: Some(zone),
        ..ExitPlacement::edge_exit(RoomEdge::Bottom, 0.0, 0.0, to, (0.0, 0.0))
    }
}

/// An item standing on the floor at `x`.
fn on_floor(item: Item, x: f32, floor_y: f32) -> ItemPlacement {
    ItemPlacement {
        item,
        position: (x, floor_y - ITEM_SIZE.y / 2.0),
    }
}

/// Every exit's arrival: the free spot beside the exit of the target room that leads back here.
fn fix_spawns(rooms: &mut [(RoomId, RoomLayout)]) {
    let snapshot = rooms.to_vec();
    for (id, layout) in rooms.iter_mut() {
        for exit in &mut layout.exits {
            let (_, target) = snapshot
                .iter()
                .find(|(t, _)| *t == exit.target_room)
                .unwrap();
            let back = target
                .exits
                .iter()
                .position(|e| e.target_room == *id)
                .unwrap();
            exit.spawn = arrival_next_to(target, back).unwrap().into();
        }
    }
}

fn dash_gate(from: Entrance, exit: &ExitPlacement) -> Gate {
    Gate {
        from,
        exit: ExitRef::of(exit),
        needs: vec![Ability::Dash],
    }
}

/// The layouts of the fixture dungeon, hub first.
pub(crate) fn layouts(good: bool) -> Vec<(RoomId, RoomLayout)> {
    let mut hub = blank((0.0, 0.0, 960.0, 460.0), 460.0);
    hub.exits = vec![edge(RoomEdge::Right, 364.0, START)];

    let mut start = blank((0.0, 0.0, 960.0, 460.0), 460.0);
    start.exits = vec![
        edge(RoomEdge::Left, 364.0, HUB),
        edge(RoomEdge::Right, 364.0, SECOND),
        door((700.0, 388.0, 48.0, 72.0), CLOSET),
    ];
    start.terrain = vec![(200.0, 380.0, 160.0, 16.0), (560.0, 400.0, 100.0, 60.0)];
    start.hazards = [
        "(rect: (380.0, 444.0, 64.0, 16.0), damage: 10, knockback_speed: 420.0, \
         lift_speed: 320.0, knockback_secs: 0.25, facing: Up)",
        "(rect: (880.0, 280.0, 16.0, 64.0), damage: 10, knockback_speed: 420.0, \
         lift_speed: 320.0, knockback_secs: 0.25, facing: Left)",
    ]
    .iter()
    .map(|text| ron::from_str::<HazardPlacement>(text).unwrap())
    .collect();
    if good {
        start.items = vec![
            on_floor(Item::Key(1), 480.0, 460.0),
            on_floor(Item::Dash, 300.0, 460.0),
        ];
    }

    let mut second = blank((80.0, 40.0, 800.0, 360.0), 400.0);
    second.spawn = (180.0, 376.0);
    let locked = ExitPlacement {
        locked_by: Some(1),
        ..edge(RoomEdge::Right, 264.0, BOSS)
    };
    second.exits = vec![
        edge(RoomEdge::Left, 264.0, START),
        locked.clone(),
        edge(RoomEdge::Top, 300.0, TOWER),
    ];
    second.terrain = vec![(400.0, 340.0, 80.0, 60.0)];
    second.switches = vec![SwitchPlacement {
        rect: (300.0, 300.0, 16.0, 24.0),
        id: 1,
    }];
    second.barriers = vec![BarrierPlacement {
        rect: (600.0, 300.0, 24.0, 100.0),
        switch: 1,
    }];
    // From the start room's right exit, the locked exit also needs the Dash.
    let from_start = Entrance::Arrival {
        room: START,
        exit: ExitRef::Edge {
            edge: RoomEdge::Right,
            offset: 364.0,
        },
    };
    second.gates = vec![dash_gate(from_start, &locked)];

    let mut boss = blank((0.0, 0.0, 960.0, 460.0), 460.0);
    boss.exits = vec![edge(RoomEdge::Left, 364.0, SECOND)];

    let mut tower = blank((200.0, 100.0, 640.0, 400.0), 480.0);
    tower.spawn = (300.0, 456.0);
    let down = edge(RoomEdge::Bottom, 300.0, SECOND);
    tower.exits = vec![down.clone()];
    tower.terrain = vec![(260.0, 400.0, 100.0, 16.0)];
    let from_second = Entrance::Arrival {
        room: SECOND,
        exit: ExitRef::Edge {
            edge: RoomEdge::Top,
            offset: 300.0,
        },
    };
    tower.gates = vec![dash_gate(from_second, &down)];

    let mut closet = blank((60.0, 20.0, 400.0, 300.0), 300.0);
    closet.exits = vec![door((120.0, 228.0, 48.0, 72.0), START)];

    let mut rooms = vec![
        (HUB, hub),
        (START, start),
        (SECOND, second),
        (BOSS, boss),
        (TOWER, tower),
        (CLOSET, closet),
    ];
    fix_spawns(&mut rooms);
    rooms
}

/// The fixture dungeon as room documents (no enemies, no bosses), hub first.
pub(crate) fn dungeon(good: bool) -> Vec<(RoomId, RoomEnemies)> {
    layouts(good)
        .into_iter()
        .map(|(id, layout)| {
            let document = RoomEnemies {
                enemies: Vec::new(),
                boss: None,
                layout: Some(layout),
            };
            (id, document)
        })
        .collect()
}

/// The layouts of `documents`, the way the checker and the editor take them.
pub(crate) fn catalog(documents: &[(RoomId, RoomEnemies)]) -> Vec<(RoomId, RoomLayout)> {
    documents
        .iter()
        .filter_map(|(id, document)| Some((*id, document.layout.clone()?)))
        .collect()
}
