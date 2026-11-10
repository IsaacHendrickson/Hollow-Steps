//! What happens when the editor is asked to close or switch rooms while it has unsaved
//! changes. `UnsavedGuard` is a small state machine: the front end reports requests,
//! dialog buttons and save results, and it says what to do next. It never touches the
//! session itself; `perform` carries out an approved action.
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

#[derive(Debug, Default)]
pub struct UnsavedGuard {
    prompt: Option<Prompt>,
    saving: bool,
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
        GuardStep::Wait
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

/// Carries out an approved action: closes the editor, opens a room in it, or starts a
/// new room draft. The error (a room that will not load) is for the editor message.
pub fn perform(game: &mut GameState, action: &GuardedAction) -> Result<(), String> {
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
