//! How `GameState` holds the player draft (`crate::player_editor`): one draft of the
//! `player.ron` numbers that the F4 window edits directly and the editor's Player panel edits
//! through the session's working copy.
//!
//! While the editor is open its session carries a copy of the draft for the panel to edit.
//! The game takes that copy into effect once per editor frame (`sync_player_from_editor`), and
//! the copy is refreshed from the game whenever the editor opens or a test room ends, so the
//! two never drift apart. F4 is not drawn while the editor is open. The look
//! (`player_look_editor`) lives only in the session; the Player panel's Save writes it too.
use std::path::PathBuf;

use super::GameState;
use crate::player_editor::{PlayerDraft, SpawnCheck};
use crate::tuning::Field;

impl GameState {
    /// The player and enemy tunables as saved and as in effect.
    pub fn player_draft(&self) -> &PlayerDraft {
        &self.player_draft
    }

    /// Where the player starts in every room. With the editor open this is the editor's view of
    /// the rooms: every saved room, with the room being edited as its draft has it.
    pub fn spawn_checks(&self) -> Vec<SpawnCheck> {
        match &self.editor {
            Some(session) => session.spawn_checks(),
            None => self.rooms.iter().map(SpawnCheck::from_room).collect(),
        }
    }

    /// Puts one edited number into effect at once, mid-game. A value outside the field's range
    /// is refused and nothing changes. Combinations that only fail `PlayerDraft::problems` are
    /// accepted (they are shown, and block Save).
    pub fn set_tuning_field(&mut self, field: Field, value: f32) -> Result<(), String> {
        let mut draft = self.player_draft;
        draft.set_field(field, value)?;
        self.set_tuning(*draft.edited())
    }

    /// Writes the tunables in effect to `player.ron` in the assets folder and returns its path.
    /// A draft with problems (`PlayerDraft::validate`) writes nothing.
    pub fn save_tuning(&mut self) -> Result<PathBuf, String> {
        self.sync_player_from_editor();
        let spawns = self.spawn_checks();
        let path = self.player_draft.save(&self.assets, &spawns)?;
        self.attach_player_draft();
        Ok(path)
    }

    /// Saves what the Player panel edits: `player.ron`, and `player_body.ron` when the look
    /// changed. Both are checked before either is written, so a refused look leaves
    /// `player.ron` as it was too. Returns the files written.
    pub fn save_player(&mut self) -> Result<Vec<PathBuf>, String> {
        self.sync_player_from_editor();
        let spawns = self.spawn_checks();
        self.player_draft.validate(&spawns)?;
        if let Some(session) = &self.editor {
            if session.look.is_dirty() {
                session.look.check_savable(
                    &self.assets,
                    &session.content.bodies,
                    &session.content.poses,
                )?;
            }
        }
        let mut written = vec![self.player_draft.save(&self.assets, &spawns)?];
        self.attach_player_draft();
        if let Some(session) = &mut self.editor {
            if session.look.is_dirty() {
                let (bodies, poses) = (&session.content.bodies, &session.content.poses);
                written.push(session.look.save(&self.assets, bodies, poses)?);
            }
        }
        Ok(written)
    }

    /// Gives the open editor session a copy of the game's draft to edit.
    pub(super) fn attach_player_draft(&mut self) {
        if let Some(session) = &mut self.editor {
            session.player = self.player_draft;
        }
    }

    /// Takes the open session's draft into effect: the player and every enemy use its values
    /// from now on. Does nothing with the editor closed or when nothing changed.
    pub fn sync_player_from_editor(&mut self) {
        let Some(draft) = self.editor.as_ref().map(|session| session.player) else {
            return;
        };
        if draft == self.player_draft || draft.edited().validate().is_err() {
            return;
        }
        let edited = *draft.edited();
        self.player.set_config(edited.player);
        for enemy in self.rooms.iter_mut().flat_map(|room| &mut room.enemies) {
            enemy.set_tuning(edited.enemy);
        }
        self.player_draft = draft;
    }
}
