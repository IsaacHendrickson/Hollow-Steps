//! The health and mana bars: their settings and where each part of them goes.
//!
//! Pure rules, no drawing. `hud.rs` fills the rectangles `layout_bars` returns. The bars have a
//! fixed on-screen length; health, trailing health, pending heal and mana are only fractions of
//! it, so the picture never depends on how big the numbers are. Tunables are in `data/hud.ron`
//! (an `assets/hud.ron` replaces it whole).

use std::path::Path;

use macroquad::math::{vec2, Rect, Vec2};
use serde::Deserialize;

use crate::assets::read_override;
use crate::camera::{anchored, Anchor};
use crate::game::{VIEW_HEIGHT, VIEW_WIDTH};
use crate::vitals::VitalsState;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "hud.ron";
/// Shortest and longest a bar may be, in virtual-resolution pixels.
pub const MIN_BAR_LENGTH: f32 = 40.0;
pub const MAX_BAR_LENGTH: f32 = VIEW_WIDTH / 2.0;
/// Thinnest and thickest a bar may be.
pub const MIN_BAR_HEIGHT: f32 = 2.0;
pub const MAX_BAR_HEIGHT: f32 = 64.0;
/// Largest gap between the two bars.
pub const MAX_GAP: f32 = 64.0;

/// Red, green, blue, alpha.
pub type Rgba = [u8; 4];

/// The colors of every layer of the bars.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BarColors {
    /// The empty health bar, behind everything else.
    pub empty: Rgba,
    /// Health lost a moment ago, up to the trailing health.
    pub trailing: Rgba,
    /// Health bought with mana but not added yet.
    pub pending: Rgba,
    /// Current health.
    pub health: Rgba,
    pub mana_empty: Rgba,
    pub mana: Rgba,
}

/// The numbers in `data/hud.ron`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HudConfig {
    /// Where the health bar's top-left corner sits, from the top-left corner of the screen.
    pub margin: (f32, f32),
    /// Fixed length of both bars when full.
    pub bar_length: f32,
    pub health_height: f32,
    pub mana_height: f32,
    /// Space between the health bar and the mana bar.
    pub gap: f32,
    pub colors: BarColors,
}

impl Default for HudConfig {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/hud.ron")).expect("valid HUD defaults")
    }
}

impl HudConfig {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let config: Self = ron::from_str(text).map_err(|error| format!("hud: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `hud.ron` in `assets` if there is one (it replaces the defaults whole), otherwise
    /// the embedded `data/hud.ron`. A file that exists but cannot be read or is invalid is an
    /// error naming it, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Ok(Self::default()),
        }
    }

    /// Every size inside its range and both bars on the screen. The trailing and pending
    /// colors must differ from the health color, or those layers could not be told apart.
    pub fn validate(&self) -> Result<(), String> {
        let check = |what: &str, value: f32, min: f32, max: f32| {
            if value.is_finite() && (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(format!("hud {what} must be between {min} and {max}"))
            }
        };
        check("margin.x", self.margin.0, 0.0, VIEW_WIDTH)?;
        check("margin.y", self.margin.1, 0.0, VIEW_HEIGHT)?;
        check(
            "bar_length",
            self.bar_length,
            MIN_BAR_LENGTH,
            MAX_BAR_LENGTH,
        )?;
        check(
            "health_height",
            self.health_height,
            MIN_BAR_HEIGHT,
            MAX_BAR_HEIGHT,
        )?;
        check(
            "mana_height",
            self.mana_height,
            MIN_BAR_HEIGHT,
            MAX_BAR_HEIGHT,
        )?;
        check("gap", self.gap, 0.0, MAX_GAP)?;
        if self.margin.0 + self.bar_length > VIEW_WIDTH {
            return Err(format!(
                "hud margin.x plus bar_length must fit in the {VIEW_WIDTH} pixel wide screen"
            ));
        }
        if self.margin.1 + self.health_height + self.gap + self.mana_height > VIEW_HEIGHT {
            return Err(format!(
                "hud margin.y plus both bars and the gap must fit in the {VIEW_HEIGHT} pixel \
                 high screen"
            ));
        }
        if self.colors.trailing == self.colors.health {
            return Err("hud colors.trailing must differ from colors.health".into());
        }
        if self.colors.pending == self.colors.health {
            return Err("hud colors.pending must differ from colors.health".into());
        }
        Ok(())
    }

