//! A room that has one of everything the mirror moves, with whole-number coordinates. The bounds
//! are `(100, 50, 800, 400)`, so a point `x` mirrors to `1000 - x`.
use crate::ability::Ability;
use crate::editor::{ExitPlacement, ItemPlacement, RoomLayout};
use crate::gates::{Entrance, ExitRef, Gate};
use crate::item::Item;
use crate::puzzle::{BarrierPlacement, BreakablePlacement, SwitchPlacement};
use crate::room::{test_rooms, RoomEdge, RoomId};
use crate::room_charms::CharmPlacement;

pub(super) const TARGET: RoomId = RoomId::First;

pub(super) fn exit(edge: RoomEdge, offset: f32, length: f32) -> ExitPlacement {
    ExitPlacement::edge_exit(edge, offset, length, TARGET, (160.0, 376.0))
}

pub(super) fn door(zone: (f32, f32, f32, f32)) -> ExitPlacement {
    ExitPlacement {
        door: Some(zone),
        ..exit(RoomEdge::Bottom, 0.0, 0.0)
    }
}

fn hazard(rect: &str, facing: &str) -> crate::hazard::HazardPlacement {
    ron::from_str(&format!(
        "(rect: {rect}, damage: 10, knockback_speed: 420.0, lift_speed: 320.0, \
         knockback_secs: 0.25, facing: {facing})"
    ))
    .unwrap()
}

pub(super) fn rich() -> RoomLayout {
    let mut layout = RoomLayout::from_room(&test_rooms()[0]);
    layout.name = Some("Rich".to_owned());
    layout.bounds = (100.0, 50.0, 800.0, 400.0);
    layout.floor_y = 430.0;
    layout.spawn = (200.0, 406.0);
    layout.terrain = vec![(500.0, 380.0, 100.0, 50.0)];
    layout.hazards = vec![
        hazard("(700.0, 414.0, 64.0, 16.0)", "Up"),
        hazard("(140.0, 200.0, 16.0, 64.0)", "Left"),
        hazard("(840.0, 200.0, 16.0, 64.0)", "Right"),
        hazard("(400.0, 60.0, 64.0, 16.0)", "Down"),
    ];
    layout.breakables = vec![BreakablePlacement {
        rect: (640.0, 330.0, 24.0, 100.0),
        hits: 2,
    }];
    layout.switches = vec![SwitchPlacement {
        rect: (250.0, 300.0, 16.0, 24.0),
        id: 1,
    }];
    layout.barriers = vec![BarrierPlacement {
        rect: (800.0, 330.0, 24.0, 100.0),
        switch: 1,
    }];
    layout.items = vec![ItemPlacement {
        item: Item::Dash,
        position: (350.0, 400.0),
    }];
    layout.charms = vec![CharmPlacement {
        charm: "ember".to_owned(),
        position: (450.0, 410.0),
    }];
    layout.exits = vec![
        exit(RoomEdge::Left, 300.0, 96.0),
        exit(RoomEdge::Right, 300.0, 96.0),
        exit(RoomEdge::Top, 200.0, 100.0),
        exit(RoomEdge::Bottom, 150.0, 80.0),
        door((400.0, 358.0, 48.0, 72.0)),
    ];
    layout.gates = vec![
        Gate {
            from: Entrance::Spawn,
            exit: ExitRef::Edge {
                edge: RoomEdge::Right,
                offset: 300.0,
            },
            needs: vec![Ability::Dash],
        },
        Gate {
            from: Entrance::Arrival {
                room: TARGET,
                exit: ExitRef::Edge {
                    edge: RoomEdge::Right,
                    offset: 100.0,
                },
            },
            exit: ExitRef::Edge {
                edge: RoomEdge::Bottom,
                offset: 150.0,
            },
            needs: vec![Ability::Dash],
        },
    ];
    layout.map_cell = Some((3, -2));
    layout
}
