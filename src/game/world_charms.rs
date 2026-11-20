//! Charms lying in rooms, in play and in the editor's Save (`crate::room_charms`).
//!
//! In play: a charm is picked up with the pickup key under the same reach rule as an item. It
//! joins the owned charms and leaves every room for the rest of the run, so a charm that is
//! already owned is never placed again (rooms are pruned when the game starts and when the
//! editor rebuilds them; in a test room only the test room's copy is taken). Picking one up raises `GameEvent::ItemPickedUp`: a pickup in the
//! world, with the same meaning as for an item.
//!
//! In Save, so no file names a charm that does not exist: a room can only be saved naming saved
//! charms, and a charm that a saved room places cannot be deleted from the saved list.
use super::GameState;
use crate::behavior::{read_ron, ron_files};
use crate::charms::CharmCatalog;
use crate::editor_session::RoomWork;
use crate::events::GameEvent;
use crate::room::{Room, RoomEnemies};
use crate::room_charms::{LyingCharm, WorldCharm};
use crate::room_name::default_display_name;

impl GameState {
    /// The charms lying in the room being played, with where each is, its swatch color and
    /// name, and whether the player could pick it up now.
    pub fn room_charms(&self) -> Vec<LyingCharm> {
        self.room()
            .charms
            .iter()
            .filter_map(|lying| {
                let charm = self.charms.get(&lying.charm)?;
                Some(LyingCharm {
                    position: lying.position,
                    color: charm.color,
                    name: charm.name.clone(),
                    in_reach: lying.can_pick_up(&self.player),
                })
            })
            .collect()
    }

    /// Every charm a room places is one of `charms`.
    pub(super) fn validate_room_charms(
        rooms: &[Room],
        charms: &CharmCatalog,
    ) -> Result<(), String> {
        for room in rooms {
            if let Some(lying) = room.charms.iter().find(|c| charms.get(&c.charm).is_none()) {
                return Err(format!(
                    "room {}: charm \"{}\" does not exist in charms.ron",
                    default_display_name(room.id),
                    lying.charm
                ));
            }
        }
        Ok(())
    }

    /// Takes every charm the player already owns out of the rooms.
    pub(super) fn prune_owned_charm_placements(&mut self) {
        let owned = &self.charm_state;
        for room in &mut self.rooms {
            room.charms.retain(|lying| !owned.is_owned(&lying.charm));
        }
    }

    /// The first charm within reach of the player, if any: the one the pickup key would take.
    pub(super) fn charm_in_reach(&self) -> Option<&WorldCharm> {
        self.rooms[self.current].charms.iter().find(|lying| {
            self.charms.get(&lying.charm).is_some() && lying.can_pick_up(&self.player)
        })
    }

    /// Picks up the first charm within reach, if any: it becomes owned and no room places it
    /// any more. Returns whether one was picked up.
    pub(super) fn pick_up_charm(&mut self) -> bool {
        let Some(lying) = self.charm_in_reach() else {
            return false;
        };
        let id = lying.charm.clone();
        self.charm_state.grant(&id);
        if self.test_run.is_some() {
            // A test room is a sandbox: only its own copy of the charm is taken, and the
            // charms are put back when it ends, so the saved rooms keep theirs.
            self.rooms[self.current]
                .charms
                .retain(|lying| lying.charm != id);
        } else {
            self.prune_owned_charm_placements();
        }
        self.events.push(GameEvent::ItemPickedUp);
        true
    }

    /// The ids of the saved charms, for checking a room against.
    fn saved_charm_ids(&self) -> Vec<&str> {
        match &self.editor {
            Some(session) => session
                .charms
                .saved()
                .iter()
                .map(|charm| charm.id.as_str())
                .collect(),
            None => self.charms.charms().iter().map(|c| c.id.as_str()).collect(),
        }
    }

    /// `Ok` when every room about to be saved places only saved charms.
    pub(super) fn check_room_charms(&self, works: &[RoomWork]) -> Result<(), String> {
        let saved = self.saved_charm_ids();
        for work in works {
            let Some(unknown) = work
                .draft
                .layout
                .charms
                .iter()
                .find(|placed| !saved.contains(&placed.charm.as_str()))
            else {
                continue;
            };
            let title = self.editor.as_ref().map_or_else(
                || default_display_name(work.id),
                |session| session.title_of(work.id),
            );
            let made = self.editor.as_ref().is_some_and(|session| {
                session
                    .charms
                    .edited()
                    .iter()
                    .any(|c| c.id == unknown.charm)
            });
            return Err(if made {
                format!(
                    "room {title}: charm \"{}\" is not saved yet; save the charms first",
                    unknown.charm
                )
            } else {
                format!("room {title}: charm \"{}\" does not exist", unknown.charm)
            });
        }
        Ok(())
    }

    /// The charms the saved room files place, as (file name, charm id).
    fn charms_placed_in_saved_rooms(&self) -> Result<Vec<(String, String)>, String> {
        let mut placed = Vec::new();
        for path in ron_files(&self.assets.join("rooms"))? {
            let document: RoomEnemies = read_ron(&path)?;
            let file = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            for charm in document.layout.iter().flat_map(|layout| &layout.charms) {
                placed.push((file.to_owned(), charm.charm.clone()));
            }
        }
        Ok(placed)
    }

    /// The first charm the open session's list would delete that a saved room places, as
    /// (charm id, room file name); `None` when the list may be saved as far as rooms go.
    pub(super) fn deleted_charm_in_saved_room(&self) -> Result<Option<(String, String)>, String> {
        let Some(session) = &self.editor else {
            return Ok(None);
        };
        let deleted = session.charms.deleted_ids();
        if deleted.is_empty() {
            return Ok(None);
        }
        Ok(self
            .charms_placed_in_saved_rooms()?
            .into_iter()
            .find(|(_, charm)| deleted.contains(&charm.as_str()))
            .map(|(file, charm)| (charm, file)))
    }

    /// True when saving the charm list needs the rooms saved first: it deletes a charm that a
    /// saved room places (the room being saved may be the one that takes it out).
    pub fn charm_save_needs_rooms_first(&self) -> bool {
        matches!(self.deleted_charm_in_saved_room(), Ok(Some(_)))
    }
}