    /// The whole empty health bar on a `view`-sized screen: the rectangle the other parts fit
    /// into. Anchored top left, so it depends on nothing but the view and these settings.
    pub fn health_anchor(&self, view: Vec2) -> Rect {
        let size = vec2(self.bar_length, self.health_height);
        let top_left = anchored(
            view,
            Anchor::TopLeft,
            size,
            vec2(self.margin.0, self.margin.1),
        );
        Rect::new(top_left.x, top_left.y, size.x, size.y)
    }
}

/// What the bars show, as plain numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarValues {
    pub health: f32,
    pub max_health: f32,
    /// Where health was before recent damage.
    pub trailing_health: f32,
    /// Health bought with mana, not added yet.
    pub pending_heal: f32,
    pub mana: f32,
    pub max_mana: f32,
}

impl From<&VitalsState> for BarValues {
    fn from(vitals: &VitalsState) -> Self {
        Self {
            health: vitals.health(),
            max_health: vitals.max_health(),
            trailing_health: vitals.trailing_health(),
            pending_heal: vitals.pending_heal(),
            mana: vitals.mana(),
            max_mana: vitals.max_mana(),
        }
    }
}

/// The rectangle of every layer, back to front on the health bar. A part with nothing in it
/// has zero width.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLayout {
    /// The whole empty health bar.
    pub empty: Rect,
    /// From the left end to the trailing health (never shorter than `health`).
    pub trailing: Rect,
    /// From where `health` ends to `health + pending_heal`, cut off at the end of the bar.
    pub pending: Rect,
    /// From the left end to the current health.
    pub health: Rect,
    /// The whole empty mana bar, directly under the health bar.
    pub mana_empty: Rect,
    pub mana: Rect,
}

impl BarLayout {
    /// Both bars together, for placing things under them.
    pub fn bounds(&self) -> Rect {
        Rect::new(
            self.empty.x,
            self.empty.y,
            self.empty.w,
            self.mana_empty.bottom() - self.empty.y,
        )
    }

    /// The health layers moved by `offset` (a shake); the mana bar stays where it is.
    pub fn with_health_offset(mut self, offset: Vec2) -> Self {
        self.empty = self.empty.offset(offset);
        self.trailing = self.trailing.offset(offset);
        self.pending = self.pending.offset(offset);
        self.health = self.health.offset(offset);
        self
    }
}

/// `value` as a share of `max`, from 0 to 1. A zero, negative or NaN max (or a NaN value) is 0.
pub fn fraction(value: f32, max: f32) -> f32 {
    if max.is_nan() || max <= 0.0 {
        return 0.0;
    }
    let share = value / max;
    if share.is_nan() {
        0.0
    } else {
        share.clamp(0.0, 1.0)
    }
}

/// Splits the health bar `anchor` into its layers for `values`, and puts the mana bar under it.
///
/// Every width is `anchor.w` times a fraction of the maximum, so the bars keep their length
/// whatever the maximums are. The mana bar is as long as the health bar and starts `gap`
/// below it.
pub fn layout_bars(config: &HudConfig, values: BarValues, anchor: Rect) -> BarLayout {
    let part = |bar: Rect, share: f32| Rect::new(bar.x, bar.y, bar.w * share, bar.h);
    let health = fraction(values.health, values.max_health);
    let trailing = fraction(values.trailing_health, values.max_health).max(health);
    let pending_heal = if values.pending_heal.is_nan() {
        0.0
    } else {
        values.pending_heal.max(0.0)
    };
    let pending_end = fraction(values.health + pending_heal, values.max_health).max(health);
    let mana_empty = Rect::new(
        anchor.x,
        anchor.bottom() + config.gap,
        anchor.w,
        config.mana_height,
    );
    BarLayout {
        empty: anchor,
        trailing: part(anchor, trailing),
        pending: Rect::new(
            anchor.x + anchor.w * health,
            anchor.y,
            anchor.w * (pending_end - health),
            anchor.h,
        ),
        health: part(anchor, health),
        mana_empty,
        mana: part(mana_empty, fraction(values.mana, values.max_mana)),
    }
}

#[cfg(test)]
mod tests;
