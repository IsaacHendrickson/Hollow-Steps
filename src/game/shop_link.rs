//! How `GameState` owns the hub's upgrade shop (`crate::shop_menu`, `crate::upgrades`,
//! `docs/run-and-profile.md`): when it can open, the world standing still while it is open,
//! and buying, which saves the profile and puts the new numbers into effect at once.
//!
//! The shop stands at `run.ron`'s `shop` position in the hub room. It opens with the pickup key
//! (Up) within pickup reach (`item::within_reach`), only in the hub: not in a run, not on the
//! result screen and not in a test room. An item or charm in reach takes the press first.
//! While it is open nothing in the world advances (like the inventory) and Escape closes it.
use macroquad::math::Vec2;

use super::GameState;
use crate::item::within_reach;
use crate::shop_menu::{self, ShopLayout, ShopMenu, ShopRow, SHOP_SIZE};
use crate::upgrades::{Purchase, Upgrade, UpgradeCatalog};

impl GameState {
    /// Every upgrade the shop sells. The levels bought are in `profile().upgrades`.
    pub fn upgrade_catalog(&self) -> &UpgradeCatalog {
        &self.upgrades
    }

    /// The shop's cursor and last result, for drawing.
    pub fn shop_menu(&self) -> &ShopMenu {
        &self.shop
    }

    /// True while the shop window is open.
    pub fn shop_open(&self) -> bool {
        self.shop.is_open()
    }

    /// Where the shop stands, while it can be used: in the hub room, with a shop in `run.ron`.
    /// None in a run, on the result screen, in a test room and in every other room.
    pub fn shop_position(&self) -> Option<Vec2> {
        let in_hub_room = self.run_config().hub_room == Some(self.room().id);
        (self.in_hub() && in_hub_room)
            .then(|| self.run_config().shop_position())
            .flatten()
    }

    /// The player stands where the pickup key would open the shop (the scene shows "Up: shop"
    /// by this rule).
    pub fn shop_in_reach(&self) -> bool {
        self.shop_position()
            .is_some_and(|position| within_reach(position, SHOP_SIZE, &self.player))
    }

    /// Opens the shop if it is in reach. Returns whether it opened.
    pub(super) fn open_shop(&mut self) -> bool {
        let opens = self.shop_in_reach();
        if opens {
            self.shop.open();
        }
        opens
    }

    /// Closes the shop (Escape).
    pub fn close_shop(&mut self) {
        self.shop.close();
    }

    /// The rows of the list: name, level, next price, what a level gives.
    pub fn shop_rows(&self) -> Vec<ShopRow> {
        shop_menu::rows(&self.upgrades, self.profile())
    }

    /// The selected upgrade, if the shop sells anything.
    pub fn selected_upgrade(&self) -> Option<&Upgrade> {
        self.shop.selected_upgrade(&self.upgrades)
    }

    /// Where everything in the open shop goes on a `view`-sized screen.
    pub fn shop_layout(&self, view: Vec2) -> ShopLayout {
        let count = self.upgrades.upgrades().len();
        let first = self.shop.first(count, shop_menu::visible_rows(view, count));
        shop_menu::layout(view, count, first)
    }

    /// Up or down in the shop's list (negative is up), wrapping around. Does nothing while the
    /// shop is closed.
    pub fn shop_step(&mut self, steps: i32, view: Vec2) {
        if self.shop.is_open() {
            let count = self.upgrades.upgrades().len();
            self.shop
                .step(count, steps, shop_menu::visible_rows(view, count));
        }
    }

    /// Selects the upgrade at `index` (the mouse resting on its row). Does nothing while the
    /// shop is closed.
    pub fn shop_hover(&mut self, index: usize) {
        if self.shop.is_open() {
            self.shop.select(index, self.upgrades.upgrades().len());
        }
    }

    /// Buys the next level of the selected upgrade. A purchase is paid from the banked coins,
    /// saved with the profile and in effect at once: the player keeps position and timers, and
    /// nothing is refilled (a raised maximum health adds the same health, as everywhere). A
    /// refusal changes nothing and leaves its message in `shop_menu`. None while the shop is
    /// closed or sells nothing.
    pub fn shop_press(&mut self) -> Option<Purchase> {
        if !self.shop.is_open() || !self.in_hub() {
            return None;
        }
        let outcome = self.shop.buy(&self.upgrades, &mut self.run.profile)?;
        if outcome == Purchase::Bought {
            self.apply_stats();
            if let Err(error) = self.save_profile() {
                // The purchase stands for this session; say that the file was not written.
                self.shop.set_note(
                    format!(
                        "Bought, but the profile did not save (kept for this session): {error}"
                    ),
                    true,
                );
            }
        }
        Some(outcome)
    }

    /// A click on the upgrade at `index`: selects it, then buys it.
    pub fn shop_click(&mut self, index: usize) -> Option<Purchase> {
        self.shop_hover(index);
        if index >= self.upgrades.upgrades().len() {
            return None;
        }
        self.shop_press()
    }
}
