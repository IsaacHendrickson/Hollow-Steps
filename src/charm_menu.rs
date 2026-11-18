//! The Charms section of the inventory as pure state: a cursor over the owned charms, equipping
//! and taking off the selected one, the maximum mana you would have after the change, and the
//! layout the drawing and the mouse share (`layout`). The rules for what may be equipped are
//! `crate::charms`; `GameState` puts the new numbers into effect after a toggle.

mod layout;
mod text;

pub use layout::{
    charm_at, layout, row_columns, visible_rows, CharmRow, InventoryLayout, RowColumns,
    DESCRIPTION_LINES, ITEM_STEP, MESSAGE_LINES, PANEL_WIDTH, ROW_HEIGHT,
};
pub use text::{fit_line, wrap_lines};

use crate::charms::{number_text, Charm, CharmCatalog, CharmState, Toggle};

/// The cursor of the charm list, the top of its scrolling window and the last refusal.
///
/// The cursor is an index into the owned charms; it wraps around the ends like the pause menu's
/// selection, and an index past the end (a charm that went away on a reload) counts as the last.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CharmMenu {
    selected: usize,
    first: usize,
    message: Option<String>,
}

impl CharmMenu {
    /// The selected owned charm, as an index into the `count` owned charms; none when there is
    /// none.
    pub fn selected(&self, count: usize) -> Option<usize> {
        count.checked_sub(1).map(|last| self.selected.min(last))
    }

    /// The owned charm at the top of a list that shows `visible` of `count` rows: where the
    /// window was left, moved only as far as the selection needs.
    pub fn first(&self, count: usize, visible: usize) -> usize {
        let Some(selected) = self.selected(count) else {
            return 0;
        };
        let visible = visible.clamp(1, count);
        let first = self.first.min(count - visible);
        if selected < first {
            selected
        } else if selected >= first + visible {
            selected + 1 - visible
        } else {
            first
        }
    }

    /// Why the last toggle was refused, until the selection moves or something is equipped or
    /// taken off.
    pub fn message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    pub fn clear_message(&mut self) {
        self.message = None;
    }

    /// Moves the cursor by `steps` charms (negative is up), wrapping around the ends, and
    /// scrolls the window of `visible` rows to keep it in view.
    pub fn step(&mut self, count: usize, steps: i32, visible: usize) {
        let Some(selected) = self.selected(count) else {
            return;
        };
        let moved = (selected as i64 + i64::from(steps)).rem_euclid(count as i64) as usize;
        self.first = self.first(count, visible);
        self.move_to(moved);
        self.first = self.first(count, visible);
    }

    /// Selects charm `index` (the mouse resting on its row). An index the list does not have is
    /// ignored. Returns whether the selection changed.
    pub fn select(&mut self, index: usize, count: usize) -> bool {
        if index >= count || self.selected(count) == Some(index) {
            return false;
        }
        self.move_to(index);
        true
    }

    fn move_to(&mut self, index: usize) {
        if self.selected != index {
            self.selected = index;
            self.message = None;
        }
    }

    /// Equips the selected charm, or takes it off if it is on. A refusal changes nothing in
    /// `state` and is kept as the message; anything else clears the message. None when no charm
    /// is owned. `base_max_mana` is the maximum without charms.
    pub fn toggle(
        &mut self,
        catalog: &CharmCatalog,
        state: &mut CharmState,
        base_max_mana: f32,
    ) -> Option<Toggle> {
        let owned = owned_charms(catalog, state);
        let id = owned.get(self.selected(owned.len())?)?.id.clone();
        let outcome = state.toggle(catalog, &id, base_max_mana);
        self.message = match &outcome {
            Toggle::Refused(message) => Some(message.clone()),
            Toggle::Equipped | Toggle::Unequipped => None,
        };
        Some(outcome)
    }
}

/// The charms the player owns, in the order the list shows them.
pub fn owned_charms<'a>(catalog: &'a CharmCatalog, state: &CharmState) -> Vec<&'a Charm> {
    state
        .owned()
        .iter()
        .filter_map(|id| catalog.get(id))
        .collect()
}

/// The maximum mana there would be after toggling `id`: the same toggle `CharmState` does,
/// tried on a copy. A toggle that would be refused changes nothing, so it is the maximum now.
pub fn max_mana_after(
    catalog: &CharmCatalog,
    state: &CharmState,
    id: &str,
    base_max_mana: f32,
) -> f32 {
    let mut after = state.clone();
    after.toggle(catalog, id, base_max_mana);
    after.max_mana(catalog, base_max_mana)
}

/// The line under the list: the maximum mana now and after toggling the selected charm.
pub fn mana_text(now: f32, after: f32) -> String {
    format!(
        "Max mana: {} now, {} after",
        number_text(now),
        number_text(after)
    )
}

#[cfg(test)]
mod tests;
