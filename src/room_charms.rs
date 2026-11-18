//! Charms lying in rooms: how a room file places them (`CharmPlacement`, in `RoomLayout::charms`),
//! how the layout checks them, and when the player can reach one (`WorldCharm::can_pick_up`).
//!
//! A charm is found at most once: picking one up adds it to the owned charms and takes it out of
//! every room for the rest of the run (see `game::world_charms`). Which charms exist is
//! `charms.ron`; a room only names them by id.
use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::charms::Rgba;
use crate::editor::RoomLayout;
use crate::editor_selection::Selection;
use crate::item;
use crate::player::Player;
use crate::room::Room;
use crate::terrain::Terrain;

/// The size of a charm lying in a room, centered on its position.
pub const CHARM_SIZE: Vec2 = Vec2::new(16.0, 16.0);

/// One charm in a room file: the charm's id (from `charms.ron`) and where it lies.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CharmPlacement {
    pub charm: String,
    /// Center of the charm's rectangle.
    pub position: (f32, f32),
}

/// A charm lying in the room being played.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldCharm {
    pub charm: String,
    /// Center of the charm's rectangle.
    pub position: Vec2,
}

impl WorldCharm {
    /// The same reach rule as `WorldItem::can_pick_up`: a grounded player within
    /// `PICKUP_REACH` of the charm's rectangle.
    pub fn can_pick_up(&self, player: &Player) -> bool {
        item::within_reach(self.position, CHARM_SIZE, player)
    }
}

/// A charm lying in the room being played, as the scene draws it.
#[derive(Clone, Debug, PartialEq)]
pub struct LyingCharm {
    /// Center of the charm's rectangle.
    pub position: Vec2,
    /// The charm's swatch color.
    pub color: Rgba,
    /// The charm's name, for the pickup prompt.
    pub name: String,
    /// The player could pick it up now.
    pub in_reach: bool,
}

impl From<&CharmPlacement> for WorldCharm {
    fn from(placement: &CharmPlacement) -> Self {
        Self {
            charm: placement.charm.clone(),
            position: placement.position.into(),
        }
    }
}

impl RoomLayout {
    /// The charms of a built room, as the layout that would place them.
    pub(crate) fn charm_placements(room: &Room) -> Vec<CharmPlacement> {
        room.charms
            .iter()
            .map(|charm| CharmPlacement {
                charm: charm.charm.clone(),
                position: charm.position.into(),
            })
            .collect()
    }

    /// The charms this layout places, as they lie in a room.
    pub(crate) fn world_charms(&self) -> Vec<WorldCharm> {
        self.charms.iter().map(WorldCharm::from).collect()
    }

    /// Every charm lies inside the room above the floor and outside the solids.
    pub(crate) fn validate_charm_positions(&self, terrain: &Terrain) -> Result<(), String> {
        for charm in &self.charms {
            if !self.contains_body(charm.position, CHARM_SIZE) {
                return Err(format!(
                    "charm \"{}\" must be inside the room above the floor",
                    charm.charm
                ));
            }
            if terrain.overlaps_body(charm.position.into(), CHARM_SIZE) {
                return Err(format!(
                    "charm \"{}\" must not be inside terrain",
                    charm.charm
                ));
            }
        }
        Ok(())
    }

    /// Every placed charm is one `exists` knows; the error names the first that is not.
    pub fn validate_charm_ids(&self, exists: &dyn Fn(&str) -> bool) -> Result<(), String> {
        match self.charms.iter().find(|charm| !exists(&charm.charm)) {
            Some(charm) => Err(format!("charm \"{}\" does not exist", charm.charm)),
            None => Ok(()),
        }
    }

    pub(crate) fn place_charm(&mut self, charm: String, position: Vec2) -> Selection {
        self.charms.push(CharmPlacement {
            charm,
            position: position.into(),
        });
        Selection::Charm(self.charms.len() - 1)
    }

    pub(crate) fn move_charm(&mut self, index: usize, position: Vec2) -> Result<(), String> {
        self.charms.get_mut(index).ok_or("missing charm")?.position = position.into();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
