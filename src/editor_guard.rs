//! What happens when the editor is asked to close or switch rooms while it has unsaved
//! changes. `UnsavedGuard` is a small state machine: the front end reports requests,
//! dialog buttons and save results, and it says what to do next. It never touches the
//! session itself; `perform` carries out an approved action.
use crate::editor::EditorSession;
use crate::game::GameState;
use crate::room::RoomId;

/// Something that would end or replace the current editor session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuardedAction {
    /// F2 while the editor is open.
    Close,
    OpenRoom(RoomId),
    NewRoom,
}

impl GuardedAction {
    /// Finishes the sentence "Save your changes before you ...?".
    pub fn describe(&self) -> String {
        match self {
            Self::Close => "close the editor".into(),
            Self::OpenRoom(id) => format!("open room {id:?}"),
            Self::NewRoom => "start a new room".into(),
        }
    }
}

/// The three dialog buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuardChoice {
    Save,
    Discard,
    Cancel,
}

/// What the front end does next.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GuardStep {
    /// Nothing; a dialog is (or stays) open, or the dialog just closed with the session kept.
    Wait,
    /// Save the session, then report the result with `save_finished`.
    Save,
    /// Go ahead with the action. Unsaved edits were either saved or given up.
    Perform(GuardedAction),
}

/// The open dialog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prompt {
    pub action: GuardedAction,
    /// Why the last save attempt failed, shown in the dialog.
    pub error: Option<String>,
}

/// What a session has that is not saved, for the dialog to list: the room and its definitions,
/// and the Player panel's stats or look.
pub fn unsaved_items(session: &EditorSession) -> Vec<&'static str> {
    let mut items = Vec::new();
    if session.dirty {
        items.push("Room and definitions");
    }
    if player_unsaved(session) {
        items.push("Player");
    }
    items
}

fn player_unsaved(session: &EditorSession) -> bool {
    session.player.is_dirty() || session.look.is_dirty()
}

#[derive(Debug, Default)]
pub struct UnsavedGuard {
    prompt: Option<Prompt>,
    saving: bool,
    /// What the open dialog says is unsaved (`unsaved_items`); empty when not told.
    unsaved: Vec<String>,
}

impl UnsavedGuard {
    /// An action was requested. A clean session acts at once; a dirty one opens the
    /// dialog. Requests made while the dialog is open are ignored.
    pub fn request(&mut self, action: GuardedAction, dirty: bool) -> GuardStep {
        if self.prompt.is_some() {
            return GuardStep::Wait;
        }
        if !dirty {
            return GuardStep::Perform(action);
        }
        self.prompt = Some(Prompt {
            action,
            error: None,
        });
        self.unsaved.clear();
        GuardStep::Wait
    }

    /// `request` for an open session: it is dirty when the room or the player is, and the
    /// dialog lists which. `None` (no session) acts at once.
    pub fn request_for(
        &mut self,
        action: GuardedAction,
        session: Option<&EditorSession>,
    ) -> GuardStep {
        let items = session.map(unsaved_items).unwrap_or_default();
        let step = self.request(action, !items.is_empty());
        if self.prompt.is_some() {
            self.unsaved = items.into_iter().map(str::to_owned).collect();
        }
        step
    }

    /// What the open dialog lists as unsaved.
    pub fn unsaved(&self) -> &[String] {
        &self.unsaved
    }

    /// A dialog button was pressed. Cancel closes the dialog and keeps the session;
    /// Discard performs the action; Save asks the front end to save first.
    pub fn choose(&mut self, choice: GuardChoice) -> GuardStep {
        if self.saving {
            return GuardStep::Wait;
        }
        let Some(prompt) = self.prompt.take() else {
            return GuardStep::Wait;
        };
        match choice {
            GuardChoice::Cancel => GuardStep::Wait,
            GuardChoice::Discard => GuardStep::Perform(prompt.action),
            GuardChoice::Save => {
                self.prompt = Some(Prompt {
                    error: None,
                    ..prompt
                });
                self.saving = true;
                GuardStep::Save
            }
        }
    }

    /// The save asked for by `GuardStep::Save` finished. Success performs the action; a
    /// failure keeps the dialog open and shows the error. Ignored if no save was asked for.
    pub fn save_finished(&mut self, result: Result<(), String>) -> GuardStep {
        if !self.saving {
            return GuardStep::Wait;
        }
        self.saving = false;
        let Some(prompt) = self.prompt.take() else {
            return GuardStep::Wait;
        };
        match result {
            Ok(()) => GuardStep::Perform(prompt.action),
            Err(error) => {
                self.prompt = Some(Prompt {
                    error: Some(error),
                    ..prompt
                });
                GuardStep::Wait
            }
        }
    }

    pub fn prompt(&self) -> Option<&Prompt> {
        self.prompt.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.prompt.is_some()
    }
}

/// Saves what is unsaved in the open editor: the room and definitions, then the Player
/// panel's stats and look (which may name poses the first save just wrote). Stops at the first
/// failure, so nothing after it is written.
pub fn save_all(game: &mut GameState) -> Result<(), String> {
    let session = game.editor().ok_or("editor is closed")?;
    let (room, player) = (session.dirty, player_unsaved(session));
    if room {
        game.save_editor()?;
    }
    if player {
        game.save_player()?;
    }
    Ok(())
}

/// Carries out an approved action: closes the editor, opens a room in it, or starts a
/// new room draft. The error (a room that will not load) is for the editor message.
pub fn perform(game: &mut GameState, action: &GuardedAction) -> Result<(), String> {
    // Whatever the Player panel still has unsaved was given up (a Save wrote it first).
    game.discard_player_edits();
    match action {
        GuardedAction::Close if game.editor().is_some() => game.toggle_editor(),
        GuardedAction::Close => Ok(()),
        GuardedAction::OpenRoom(id) => game.edit_room(*id),
        GuardedAction::NewRoom => {
            game.editor_mut().ok_or("editor is closed")?.new_room();
            Ok(())
        }
    }
}
