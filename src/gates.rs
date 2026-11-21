//! Gates: marks that say crossing a room, from one entrance to one exit, needs abilities. The
//! game never reads them; the dungeon checker and the room editor do (`docs/gates.md`).
//!
//! An exit is named by where it stands (`ExitRef`, never its place in the list), an entrance by
//! where it comes from (`Entrance`, never where it lands), so editing either list or moving an
//! arrival point cannot detach a gate.
use serde::{Deserialize, Serialize};

use crate::ability::Ability;
use crate::editor::{ExitPlacement, RoomDraft, RoomLayout};
use crate::room::{RoomEdge, RoomId};

pub mod session;

/// One exit of a room, as its file places it: `(edge, offset)` for an edge exit, the rectangle
/// for a door (a door has no edge or offset of its own). The same identity `OpenedExits` uses.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ExitRef {
    Edge {
        edge: RoomEdge,
        offset: f32,
    },
    Door {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
}

impl ExitRef {
    pub fn of(exit: &ExitPlacement) -> Self {
        match exit.door {
            Some((x, y, width, height)) => Self::Door {
                x,
                y,
                width,
                height,
            },
            None => Self::Edge {
                edge: exit.edge,
                offset: exit.offset,
            },
        }
    }

    /// For messages: `right edge exit at 120` or `door at (600, 300)`.
    pub fn describe(&self) -> String {
        match *self {
            Self::Edge { edge, offset } => {
                format!(
                    "{} edge exit at {offset:.0}",
                    format!("{edge:?}").to_lowercase()
                )
            }
            Self::Door { x, y, .. } => format!("door at ({x:.0}, {y:.0})"),
        }
    }
}

/// Where the player comes into a room from: its spawn, or the arrival of another room's exit
/// that leads here. Named by the source, so moving the arrival point changes nothing here.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Entrance {
    Spawn,
    Arrival { room: RoomId, exit: ExitRef },
}

/// Going from `from` to `exit` needs every ability in `needs`. Absent means free.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Gate {
    pub from: Entrance,
    pub exit: ExitRef,
    pub needs: Vec<Ability>,
}

impl RoomLayout {
    /// The abilities that going from `from` to `exit` needs; empty when it is free.
    pub fn gate_needs(&self, from: &Entrance, exit: &ExitRef) -> &[Ability] {
        self.gates
            .iter()
            .find(|gate| gate.from == *from && gate.exit == *exit)
            .map_or(&[], |gate| &gate.needs)
    }

    fn has_exit(&self, exit: &ExitRef) -> bool {
        self.exits
            .iter()
            .any(|placement| ExitRef::of(placement) == *exit)
    }

    /// Every gate has abilities (each once), names an exit the room has, and no two share
    /// their entrance and exit. An entrance that does not exist is not checked here: it depends
    /// on other rooms, and is reported as a problem instead (`EditorSession::orphan_gates`).
    pub fn validate_gates(&self) -> Result<(), String> {
        for (index, gate) in self.gates.iter().enumerate() {
            let number = index + 1;
            if gate.needs.is_empty() {
                return Err(format!("gate {number} needs no ability; remove it"));
            }
            for (at, ability) in gate.needs.iter().enumerate() {
                if gate.needs[..at].contains(ability) {
                    return Err(format!("gate {number} lists {} twice", ability.name()));
                }
            }
            if !self.has_exit(&gate.exit) {
                return Err(format!(
                    "gate {number} names a {} that the room does not have",
                    gate.exit.describe()
                ));
            }
            if self.gates[..index]
                .iter()
                .any(|earlier| earlier.from == gate.from && earlier.exit == gate.exit)
            {
                return Err(format!(
                    "gate {number} repeats an earlier gate for the {}",
                    gate.exit.describe()
                ));
            }
        }
        Ok(())
    }

    /// Keeps the gates in step after an edit that changed the exits from `before`: a gate follows
    /// an exit that moved (same list length, the exit at the same place in the list changed
    /// where it stands), and a gate on an exit that is gone is dropped.
    pub(crate) fn follow_exits(&mut self, before: &[ExitPlacement]) {
        if before.len() == self.exits.len() {
            let moved: Vec<(ExitRef, ExitRef)> = before
                .iter()
                .zip(&self.exits)
                .map(|(old, new)| (ExitRef::of(old), ExitRef::of(new)))
                .filter(|(old, new)| old != new)
                .collect();
            for gate in &mut self.gates {
                if let Some((_, new)) = moved.iter().find(|(old, _)| *old == gate.exit) {
                    gate.exit = *new;
                }
            }
        }
        self.drop_gates_without_exit();
    }

    /// Drops the gates on an exit the room no longer has (it was deleted).
    pub(crate) fn drop_gates_without_exit(&mut self) {
        let exits = &self.exits;
        self.gates.retain(|gate| {
            exits
                .iter()
                .any(|placement| ExitRef::of(placement) == gate.exit)
        });
    }
}

impl RoomDraft {
    /// Marks (or unmarks) that going from `from` to exit `exit` needs `ability`. The gate is
    /// made when the first ability is ticked and removed when the last one is cleared, so a free
    /// crossing never leaves an empty gate behind. Ticking what is ticked changes nothing.
    pub fn set_gate(
        &mut self,
        from: Entrance,
        exit: usize,
        ability: Ability,
        needed: bool,
    ) -> Result<(), String> {
        let to = ExitRef::of(self.layout.exits.get(exit).ok_or("missing exit")?);
        let gates = &mut self.layout.gates;
        let found = gates
            .iter()
            .position(|gate| gate.from == from && gate.exit == to);
        match (found, needed) {
            (Some(at), true) => {
                if !gates[at].needs.contains(&ability) {
                    gates[at].needs.push(ability);
                    gates[at]
                        .needs
                        .sort_by_key(|a| Ability::ALL.iter().position(|known| known == a));
                }
            }
            (Some(at), false) => {
                gates[at].needs.retain(|a| *a != ability);
                if gates[at].needs.is_empty() {
                    gates.remove(at);
                }
            }
            (None, true) => gates.push(Gate {
                from,
                exit: to,
                needs: vec![ability],
            }),
            (None, false) => {}
        }
        Ok(())
    }

    /// Deletes gate `index` (the Remove button of an orphan).
    pub fn remove_gate(&mut self, index: usize) -> Result<(), String> {
        if index >= self.layout.gates.len() {
            return Err("missing gate".into());
        }
        self.layout.gates.remove(index);
        Ok(())
    }
}

#[cfg(test)]
mod tests;
