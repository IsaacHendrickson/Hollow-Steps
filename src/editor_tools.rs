//! Editor tool selection and the terrain tool's drag state. Pure: the front end passes
//! mouse positions in world space, and every change goes through the session so undo
//! covers it (a drag is one step).
use macroquad::math::{Rect, Vec2};

use crate::editor::{EditorSession, RoomDraft};
use crate::editor_history::{tracked, History};
use crate::editor_terrain::{clamp_to_room, corners_rect, handle_at, Handle};
use crate::editor_view::Grid;

/// What a click or drag on the editor canvas does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EditorTool {
    /// Select and drag things; resize a selected solid by its handles.
    #[default]
    Select,
    /// Place the chosen entity or item.
    Place,
    /// Drag a rectangle to make a solid.
    Terrain,
    /// Click to delete the thing under the cursor.
    Erase,
}

impl EditorTool {
    pub const ALL: [EditorTool; 4] = [
        EditorTool::Select,
        EditorTool::Place,
        EditorTool::Terrain,
        EditorTool::Erase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EditorTool::Select => "Select",
            EditorTool::Place => "Place",
            EditorTool::Terrain => "Terrain",
            EditorTool::Erase => "Erase",
        }
    }

    /// The keyboard shortcut, shown in the panel.
    pub fn key(self) -> char {
        match self {
            EditorTool::Select => 'V',
            EditorTool::Place => 'P',
            EditorTool::Terrain => 'T',
            EditorTool::Erase => 'E',
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            EditorTool::Select => {
                "Click to select and drag things or solids. Drag a handle to resize a solid."
            }
            EditorTool::Place => "Click to place the chosen entity or item.",
            EditorTool::Terrain => "Drag on the canvas to draw a solid.",
            EditorTool::Erase => "Click a solid, enemy, item or exit to delete it.",
        }
    }
}

/// Maps the keyboard to a tool: V, P, T and E ("pressed this frame"). Ignored when a
/// modifier key is held, so Ctrl+E and the like stay free.
pub fn tool_shortcut(
    modified: bool,
    select: bool,
    place: bool,
    terrain: bool,
    erase: bool,
) -> Option<EditorTool> {
    if modified {
        return None;
    }
    [
        (select, EditorTool::Select),
        (place, EditorTool::Place),
        (terrain, EditorTool::Terrain),
        (erase, EditorTool::Erase),
    ]
    .into_iter()
    .find_map(|(pressed, tool)| pressed.then_some(tool))
}

