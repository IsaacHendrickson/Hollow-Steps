//! How `GameState` holds the Upgrades panel's edits (`crate::upgrade_editor`).
//!
//! While the editor is open its session carries an `UpgradeDraft` for the panel to edit. The game
//! takes the draft's list into effect once per editor frame (`sync_upgrades_from_editor`), so the
//! levels the profile holds change the player's stats at once and a Test room plays the unsaved
//! list. The draft is refreshed from the game when the editor opens. Nothing is written until
//! `save_upgrades`, which then makes the saved list the running one the way F5 does: the player is
//! not reset and the numbers in effect follow (`apply_stats`).
//!
//! The levels bought live in the profile, by upgrade id, and are never touched here: an upgrade
//! deleted from the list keeps its levels, with no effect, until an upgrade with that id is
//! listed again. Upgrades are in effect only in a game with a hub (`charm_link::permanent_effects`).
use std::path::PathBuf;

use super::GameState;
use crate::upgrade_editor::UpgradeDraft;

impl GameState {
    /// Gives the open editor session a copy of the game's upgrade list to edit.
    fn attach_upgrade_draft(&mut self) {
        if let Some(session) = &mut self.editor {
            session.upgrades = UpgradeDraft::from_catalog(&self.upgrades);
        }
    }

    /// Gives the open editor session the game's charm list and upgrade list to edit.
    pub(super) fn attach_catalog_drafts(&mut self) {
        self.attach_charm_draft();
        self.attach_upgrade_draft();
        self.attach_run_config();
    }

    /// After a reload from the files: a panel with nothing unsaved shows the re-read list;
    /// unsaved edits stay (and are played again by the next frame's sync).
    pub(super) fn refresh_catalog_drafts(&mut self) {
        if !self.charms_unsaved() {
            self.attach_charm_draft();
        }
        if !self.upgrades_unsaved() {
            self.attach_upgrade_draft();
        }
    }

    /// True while the open session's upgrade list differs from the saved one.
    pub fn upgrades_unsaved(&self) -> bool {
        self.editor
            .as_ref()
            .is_some_and(|session| session.upgrades.is_dirty())
    }

    /// Takes the open session's upgrade list into effect: every upgrade, and the numbers the
    /// profile's levels give, use it from now on. A list with a problem is not applied (the panel
    /// shows it). Does nothing with the editor closed or when nothing changed.
    pub fn sync_upgrades_from_editor(&mut self) {
        let Some(session) = &self.editor else {
            return;
        };
        if session.upgrades.edited() == self.upgrades.upgrades() {
            return;
        }
        if let Ok(catalog) = session.upgrades.catalog() {
            self.upgrades = catalog;
            self.apply_stats();
        }
    }

    /// `Ok` when the session's upgrade list may be played or saved (the same rules as Save).
    pub fn upgrade_problem(&self) -> Result<(), String> {
        match &self.editor {
            Some(session) => session.upgrades.validate(),
            None => Ok(()),
        }
    }

    /// Writes the session's upgrade list to `upgrades.ron` in the assets folder (an invalid list
    /// writes nothing) and makes it the running one without resetting the player.
    pub fn save_upgrades(&mut self) -> Result<PathBuf, String> {
        let session = self.editor.as_mut().ok_or("editor is closed")?;
        let path = session.upgrades.save(&self.assets)?;
        self.upgrades = session.upgrades.catalog()?;
        self.apply_stats();
        Ok(path)
    }

    /// Gives up the Upgrades panel's unsaved edits: the session's list and the running one go
    /// back to the saved list.
    pub fn discard_upgrade_edits(&mut self) {
        let Some(session) = &mut self.editor else {
            return;
        };
        session.upgrades.reset_all();
        if let Ok(catalog) = session.upgrades.catalog() {
            self.upgrades = catalog;
            self.apply_stats();
        }
    }
}
