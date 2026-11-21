//! Gives the open editor session the run rules Check dungeon needs (`dungeon_check`).
use super::GameState;

impl GameState {
    /// Copies `run.ron`'s rules into the open editor session.
    pub(super) fn attach_run_config(&mut self) {
        let config = *self.run_config();
        if let Some(session) = &mut self.editor {
            session.run = config;
        }
    }
}
