//! What the "New room" window's decision does to the editor's state.
use super::EditorUi;
use crate::editor_panel::NameChoice;
use hollow_steps::editor_guard::GuardedAction;

impl EditorUi {
    /// The "New room" window decided something this frame. A good name goes on to the
    /// unsaved-changes guard as a request to start that room; Cancel only closes the window.
    pub(super) fn finish_naming(&mut self, choice: Option<NameChoice>) {
        match choice {
            Some(NameChoice::Create(name)) => {
                self.name_prompt = None;
                self.requested = Some(GuardedAction::NewNamedRoom(name));
            }
            Some(NameChoice::Cancel) => self.name_prompt = None,
            None => {}
        }
    }
}
