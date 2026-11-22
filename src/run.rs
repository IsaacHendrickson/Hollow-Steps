//! The run loop's pure rules: the settings in `data/run.ron`, how a run ends, and how its coins
//! are banked into the profile.
//!
//! No window and no `GameState` here; `game::run_link` drives these. A run is one trip from the
//! hub to the end of the dungeon. It ends when the player dies, when the boss of
//! `final_boss_room` falls, or on Abandon; the run's coins are then banked into the
//! `Profile` (a share of them, `coin_keep_percent`) and a clear raises the difficulty tier.
//! `docs/run-and-profile.md` has the design.
//!
//! `data/run.ron` is embedded; an `assets/run.ron` replaces it whole (`assets::read_override`).
//! With no `hub_room` (the embedded default) none of this runs and the game plays as before.

use macroquad::math::{Rect, Vec2};
use serde::Deserialize;

use crate::assets::read_override;
use crate::profile::Profile;
use crate::room::RoomId;

pub use bonus::{result_lines_with_bonus, CoinBonus};
pub use screen::{
    hud_counter, inventory_coins, result_layout, result_lines, result_title, tier_label,
    tier_label_origin, Counter, ResultLayout, RETURN_HINT, TIER_LABEL_SIZE,
};

pub const FILE_NAME: &str = "run.ron";
const DEFAULTS: &str = include_str!("../data/run.ron");
/// The share of a run's coins that is banked when the file does not say.
pub const DEFAULT_COIN_KEEP_PERCENT: f32 = 100.0;

fn first_room() -> RoomId {
    RoomId::First
}

fn boss_room() -> RoomId {
    RoomId::Boss
}

fn keep_everything() -> f32 {
    DEFAULT_COIN_KEEP_PERCENT
}

/// The rules of the run loop (`data/run.ron`).
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunConfig {
    /// The safe room the player starts in and returns to. `None`: no hub, no runs, and the game
    /// plays as it did before the loop existed.
    #[serde(default)]
    pub hub_room: Option<RoomId>,
    /// The room the game begins in when there is no hub; with a hub, the room its exit leads to.
    #[serde(default = "first_room")]
    pub start_room: RoomId,
    /// Defeating the boss of this room clears the dungeon.
    #[serde(default = "boss_room")]
    pub final_boss_room: RoomId,
    /// How many percent of a run's coins are banked when it ends, 0 to 100.
    #[serde(default = "keep_everything")]
    pub coin_keep_percent: f32,
    /// Where the upgrade shop stands in the hub room (the center of its marker). In the hub, Up
    /// within pickup reach opens it. `None`: no shop. Needs a hub, and must lie inside it.
    #[serde(default)]
    pub shop: Option<(f32, f32)>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self::from_ron(DEFAULTS).expect("valid run defaults")
    }
}

