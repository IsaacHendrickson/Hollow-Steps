//! What the exit-link flow does to the editor's window state. The flow itself is
//! `hollow_steps::exit_link` (it swaps the rooms in the session); this keeps the window state
//! that belongs to room A while B is open (selection, view, tool, undo history) and puts it
//! back when the flow ends, and records the finished link as one undo step in each room.
use super::EditorUi;
use crate::editor_panel::{BossPropertyUi, LinkChoice};
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_history::History;
use hollow_steps::editor_tools::EditorTool;
use hollow_steps::editor_view::CanvasView;
use hollow_steps::exit_link::Linked;
use hollow_steps::room::{RoomEdge, RoomId};
use macroquad::prelude::*;

/// Room A's window state while B is open. Undo is off while B is open, so A's history waits
/// here and `EditorUi::history` is a scratch one that drags in B can use.
pub(crate) struct LinkStash {
    selected: Option<Selection>,
    destination: Option<usize>,
    view: Option<CanvasView>,
    view_destination: Option<usize>,
    tool: EditorTool,
    properties: bool,
    boss_property: Option<BossPropertyUi>,
    history: History,
    /// B and its undo history from before the flow, put back if the flow is cancelled and
    /// extended with the link if it finishes.
    target: RoomId,
    target_history: Option<History>,
}

/// Whether a window of the flow (the room chooser or the name window) is open.
pub(crate) fn link_dialog_open(editor: &EditorSession) -> bool {
    editor
        .link
        .as_ref()
        .is_some_and(|link| !link.is_connecting())
}

/// What the canvas still lets the user press while exits are being linked: B's exits and the
/// arrival marker of the one picked. Everything else is out of play until Connect or Cancel.
pub(crate) fn live_while_linking(marker: Selection) -> bool {
    matches!(marker, Selection::Exit(_) | Selection::LinkArrival(_))
}

/// Whether B is open and the editor is restricted to selecting and placing exits.
pub(crate) fn link_connecting(editor: &EditorSession) -> bool {
    editor
        .link
        .as_ref()
        .is_some_and(|link| link.is_connecting())
}

impl EditorUi {
    /// "Link..." on the selected exit: opens the room chooser.
    pub(crate) fn start_link(&mut self, editor: &mut EditorSession, exit: usize) {
        editor.message.clear();
        match editor.begin_link(exit) {
            Ok(()) => self.selected = Some(Selection::Exit(exit)),
            Err(error) => editor.message = error,
        }
    }

    /// "Add exit and link...": places an exit on `edge` and opens the room chooser for it.
    pub(crate) fn start_link_new_exit(&mut self, editor: &mut EditorSession, edge: RoomEdge) {
        editor.message.clear();
        match editor.begin_link_new_exit(edge) {
            Ok(exit) => {
                self.selected = Some(Selection::Exit(exit));
                self.tool = EditorTool::Select;
                self.destination = None;
            }
            Err(error) => editor.message = error,
        }
    }

    /// A window of the flow decided something this frame.
    pub(crate) fn apply_link_choice(&mut self, editor: &mut EditorSession, choice: LinkChoice) {
        match choice {
            LinkChoice::Room(room) => {
                editor.message.clear();
                match editor.link_choose(room) {
                    Ok(()) => self.enter_target(editor),
                    Err(error) => editor.message = error,
                }
            }
            LinkChoice::NewRoom => {
                if let Err(error) = editor.link_new_room() {
                    editor.message = error;
                }
            }
            LinkChoice::Name(name) => match editor.link_name_room(&name) {
                Ok(()) => self.enter_target(editor),
                Err(error) => {
                    if let Some(prompt) = editor.link_prompt() {
                        prompt.error = Some(error);
                    }
                }
            },
            LinkChoice::Cancel => {
                self.cancel_link(editor);
            }
        }
    }

