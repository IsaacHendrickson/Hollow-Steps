//! The arrival points of other rooms' exits that lead into the room being edited. The editor
//! lists them to draw on this room's canvas, and moves them here without leaving the room.
//!
//! The exit belongs to the other room, so moving its arrival changes that room's draft: the
//! draft is parked (the way the link flow parks the rooms it touches) and marked unsaved, and
//! Save writes it together with this room, checking them as a set, all or none.
use macroquad::math::Vec2;

use crate::editor_session::EditorSession;
use crate::exit_arrival::arrival_error;
use crate::room::RoomId;

/// One exit of another room that leads into the room being edited.
#[derive(Clone, Debug, PartialEq)]
pub struct InboundArrival {
    /// The room the exit is in, and the exit's index there.
    pub room: RoomId,
    pub exit: usize,
    /// Where the player arrives in the room being edited.
    pub position: Vec2,
    /// The name of the room the exit is in.
    pub title: String,
    /// Why the point does not work in the room as it is now (a solid was drawn over it, say).
    pub error: Option<&'static str>,
}

impl EditorSession {
    /// The exits of the other rooms that lead here, as the editor has those rooms (unsaved
    /// drafts included). An exit of this room that leads to itself is not listed.
    pub fn inbound_arrivals(&self) -> Vec<InboundArrival> {
        let saved = self
            .rooms
            .iter()
            .filter(|(id, _)| !self.parked.iter().any(|work| work.id == *id))
            .map(|(id, layout)| (*id, layout));
        let parked = self.parked.iter().map(|work| (work.id, &work.draft.layout));
        let mut found = Vec::new();
        for (room, layout) in saved.chain(parked).filter(|(room, _)| *room != self.id) {
            for (exit, placement) in layout.exits.iter().enumerate() {
                if placement.target_room != self.id {
                    continue;
                }
                let position = Vec2::from(placement.spawn);
                found.push(InboundArrival {
                    room,
                    exit,
                    position,
                    title: self.title_of(room),
                    error: arrival_error(&self.draft.layout, position),
                });
            }
        }
        found
    }

    /// Moves where the player arrives from exit `exit` of room `room` to `position` in the
    /// room being edited. Refused, with the reason and nothing changed, when the exit does
    /// not lead here or the player does not fit at `position`. On success `room` is parked
    /// with the change and marked unsaved; this room is not touched.
    pub fn move_inbound_arrival(
        &mut self,
        room: RoomId,
        exit: usize,
        position: Vec2,
    ) -> Result<(), String> {
        if self.link.is_some() {
            return Err("finish or cancel the link in progress first".into());
        }
        if !position.is_finite() {
            return Err("position must be finite".into());
        }
        if room == self.id {
            return Err("that exit is in this room".into());
        }
        if let Some(reason) = arrival_error(&self.draft.layout, position) {
            return Err(format!("The player cannot arrive there: {reason}"));
        }
        let here = self.id;
        let mut loaded = None;
        if !self.parked.iter().any(|work| work.id == room) {
            loaded = Some(self.load_saved_work(room)?);
        }
        let work = match &mut loaded {
            Some(work) => work,
            None => self
                .parked
                .iter_mut()
                .find(|work| work.id == room)
                .expect("checked above"),
        };
        let placement = work
            .draft
            .layout
            .exits
            .get_mut(exit)
            .filter(|placement| placement.target_room == here)
            .ok_or("that exit no longer leads into this room")?;
        placement.spawn = position.into();
        work.dirty = true;
        if let Some(work) = loaded {
            self.parked.push(work);
        }
        Ok(())
    }
}
