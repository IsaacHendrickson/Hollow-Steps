//! The hub's upgrade shop as pure state: a cursor over the upgrades, buying the selected one, the
//! rows the list shows (name, level as "2/3", next price or "Max", what one level gives), and the
//! layout the drawing and the mouse share (`layout`). The rules for buying are
//! `crate::upgrades`; `GameState` owns the menu and puts the new numbers into effect after a
//! purchase (`game::shop_link`).
//!
//! The cursor wraps around the ends like the charm list and the pause menu, and an index past the
//! end (an upgrade that went away on a reload) counts as the last. The last result of a purchase
//! is kept as a message line until the cursor moves or the shop is opened again.

mod layout;

pub use layout::{
    layout, row_columns, upgrade_at, visible_rows, RowColumns, ShopLayout, ShopRowRect,
    DESCRIPTION_LINES, MESSAGE_LINES, PANEL_WIDTH, ROW_HEIGHT,
};

use macroquad::math::Vec2;

use crate::profile::Profile;
use crate::upgrades::{self, Purchase, Upgrade, UpgradeCatalog};

/// How big the shop's marker is, and so how near the player must stand (`item::within_reach`).
pub const SHOP_SIZE: Vec2 = Vec2::new(40.0, 48.0);

/// One row of the list, as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopRow {
    pub name: String,
    /// Level bought out of the levels there are: `2/3`.
    pub level: String,
    /// The price of the next level, or `Max` at the last one.
    pub cost: String,
    /// What one level gives: `+10 max health per level`.
    pub effect: String,
    pub maxed: bool,
    /// The profile has the coins for the next level (false at the last level).
    pub affordable: bool,
}

/// The rows of the list, in the order the catalog lists the upgrades.
pub fn rows(catalog: &UpgradeCatalog, profile: &Profile) -> Vec<ShopRow> {
    catalog
        .upgrades()
        .iter()
        .map(|upgrade| row(upgrade, catalog, profile))
        .collect()
}

fn row(upgrade: &Upgrade, catalog: &UpgradeCatalog, profile: &Profile) -> ShopRow {
    let next = upgrades::next_cost(profile, catalog, &upgrade.id);
    ShopRow {
        name: upgrade.name.clone(),
        level: format!(
            "{}/{}",
            upgrades::level(profile, &upgrade.id).min(upgrade.max_level()),
            upgrade.max_level()
        ),
        cost: next.map_or_else(|| "Max".to_owned(), |cost| cost.to_string()),
        effect: format!("{} per level", upgrade.effect.text()),
        maxed: next.is_none(),
        affordable: next.is_some_and(|cost| profile.coins >= cost),
    }
}

/// The line over the list: how many coins the player has banked.
pub fn coins_text(profile: &Profile) -> String {
    format!("Banked coins: {}", profile.coins)
}

/// The last result of a purchase, for the message line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShopNote {
    pub text: String,
    /// True for a refusal (shown in a warning color), false for a purchase made.
    pub refusal: bool,
}

/// Whether the shop is open, its cursor, the top of its scrolling window and the last result.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ShopMenu {
    open: bool,
    selected: usize,
    first: usize,
    note: Option<ShopNote>,
}

impl ShopMenu {
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Opens the shop. An old result is not kept for the next time.
    pub fn open(&mut self) {
        self.open = true;
        self.note = None;
    }

    pub fn close(&mut self) {
        self.open = false;
        self.note = None;
    }

    /// The selected upgrade, as an index into the `count` upgrades; none when there is none.
    pub fn selected(&self, count: usize) -> Option<usize> {
        count.checked_sub(1).map(|last| self.selected.min(last))
    }

    /// The upgrade at the top of a list that shows `visible` of `count` rows: where the window
    /// was left, moved only as far as the selection needs.
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

    /// The result of the last purchase attempt.
    pub fn note(&self) -> Option<&ShopNote> {
        self.note.as_ref()
    }

    /// Shows `text` as the message line (for a result the menu itself cannot know, such as a
    /// profile that did not save).
    pub fn set_note(&mut self, text: String, refusal: bool) {
        self.note = Some(ShopNote { text, refusal });
    }

    /// Moves the cursor by `steps` upgrades (negative is up), wrapping around the ends, and
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

    /// Selects upgrade `index` (the mouse resting on its row). An index the list does not have
    /// is ignored. Returns whether the selection changed.
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
            self.note = None;
        }
    }

    /// The selected upgrade of `catalog`, if there is one.
    pub fn selected_upgrade<'a>(&self, catalog: &'a UpgradeCatalog) -> Option<&'a Upgrade> {
        let list = catalog.upgrades();
        list.get(self.selected(list.len())?)
    }

    /// Buys the next level of the selected upgrade with the profile's coins. The result is kept
    /// as the message line: a refusal changes nothing in `profile` and says why, a purchase says
    /// what was bought. None when the catalog lists nothing.
    pub fn buy(&mut self, catalog: &UpgradeCatalog, profile: &mut Profile) -> Option<Purchase> {
        let upgrade = self.selected_upgrade(catalog)?;
        let outcome = upgrades::buy(profile, catalog, &upgrade.id);
        self.note = Some(match &outcome {
            Purchase::Bought => ShopNote {
                text: format!(
                    "Bought the {}: level {}",
                    upgrade.name,
                    upgrades::level(profile, &upgrade.id)
                ),
                refusal: false,
            },
            Purchase::Refused(message) => ShopNote {
                text: message.clone(),
                refusal: true,
            },
        });
        Some(outcome)
    }
}

#[cfg(test)]
mod tests;
