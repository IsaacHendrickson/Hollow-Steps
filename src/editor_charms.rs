//! The room editor's side of charms in rooms: charms are placed like items, from the same
//! list ("Charm: <name>"), and a placed charm is drawn as the charm's color swatch.
//!
//! The list offers the charms of the Charms panel's working copy, so a charm that was just made
//! can be placed. A room can only be saved naming charms that are saved (`GameState::save_editor`
//! says so), and a charm that rooms place cannot be deleted from the saved list
//! (`GameState::save_charms`).
use crate::charms::Rgba;
use crate::editor::Placement;
use crate::editor_session::EditorSession;

/// The swatch of a charm the working list does not have (deleted in the Charms panel).
const UNKNOWN_SWATCH: Rgba = [128, 128, 128, 255];

impl EditorSession {
    /// Everything the placement list offers: enemies, bosses and items from the definitions,
    /// then the charms.
    pub fn placement_catalog(&self) -> Vec<(String, Placement)> {
        let mut list = self.content.placement_catalog();
        list.extend(self.charms.edited().iter().map(|charm| {
            (
                format!("Charm: {}", charm.name),
                Placement::Charm(charm.id.clone()),
            )
        }));
        list
    }

    /// The color a placed charm is drawn in.
    pub fn charm_swatch(&self, id: &str) -> Rgba {
        self.charms
            .edited()
            .iter()
            .find(|charm| charm.id == id)
            .map_or(UNKNOWN_SWATCH, |charm| charm.color)
    }
}
