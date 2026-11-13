//! What the room editor can have selected on the canvas.
use crate::room::RoomId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    Enemy(usize),
    Boss,
    Item(usize),
    PatrolLeft(usize),
    PatrolRight(usize),
    Spawn,
    Exit(usize),
    /// The arrival point of exit `i` of the room being edited, drawn on the destination
    /// room's own view ("Arrival spawn").
    Destination(usize),
    /// The arrival point of exit `exit` of another room, `room`, whose exit leads into the
    /// room being edited (see `editor_inbound`).
    Inbound {
        room: RoomId,
        exit: usize,
    },
    /// While exits are being linked: where the player will arrive in the room being linked
    /// to, next to its exit `exit` (see `exit_link`).
    LinkArrival(usize),
}

impl Selection {
    /// Moving it edits the room being edited. The inbound and link arrivals instead edit
    /// another room's draft or a link not made yet, so they leave this room's unsaved flag
    /// and undo history alone.
    pub fn edits_current_room(self) -> bool {
        !matches!(self, Selection::Inbound { .. } | Selection::LinkArrival(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_inbound_and_link_arrivals_leave_the_current_room_alone() {
        assert!(!Selection::Inbound {
            room: RoomId::Second,
            exit: 0
        }
        .edits_current_room());
        assert!(!Selection::LinkArrival(1).edits_current_room());
        for own in [
            Selection::Enemy(0),
            Selection::Boss,
            Selection::Item(0),
            Selection::PatrolLeft(0),
            Selection::PatrolRight(0),
            Selection::Spawn,
            Selection::Exit(0),
            Selection::Destination(0),
        ] {
            assert!(own.edits_current_room(), "{own:?}");
        }
    }
}
