//! How `GameState` holds the Charms panel's edits (`crate::charm_editor`).
//!
//! While the editor is open its session carries a `CharmDraft` for the panel to edit. The game
//! takes the draft's list into effect once per editor frame (`sync_charms_from_editor`), so an
//! equipped charm that is edited changes the player's stats at once, and a Test room plays the
//! unsaved list. The draft is refreshed from the game when the editor opens. Nothing is written
//! until `save_charms`, which then makes the saved list the running one the way F5 does: the
//! player is not reset, a charm the list no longer has is unequipped and dropped from the owned
//! list, and the numbers in effect follow (`apply_stats`).
//!
//! The draft is checked against the base maximum mana of the *saved* `vitals.ron`, and a charm
//! that a saved room places cannot be deleted (`world_charms`).
use std::path::PathBuf;

use super::GameState;
use crate::charm_editor::CharmDraft;
use crate::charms::CharmCatalog;
use crate::player_editor::CharmCost;

/// What `GameState::save_charms` did.
#[derive(Clone, Debug, PartialEq)]
pub struct CharmsSaved {
    pub path: PathBuf,
    /// Names of the charms taken off because the saved costs no longer fit the base maximum
    /// mana together, last equipped first.
    pub taken_off: Vec<String>,
}

impl GameState {
    /// The base maximum mana the charm list is checked against: the saved vitals'.
    fn charm_check_mana(&self) -> f32 {
        match &self.editor {
            Some(session) => session.player.saved_vitals().max_mana,
            None => self.player_draft.saved_vitals().max_mana,
        }
    }

    /// The mana cost of every charm in `charms.ron` as it is on disk now: the Charms panel's saved
    /// list (also while a test room has the session parked) or, with the editor closed, the list
    /// in play. Unsaved edits of the list are not in it.
    pub(super) fn saved_charm_costs(&self) -> Vec<CharmCost> {
        match self.editor.as_ref().or_else(|| self.parked_editor()) {
            Some(session) => CharmCost::from_charms(session.charms.saved()),
            None => CharmCost::from_charms(self.charms.charms()),
        }
    }

    /// Gives the open editor session a copy of the game's charm list to edit.
    pub(super) fn attach_charm_draft(&mut self) {
        if let Some(session) = &mut self.editor {
            session.charms = CharmDraft::from_catalog(&self.charms);
        }
    }

    /// True while the open session's charm list differs from the saved one.
    pub fn charms_unsaved(&self) -> bool {
        self.editor
            .as_ref()
            .is_some_and(|session| session.charms.is_dirty())
    }

    /// Takes the open session's charm list into effect: every charm and the numbers equipped
    /// charms give use it from now on. A list with a problem is not applied (the panel shows
    /// it). Does nothing with the editor closed or when nothing changed.
    pub fn sync_charms_from_editor(&mut self) {
        let base = self.charm_check_mana();
        let Some(session) = &self.editor else {
            return;
        };
        if session.charms.edited() == self.charms.charms() {
            return;
        }
        if let Ok(catalog) = session.charms.catalog(base) {
            self.charms = catalog;
            self.apply_stats();
        }
    }

    /// `Ok` when the session's charm list may be played or saved (the same rules as Save).
    pub fn charm_problem(&self) -> Result<(), String> {
        match &self.editor {
            Some(session) => session.charms.validate(self.charm_check_mana()),
            None => Ok(()),
        }
    }

    /// Writes the session's charm list to `charms.ron` in the assets folder (an invalid list
    /// writes nothing) and makes it the running one without resetting the player. A charm the
    /// list no longer has is unequipped and no longer owned; if the saved costs together no
    /// longer fit the base maximum mana, charms are taken off from the last equipped.
    pub fn save_charms(&mut self) -> Result<CharmsSaved, String> {
        // A list with a problem is reported as itself, before anything about rooms.
        self.charm_problem()?;
        if let Some((charm, file)) = self.deleted_charm_in_saved_room()? {
            return Err(format!(
                "charm \"{charm}\" is placed in {file}: take it out of the room and save the \
                 room before deleting the charm"
            ));
        }
        let base = self.charm_check_mana();
        let session = self.editor.as_mut().ok_or("editor is closed")?;
        let path = session.charms.save(&self.assets, base)?;
        self.charms = session.charms.catalog(base)?;
        self.charm_state.retain_known(&self.charms);
        let taken_off = self
            .charm_state
            .trim_to_budget(&self.charms, self.player_draft.edited_vitals().max_mana)
            .into_iter()
            .filter_map(|id| self.charms.get(&id).map(|charm| charm.name.clone()))
            .collect();
        self.apply_stats();
        Ok(CharmsSaved { path, taken_off })
    }

    /// Gives up the Charms panel's unsaved edits: the session's list and the running one go
    /// back to the saved list. Owned and equipped charms are untouched.
    pub fn discard_charm_edits(&mut self) {
        let base = self.charm_check_mana();
        let Some(session) = &mut self.editor else {
            return;
        };
        session.charms.reset_all();
        if let Ok(catalog) = CharmCatalog::new(session.charms.edited().to_vec(), base) {
            self.charms = catalog;
            self.apply_stats();
        }
    }
}
