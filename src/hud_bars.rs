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
/// Shortest and longest the health bar may shake after a hit, in seconds.
pub const MIN_SHAKE_SECONDS: f32 = 0.05;
pub const MAX_SHAKE_SECONDS: f32 = 2.0;
/// Fastest and slowest the shake may swing, in swings per second.
pub const MIN_SHAKE_FREQUENCY_HZ: f32 = 1.0;
pub const MAX_SHAKE_FREQUENCY_HZ: f32 = 60.0;
/// Largest shake distance, and largest distance a hit on all of max health asks for, in pixels.
pub const MAX_SHAKE_AMPLITUDE: f32 = 32.0;
pub const MAX_SHAKE_AMPLITUDE_PER_MAX_HEALTH: f32 = 320.0;

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
    /// The divider where capacity added by a charm starts, on either bar (the maximum is above
    /// the base). A file without it gets the shipped color.
    #[serde(default = "default_extension_edge")]
    pub extension_edge: Rgba,
    /// Capacity a charm took away, at the end of either bar (the maximum is below the base):
    /// greyed out and never filled. A file without it gets the shipped color.
    #[serde(default = "default_locked")]
    pub locked: Rgba,
}

/// The shipped `colors.extension_edge` and `colors.locked` (the same as `data/hud.ron`; a test
/// keeps them equal), for a `hud.ron` written before they existed.
fn default_extension_edge() -> Rgba {
    [240, 236, 214, 255]
}

fn default_locked() -> Rgba {
    [62, 62, 72, 255]
}

/// How the health bar shakes when the player is hurt. See `hud_damage::BarShake`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DamageShake {
    /// How long a shake takes to fade to nothing.
    pub seconds: f32,
    /// Swings per second.
    pub frequency_hz: f32,
    /// Peak distance in pixels for a hit that takes all of max health; a hit asks for the
    /// share of it that it took.
    pub amplitude_per_max_health: f32,
    /// Smallest and largest peak distance, in pixels, whatever the hit.
    pub min_amplitude: f32,
    pub max_amplitude: f32,
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
    /// The health bar's shake when the player takes damage.
    pub damage_shake: DamageShake,
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

    /// Every size and shake value inside its range and both bars on the screen. The trailing
    /// and pending colors must differ from the health color, or those layers could not be told
    /// apart.
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
        let shake = &self.damage_shake;
        check(
            "damage_shake.seconds",
            shake.seconds,
            MIN_SHAKE_SECONDS,
            MAX_SHAKE_SECONDS,
        )?;
        check(
            "damage_shake.frequency_hz",
            shake.frequency_hz,
            MIN_SHAKE_FREQUENCY_HZ,
            MAX_SHAKE_FREQUENCY_HZ,
        )?;
        check(
            "damage_shake.amplitude_per_max_health",
            shake.amplitude_per_max_health,
            0.0,
            MAX_SHAKE_AMPLITUDE_PER_MAX_HEALTH,
        )?;
        check(
            "damage_shake.min_amplitude",
            shake.min_amplitude,
            0.0,
            MAX_SHAKE_AMPLITUDE,
        )?;
        check(
            "damage_shake.max_amplitude",
            shake.max_amplitude,
            0.0,
            MAX_SHAKE_AMPLITUDE,
        )?;
        if shake.min_amplitude > shake.max_amplitude {
            return Err(
                "hud damage_shake.min_amplitude must not be above damage_shake.max_amplitude"
                    .into(),
            );
        }
        if self.colors.trailing == self.colors.health {
            return Err("hud colors.trailing must differ from colors.health".into());
        }
        if self.colors.pending == self.colors.health {
            return Err("hud colors.pending must differ from colors.health".into());
        }
        for (name, color) in [
            ("extension_edge", self.colors.extension_edge),
            ("locked", self.colors.locked),
        ] {
            if color == self.colors.empty || color == self.colors.mana_empty {
                return Err(format!(
                    "hud colors.{name} must differ from colors.empty and colors.mana_empty"
                ));
            }
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
    /// Maximum health at the baseline (the value from the Player panel), before charms and
    /// permanent upgrades. The bar's scale follows the larger of this and `max_health`; see
    /// `layout_bars`.
    pub base_max_health: f32,
    /// Maximum mana at the baseline.
    pub base_max_mana: f32,
}

impl BarValues {
    /// The same values measured against base maximums: what the bars show for maximums that
    /// charms or permanent upgrades have raised or lowered.
    pub fn with_base(mut self, base_max_health: f32, base_max_mana: f32) -> Self {
        self.base_max_health = base_max_health;
        self.base_max_mana = base_max_mana;
        self
    }
}