impl RunConfig {
    /// Parses and validates a file's text. A hub may be written as `hub_room: Custom(0)` or
    /// `hub_room: Some(Custom(0))`.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let options = ron::Options::default()
            .with_default_extension(ron::extensions::Extensions::IMPLICIT_SOME);
        let config: Self = options
            .from_str(text)
            .map_err(|error| format!("run: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `run.ron` in `assets` if there is one (it replaces the defaults whole), otherwise the
    /// embedded `data/run.ron`. A file that exists but cannot be read or is invalid is an error
    /// naming it, never a fallback.
    pub fn load(assets: &std::path::Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Self::from_ron(DEFAULTS).map_err(|error| format!("data/run.ron: {error}")),
        }
    }

    /// The coin share is a finite number from 0 to 100, the hub is not the start room, and a
    /// shop has finite coordinates and a hub to stand in (`check_shop` checks where).
    pub fn validate(&self) -> Result<(), String> {
        if !self.coin_keep_percent.is_finite() || !(0.0..=100.0).contains(&self.coin_keep_percent) {
            return Err(format!(
                "coin_keep_percent must be a number from 0 to 100, not {}",
                self.coin_keep_percent
            ));
        }
        if self.hub_room == Some(self.start_room) {
            return Err(format!(
                "hub_room must differ from start_room (both are {:?})",
                self.start_room
            ));
        }
        if let Some((x, y)) = self.shop {
            if !x.is_finite() || !y.is_finite() {
                return Err(format!("shop must be two finite numbers, not ({x}, {y})"));
            }
            if self.hub_room.is_none() {
                return Err("shop needs a hub_room: the shop stands in the hub".to_owned());
            }
        }
        Ok(())
    }

    /// The shop's position, if there is a shop.
    pub fn shop_position(&self) -> Option<Vec2> {
        self.shop.map(|(x, y)| Vec2::new(x, y))
    }

    /// The shop lies inside the hub room, whose bounds are `hub` (its edges count as inside).
    /// Nothing to check without a shop. Run when the game loads, where the rooms are known.
    pub fn check_shop(&self, hub: Rect) -> Result<(), String> {
        let Some((x, y)) = self.shop else {
            return Ok(());
        };
        if !hub.contains(Vec2::new(x, y)) {
            return Err(format!(
                "run.ron: shop ({x}, {y}) is outside the hub room (it spans x {} to {}, y {} to {})",
                hub.x,
                hub.right(),
                hub.y,
                hub.bottom()
            ));
        }
        Ok(())
    }

    /// True when a hub is set, which is what turns the run loop on.
    pub fn has_hub(&self) -> bool {
        self.hub_room.is_some()
    }

    /// Every room this file names is one of `rooms`, so a run cannot start in a room that does
    /// not exist. The error names the room and the file to add it in.
    pub fn check_rooms(&self, rooms: &[RoomId]) -> Result<(), String> {
        let named = [
            ("hub_room", self.hub_room),
            ("start_room", Some(self.start_room)),
            ("final_boss_room", Some(self.final_boss_room)),
        ];
        for (field, room) in named {
            if let Some(room) = room.filter(|room| !rooms.contains(room)) {
                let hint = match room {
                    RoomId::Custom(number) => {
                        format!(" (add assets/rooms/room_{number}.ron or change run.ron)")
                    }
                    _ => " (change run.ron)".to_owned(),
                };
                return Err(format!("run.ron: {field} {room:?} is not a room{hint}"));
            }
        }
        Ok(())
    }
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunOutcome {
    /// The boss of the final boss room was defeated.
    Cleared,
    /// Health ran out.
    Died,
    /// The player gave up (the pause menu, entering the hub, or closing the game).
    Abandoned,
}

/// What a finished run gave the profile, for the result screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RunResult {
    pub outcome: RunOutcome,
    /// Coins the run collected.
    pub coins_earned: u32,
    /// The share of them that went into the profile.
    pub coins_banked: u32,
    pub tier_before: u32,
    pub tier_after: u32,
}

/// Where the player is in the loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunPhase {
    /// In the hub room, between runs.
    Hub,
    /// A run is going.
    Running,
    /// A run just ended; the result screen shows until the player continues.
    Result(RunResult),
}

/// `earned` coins at `keep_percent`: the whole coins of that share, never more than `earned`. A
/// share outside 0 to 100 (infinity included) is clamped into it, and NaN counts as the default.
pub fn kept_coins(earned: u32, keep_percent: f32) -> u32 {
    let percent = if keep_percent.is_nan() {
        f64::from(DEFAULT_COIN_KEEP_PERCENT)
    } else {
        f64::from(keep_percent).clamp(0.0, 100.0)
    };
    let kept = (f64::from(earned) * percent / 100.0).floor();
    (kept as u32).min(earned)
}

impl Profile {
    /// Banks a finished run: `floor(earned * keep_percent / 100)` coins go into the profile,
    /// and a clear raises the tier and the cleared count by one. Dying or abandoning changes
    /// neither. Every addition saturates.
    pub fn bank(&mut self, earned: u32, keep_percent: f32, outcome: RunOutcome) -> RunResult {
        let banked = kept_coins(earned, keep_percent);
        let tier_before = self.tier;
        self.coins = self.coins.saturating_add(banked);
        if outcome == RunOutcome::Cleared {
            self.tier = self.tier.saturating_add(1);
            self.cleared = self.cleared.saturating_add(1);
        }
        RunResult {
            outcome,
            coins_earned: earned,
            coins_banked: banked,
            tier_before,
            tier_after: self.tier,
        }
    }
}

mod bonus;
mod screen;

#[cfg(test)]
mod tests;
