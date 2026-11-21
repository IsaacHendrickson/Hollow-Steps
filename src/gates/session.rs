//! The editor's view of the gates of the room being edited: the entrances a crossing can start
//! from, and the gates that name an entrance the room no longer has.
use macroquad::math::Vec2;

use super::{Entrance, ExitRef};
use crate::editor_session::EditorSession;

/// One entrance of the room being edited, as the editor lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct EntranceInfo {
    pub entrance: Entrance,
    /// What the editor shows: `Room spawn` or `Arrival from Second (right edge exit at 40)`.
    pub label: String,
    /// Where the player appears.
    pub position: Vec2,
}

/// A gate whose entrance does not exist any more (see `docs/gates.md`).
#[derive(Clone, Debug, PartialEq)]
pub struct OrphanGate {
    /// Its place in `layout.gates`, for `RoomDraft::remove_gate`.
    pub index: usize,
    /// What the panel shows as the problem.
    pub text: String,
}

impl EditorSession {
    /// Every entrance of the room being edited: its spawn, then the arrival of each exit of
    /// another room that leads here (unsaved drafts included), in the order the rooms list them.
    pub fn gate_entrances(&self) -> Vec<EntranceInfo> {
        let mut found = vec![EntranceInfo {
            entrance: Entrance::Spawn,
            label: "Room spawn".to_owned(),
            position: self.draft.layout.spawn.into(),
        }];
        let catalog = self.catalog();
        for arrival in self.inbound_arrivals() {
            let Some(placement) = catalog
                .iter()
                .find(|(id, _)| *id == arrival.room)
                .and_then(|(_, layout)| layout.exits.get(arrival.exit))
            else {
                continue;
            };
            let exit = ExitRef::of(placement);
            found.push(EntranceInfo {
                entrance: Entrance::Arrival {
                    room: arrival.room,
                    exit,
                },
                label: format!("Arrival from {} ({})", arrival.title, exit.describe()),
                position: arrival.position,
            });
        }
        found
    }

    /// The gates that come from an entrance this room does not have: the other room's exit was
    /// moved, re-linked elsewhere or deleted. They are reported, never enforced, and kept until
    /// the author removes them.
    pub fn orphan_gates(&self) -> Vec<OrphanGate> {
        let entrances = self.gate_entrances();
        self.draft
            .layout
            .gates
            .iter()
            .enumerate()
            .filter(|(_, gate)| !entrances.iter().any(|known| known.entrance == gate.from))
            .map(|(index, gate)| {
                let from = match gate.from {
                    Entrance::Spawn => "the room spawn".to_owned(),
                    Entrance::Arrival { room, exit } => {
                        format!(
                            "the arrival from {} ({})",
                            self.title_of(room),
                            exit.describe()
                        )
                    }
                };
                let needs: Vec<_> = gate.needs.iter().map(|a| a.name()).collect();
                OrphanGate {
                    index,
                    text: format!(
                        "Gate from {from} to the {} needs {}, but that entrance no longer exists",
                        gate.exit.describe(),
                        needs.join(" and ")
                    ),
                }
            })
            .collect()
    }
}
