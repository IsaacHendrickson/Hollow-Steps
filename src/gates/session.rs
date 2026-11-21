//! The editor's view of the gates of the room being edited: the entrances a crossing can start
//! from, and the gates that name an entrance the room no longer has.
use macroquad::math::Vec2;

use super::{entrances_of, Entrance};
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
    /// Every entrance of the room being edited (`entrances_of`, over every room as the editor
    /// has it, unsaved drafts included): its spawn, then the arrival of each exit that leads here.
    pub fn gate_entrances(&self) -> Vec<EntranceInfo> {
        entrances_of(self.id, &self.catalog())
            .into_iter()
            .map(|(entrance, position)| {
                let label = match entrance {
                    Entrance::Spawn => "Room spawn".to_owned(),
                    Entrance::Arrival { room, exit } if room == self.id => {
                        format!("Arrival from this room ({})", exit.describe())
                    }
                    Entrance::Arrival { room, exit } => {
                        format!("Arrival from {} ({})", self.title_of(room), exit.describe())
                    }
                };
                EntranceInfo {
                    entrance,
                    label,
                    position,
                }
            })
            .collect()
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
