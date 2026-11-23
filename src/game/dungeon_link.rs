//! Gives the open editor session the run rules Check dungeon needs (`dungeon_check`) and the
//! generator rules the dungeon preview needs (`dungeon_preview`).
use super::GameState;

impl GameState {
    /// Copies `run.ron`'s rules and `variety.ron`'s generator rules into the open editor session.
    pub(super) fn attach_run_config(&mut self) {
        let config = *self.run_config();
        let generator = self.variety_rules().generator;
        if let Some(session) = &mut self.editor {
            session.run = config;
            session.generator = generator;
        }
    }
}
