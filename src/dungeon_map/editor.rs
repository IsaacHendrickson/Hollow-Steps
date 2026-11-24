//! The dungeon map in the editor: a room's own cell on its draft, the map of every room the
//! session knows (saved rooms and unsaved drafts alike), and what the Dungeon map window shows
//! and what a click in it hits. Pure: the window itself is `editor_panel::map_window`.
use macroquad::math::Vec2;

use super::{check_cell, Cell, MapGrid, MapLayout, MapLink, MapRoomInput};
use crate::editor::RoomDraft;
use crate::editor_session::EditorSession;
use crate::room::RoomId;

impl RoomDraft {
    /// Gives the room its own map cell, or `None` to let the exits place it. Refused, with the
    /// draft unchanged, for a cell outside -64..=64 on either number.
    pub fn set_map_cell(&mut self, cell: Option<Cell>) -> Result<(), String> {
        if let Some(cell) = cell {
            check_cell(cell)?;
        }
        self.layout.map_cell = cell;
        Ok(())
    }
}

impl EditorSession {
    /// Every room the session knows on the map: the saved rooms, with each unsaved draft (the
    /// room being edited and the parked ones) in place of its saved layout. The same rooms as
    /// `catalog`, without copying their layouts: the panel asks every frame.
    pub fn dungeon_map(&self) -> MapLayout {
        let drafts = std::iter::once((self.id, &self.draft.layout))
            .chain(self.parked.iter().map(|work| (work.id, &work.draft.layout)));
        let saved = self.rooms.iter().map(|(id, layout)| (*id, layout));
        // The first of a room's layouts wins, so a draft stands in for the saved room.
        let inputs: Vec<MapRoomInput> = drafts
            .chain(saved)
            .map(|(id, layout)| MapRoomInput::from_layout(id, layout))
            .collect();
        MapLayout::build(&inputs)
    }

    /// The cell the map puts the room being edited in, whether it names one or not.
    pub fn resolved_map_cell(&self) -> Option<Cell> {
        self.dungeon_map().room(self.id).map(|room| room.cell)
    }
}

/// One room of the Dungeon map window.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorMapRoom {
    pub id: RoomId,
    pub name: String,
    pub cell: Cell,
    /// The room being edited.
    pub current: bool,
    /// The room named a cell another room has; the map moved it.
    pub conflict: bool,
    /// No exit of any room leads here.
    pub no_entrance: bool,
}

/// What the Dungeon map window draws: every room, in room id order, and the links between them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditorMap {
    pub rooms: Vec<EditorMapRoom>,
    pub links: Vec<MapLink>,
}

impl EditorMap {
    /// The map of `layout` as the window shows it, with `current` highlighted.
    pub fn new(layout: &MapLayout, current: RoomId) -> Self {
        Self::marking(layout, |id| id == current)
    }

    /// The map of `layout` with no room highlighted: a map of rooms nobody is editing (the
    /// generated dungeon preview).
    pub fn plain(layout: &MapLayout) -> Self {
        Self::marking(layout, |_| false)
    }

    fn marking(layout: &MapLayout, is_current: impl Fn(RoomId) -> bool) -> Self {
        Self {
            rooms: layout
                .rooms
                .iter()
                .map(|room| EditorMapRoom {
                    id: room.id,
                    name: room.name.clone(),
                    cell: room.cell,
                    current: is_current(room.id),
                    conflict: layout.in_conflict(room.id),
                    no_entrance: !layout.has_entrance(room.id),
                })
                .collect(),
            links: layout.links.clone(),
        }
    }

    /// The cells of every room, for fitting a grid.
    pub fn cells(&self) -> Vec<Cell> {
        self.rooms.iter().map(|room| room.cell).collect()
    }

    /// The room whose box holds `point` on `grid`, if any.
    pub fn room_at(&self, grid: &MapGrid, point: Vec2) -> Option<RoomId> {
        self.rooms
            .iter()
            .find(|room| grid.room_rect(room.cell).contains(point))
            .map(|room| room.id)
    }
}