/// With no other base given, the base maximums are the maximums themselves: nothing added,
/// nothing locked.
impl From<&VitalsState> for BarValues {
    fn from(vitals: &VitalsState) -> Self {
        Self {
            health: vitals.health(),
            max_health: vitals.max_health(),
            trailing_health: vitals.trailing_health(),
            pending_heal: vitals.pending_heal(),
            mana: vitals.mana(),
            max_mana: vitals.max_mana(),
            base_max_health: vitals.max_health(),
            base_max_mana: vitals.max_mana(),
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
    /// The part of the health bar that is the base maximum, from the left end. The whole bar
    /// unless the maximum is above the base.
    pub health_base: Rect,
    /// Capacity added above the base, after the base part; zero width when there is none.
    pub health_extension: Rect,
    /// Capacity taken below the base, at the end of the bar, never filled; zero width when
    /// there is none.
    pub health_locked: Rect,
    /// The divider line at the start of the extension; zero width when there is none.
    pub health_edge: Rect,
    /// The same four parts of the mana bar.
    pub mana_base: Rect,
    pub mana_extension: Rect,
    pub mana_locked: Rect,
    pub mana_edge: Rect,
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
        self.health_base = self.health_base.offset(offset);
        self.health_extension = self.health_extension.offset(offset);
        self.health_locked = self.health_locked.offset(offset);
        self.health_edge = self.health_edge.offset(offset);
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

/// Maximums closer than this count as equal (float noise from adding up charm costs).
const SAME_MAX: f32 = 1e-3;
/// Width of the divider line at the start of added capacity, in virtual-resolution pixels.
pub const EDGE_WIDTH: f32 = 2.0;

/// A maximum as a length: finite and positive, else zero.
fn sane(max: f32) -> f32 {
    if max.is_finite() && max > 0.0 {
        max
    } else {
        0.0
    }
}

/// How a bar of `max` units is laid out against its `base` units, as shares of the whole bar.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Scale {
    /// Share of the bar the whole of `max` takes: 1, or less when `max` is below the base and
    /// the end is locked. A value's share of the bar is its share of `max` times this.
    fill: f32,
    /// The base part, the added capacity after it and the locked capacity at the end.
    base: f32,
    extension: f32,
    locked: f32,
}

/// The bar scale is its length over the larger of `base` and `max`. With `max` equal to `base`
/// (or both unusable) the whole bar is the base part, as it was before charms.
fn scale(max: f32, base: f32) -> Scale {
    let (max, base) = (sane(max), sane(base));
    let total = max.max(base);
    if total <= 0.0 || (max - base).abs() <= SAME_MAX {
        Scale {
            fill: 1.0,
            base: 1.0,
            extension: 0.0,
            locked: 0.0,
        }
    } else if max > base {
        Scale {
            fill: 1.0,
            base: base / total,
            extension: (max - base) / total,
            locked: 0.0,
        }
    } else {
        Scale {
            fill: max / total,
            base: 1.0,
            extension: 0.0,
            locked: (base - max) / total,
        }
    }
}

/// The part of the health bar `anchor` that can hold health: all of it, unless the maximum is
/// below the base, when the locked end is cut off. Damage particles measure against this.
pub fn fillable_bar(anchor: Rect, max: f32, base: f32) -> Rect {
    Rect::new(
        anchor.x,
        anchor.y,
        anchor.w * scale(max, base).fill,
        anchor.h,
    )
}

/// The base part, the extension, the locked end and the divider of `bar`.
fn sections(bar: Rect, scale: Scale) -> [Rect; 4] {
    let part = |from: f32, share: f32| Rect::new(from, bar.y, bar.w * share, bar.h);
    let boundary = bar.x + bar.w * scale.base;
    let edge = if scale.extension > 0.0 {
        let left = (boundary - EDGE_WIDTH / 2.0).max(bar.x);
        let right = (boundary + EDGE_WIDTH / 2.0).min(bar.right());
        Rect::new(left, bar.y, (right - left).max(0.0), bar.h)
    } else {
        Rect::new(bar.right(), bar.y, 0.0, bar.h)
    };
    [
        part(bar.x, scale.base),
        part(boundary, scale.extension),
        part(bar.right() - bar.w * scale.locked, scale.locked),
        edge,
    ]
}

/// Splits the health bar `anchor` into its layers for `values`, and puts the mana bar under it.
///
/// The bars keep their length (`anchor.w`) whatever the maximums are. A value's width is its
/// value times `anchor.w / max(base maximum, maximum)`, from the left end. With the maximum
/// above the base (a charm or a permanent upgrade added capacity) the base part comes first and
/// the added capacity follows it at the end, behind a divider line, so the base part gets shorter
/// as capacity is added; with the maximum below the base (a charm took
/// capacity away) the base scale stays and the lost capacity at the end is locked, and nothing
/// fills it. With the maximum equal to the base the layout is the plain proportional one. Both
/// bars follow the same rule, each against its own base. The mana bar is as long as the health
/// bar and starts `gap` below it.
pub fn layout_bars(config: &HudConfig, values: BarValues, anchor: Rect) -> BarLayout {
    let part = |bar: Rect, share: f32| Rect::new(bar.x, bar.y, bar.w * share, bar.h);
    let health_scale = scale(values.max_health, values.base_max_health);
    let mana_scale = scale(values.max_mana, values.base_max_mana);
    let fill = health_scale.fill;
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
    let [health_base, health_extension, health_locked, health_edge] =
        sections(anchor, health_scale);
    let [mana_base, mana_extension, mana_locked, mana_edge] = sections(mana_empty, mana_scale);
    BarLayout {
        empty: anchor,
        trailing: part(anchor, trailing * fill),
        pending: Rect::new(
            anchor.x + anchor.w * (health * fill),
            anchor.y,
            anchor.w * (pending_end * fill - health * fill),
            anchor.h,
        ),
        health: part(anchor, health * fill),
        mana_empty,
        mana: part(
            mana_empty,
            fraction(values.mana, values.max_mana) * mana_scale.fill,
        ),
        health_base,
        health_extension,
        health_locked,
        health_edge,
        mana_base,
        mana_extension,
        mana_locked,
        mana_edge,
    }
}

#[cfg(test)]
mod tests;