    /// B just opened: A's window state is set aside and B starts with a fresh view.
    fn enter_target(&mut self, editor: &EditorSession) {
        let target = editor.id;
        let target_history = self
            .parked_histories
            .iter()
            .position(|(id, _)| *id == target)
            .map(|index| self.parked_histories.remove(index).1);
        self.link_stash = Some(Box::new(LinkStash {
            selected: self.selected.take(),
            destination: self.destination.take(),
            view: self.view.take(),
            view_destination: self.view_destination.take(),
            tool: self.tool,
            properties: self.properties,
            boss_property: self.boss_property.take(),
            history: std::mem::take(&mut self.history),
            target,
            target_history,
        }));
        self.tool = EditorTool::Select;
        self.properties = false;
        self.dragging = None;
        self.terrain.clear();
        self.link_placing = false;
    }

    /// Connect, with the selected exit of B: on success the editor is back in A.
    pub(crate) fn finish_link(&mut self, editor: &mut EditorSession) {
        let exit = match self.selected {
            Some(Selection::Exit(index)) => index,
            // Refused by the flow with the message that says what to do.
            _ => usize::MAX,
        };
        match editor.link_connect(exit) {
            Ok(linked) => self.leave_target(editor, Some(linked)),
            Err(error) => editor.message = error,
        }
    }

    /// Cancel (or F2), at whatever step the flow is in. Returns whether there was a flow.
    /// From the name window it goes back to the chooser; otherwise the flow ends with every
    /// draft as it was.
    pub(crate) fn cancel_link(&mut self, editor: &mut EditorSession) -> bool {
        if !editor.link_active() {
            return false;
        }
        let connecting = link_connecting(editor);
        editor.link_cancel();
        if connecting {
            self.leave_target(editor, None);
        }
        true
    }

    /// The flow ended while B was open: A's window state comes back. A finished link becomes
    /// one undo step in A and one in B; a cancelled one leaves both histories as they were.
    fn leave_target(&mut self, editor: &EditorSession, linked: Option<Linked>) {
        let Some(stash) = self.link_stash.take() else {
            return;
        };
        let stash = *stash;
        self.history = stash.history;
        match linked {
            Some(linked) => {
                // The editor is in A again, and B is parked with the link in it.
                self.history.record(linked.from_before, &editor.draft);
                let mut history = stash.target_history.unwrap_or_default();
                if let Some(work) = editor.parked.iter().find(|work| work.id == linked.to) {
                    history.record(linked.to_before, &work.draft);
                }
                self.parked_histories.retain(|(id, _)| *id != linked.to);
                self.parked_histories.push((linked.to, history));
            }
            None => {
                if let Some(history) = stash.target_history {
                    self.parked_histories.push((stash.target, history));
                }
            }
        }
        self.selected = stash.selected;
        self.destination = stash.destination;
        self.view = stash.view;
        self.view_destination = stash.view_destination;
        self.tool = stash.tool;
        self.properties = stash.properties;
        self.boss_property = stash.boss_property;
        self.dragging = None;
        self.terrain.clear();
        self.link_placing = false;
    }

    /// "New exit here", then a click at `position` in B (already snapped to the grid).
    pub(crate) fn place_link_exit(&mut self, editor: &mut EditorSession, position: Vec2) {
        match editor.link_place_exit(position) {
            Ok(index) => {
                self.selected = Some(Selection::Exit(index));
                self.link_placing = false;
                editor.message.clear();
            }
            Err(error) => editor.message = error,
        }
    }
}

/// Outlines B's exits while one is being picked: every exit in yellow, the one placed in
/// this step in green.
pub(crate) fn draw_exit_highlights(editor: &EditorSession, canvas: &CanvasView) {
    let placed = editor.link_placed_exit();
    for (index, exit) in editor.draft.layout.exits.iter().enumerate() {
        let zone = canvas.rect(exit.exit().zone(editor.draft.layout.rect()));
        let color = if placed == Some(index) { GREEN } else { YELLOW };
        draw_rectangle_lines(
            zone.x - 4.0,
            zone.y - 4.0,
            zone.w + 8.0,
            zone.h + 8.0,
            3.0,
            color,
        );
    }
}