/// What a click landed on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainHit {
    Handle(usize, Handle),
    Body(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Gesture {
    /// Drawing a new solid between two corners (snapped, inside the room).
    Draw {
        start: Vec2,
        end: Vec2,
    },
    /// Moving a solid; `grab` is the pointer's offset from its top-left corner.
    Move {
        index: usize,
        grab: Vec2,
    },
    Resize {
        index: usize,
        handle: Handle,
    },
}

/// The selected solid and the drag in progress.
#[derive(Debug, Default)]
pub struct TerrainTool {
    selected: Option<usize>,
    gesture: Option<Gesture>,
}

impl TerrainTool {
    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn select(&mut self, index: Option<usize>) {
        self.selected = index;
    }

    /// Drops the selection and any drag, for example after undo restores a different draft.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn has_gesture(&self) -> bool {
        self.gesture.is_some()
    }

    /// The solid being drawn right now, for the front end to show.
    pub fn preview(&self) -> Option<Rect> {
        match self.gesture {
            Some(Gesture::Draw { start, end }) => Some(corners_rect(start, end)),
            _ => None,
        }
    }

    /// Forgets a selection or drag that no longer points at a solid.
    pub fn sync(&mut self, draft: &RoomDraft) {
        let count = draft.layout.terrain.len();
        if self.selected.is_some_and(|i| i >= count) {
            self.selected = None;
        }
        if let Some(Gesture::Move { index, .. } | Gesture::Resize { index, .. }) = self.gesture {
            if index >= count {
                self.gesture = None;
            }
        }
    }

    /// A handle of the selected solid within `reach` world units of `world`.
    pub fn hit_handle(&self, draft: &RoomDraft, world: Vec2, reach: f32) -> Option<TerrainHit> {
        let index = self.selected?;
        let handle = handle_at(&draft.terrain_rect(index)?, world, reach)?;
        Some(TerrainHit::Handle(index, handle))
    }

    /// The solid under `world`, topmost first.
    pub fn hit_body(draft: &RoomDraft, world: Vec2) -> Option<TerrainHit> {
        draft.terrain_at(world).map(TerrainHit::Body)
    }

    /// Select tool press on a solid: selects it and starts moving or resizing. The whole
    /// drag becomes one undo step.
    pub fn grab(
        &mut self,
        session: &EditorSession,
        history: &mut History,
        hit: TerrainHit,
        world: Vec2,
    ) {
        let (index, gesture) = match hit {
            TerrainHit::Handle(index, handle) => (index, Gesture::Resize { index, handle }),
            TerrainHit::Body(index) => {
                let Some(rect) = session.draft.terrain_rect(index) else {
                    return;
                };
                (
                    index,
                    Gesture::Move {
                        index,
                        grab: world - rect.point(),
                    },
                )
            }
        };
        self.selected = Some(index);
        self.gesture = Some(gesture);
        history.begin_gesture(&session.draft);
    }

    /// Terrain tool press: starts a new solid at `world`, snapped and kept inside the room.
    pub fn begin_draw(&mut self, session: &EditorSession, world: Vec2, grid: &Grid) {
        let at = Self::point(session, world, grid);
        self.selected = None;
        self.gesture = Some(Gesture::Draw { start: at, end: at });
    }

    fn point(session: &EditorSession, world: Vec2, grid: &Grid) -> Vec2 {
        let room = session.draft.layout.rect();
        grid.snap(clamp_to_room(&room, world), room.point())
    }

    /// The pointer moved with the button held. A move or resize that the layout rules refuse
    /// leaves the draft at its last valid state and returns the reason.
    pub fn drag(
        &mut self,
        session: &mut EditorSession,
        world: Vec2,
        grid: &Grid,
    ) -> Result<(), String> {
        let origin = session.draft.layout.rect().point();
        let before = session.draft.layout.terrain.clone();
        let result = match self.gesture {
            None => return Ok(()),
            Some(Gesture::Draw { start, .. }) => {
                let end = Self::point(session, world, grid);
                self.gesture = Some(Gesture::Draw { start, end });
                return Ok(());
            }
            Some(Gesture::Move { index, grab }) => session
                .draft
                .move_terrain(index, grid.snap(world - grab, origin)),
            Some(Gesture::Resize { index, handle }) => {
                let at = Self::point(session, world, grid);
                session.draft.resize_terrain(index, handle, at)
            }
        };
        if result.is_ok() {
            session.message.clear();
        }
        if session.draft.layout.terrain != before {
            session.dirty = true;
        }
        result
    }

    /// The button was released: a drawn solid is added, a move or resize is closed as one
    /// undo step. A drawn solid marks the session unsaved here; a drag does so as it moves.
    pub fn release(
        &mut self,
        session: &mut EditorSession,
        history: &mut History,
    ) -> Result<(), String> {
        match self.gesture.take() {
            None => Ok(()),
            Some(Gesture::Draw { start, end }) => {
                let rect = corners_rect(start, end);
                // A click without a drag is just focusing the canvas.
                if rect.w == 0.0 && rect.h == 0.0 {
                    return Ok(());
                }
                let index = tracked(history, session, |s| s.draft.add_terrain(rect))?;
                self.selected = Some(index);
                session.dirty = true;
                session.message.clear();
                Ok(())
            }
            Some(_) => {
                history.end_gesture(&session.draft);
                Ok(())
            }
        }
    }

    /// Erase tool: deletes the topmost solid under `world`. `Ok(false)` if there is none.
    pub fn erase_at(
        &mut self,
        session: &mut EditorSession,
        history: &mut History,
        world: Vec2,
    ) -> Result<bool, String> {
        match session.draft.terrain_at(world) {
            Some(index) => self.delete(session, history, index).map(|()| true),
            None => Ok(false),
        }
    }

    /// Deletes the selected solid. `Ok(false)` if none is selected.
    pub fn delete_selected(
        &mut self,
        session: &mut EditorSession,
        history: &mut History,
    ) -> Result<bool, String> {
        match self.selected {
            Some(index) => self.delete(session, history, index).map(|()| true),
            None => Ok(false),
        }
    }

    fn delete(
        &mut self,
        session: &mut EditorSession,
        history: &mut History,
        index: usize,
    ) -> Result<(), String> {
        tracked(history, session, |s| s.draft.delete_terrain(index))?;
        session.dirty = true;
        self.gesture = None;
        self.selected = match self.selected {
            Some(i) if i == index => None,
            Some(i) if i > index => Some(i - 1),
            other => other,
        };
        Ok(())
    }
}
