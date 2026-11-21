//! Check dungeon in the editor: the checker run over every room as the session has it.
use super::{check, Problem};
use crate::editor_session::EditorSession;

impl EditorSession {
    /// What is wrong with the dungeon, with every room as the editor has it (the room being
    /// edited and any unsaved drafts in place of their files) under the run rules of `run.ron`.
    pub fn check_dungeon(&self) -> Vec<Problem> {
        check(&self.catalog(), &self.run)
    }
}
