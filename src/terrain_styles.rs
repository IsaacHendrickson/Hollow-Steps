//! The terrain look presets and a room's choice among them.
//!
//! `data/terrain_styles.ron` (embedded in the program) lists the presets: a palette, whether
//! moss grows, how many speckles and cracks the rock gets and how thick the boundary slabs
//! are. A `terrain_styles.ron` in the assets folder replaces the embedded file whole, the way
//! `postfx.ron` does.
//!
//! A room's `layout.terrain_style` is a `RoomStyle`: the name of a preset (`stone` when it names
//! none) plus what the room changes about it, the four colors and the moss switch, a seed
//! offset and whether the boundary is drawn. `TerrainStyles::resolve` is preset plus overrides:
//! the room's own value wins, the preset fills in the rest, and the result is the `Look` the
//! terrain is drawn with. A name the catalog does not have is an error that lists the known
//! presets.
//!
//! Pure data and math: nothing here draws.

use std::path::Path;

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::assets::read_override;
use crate::terrain_style::{
    Look, Rgb, TerrainStyle, DEFAULT_ACCENT, DEFAULT_EDGE, DEFAULT_FILL, DEFAULT_HIGHLIGHT,
};

#[cfg(test)]
mod style_tests;
#[cfg(test)]
mod tests;

/// Name of the optional file in the assets folder that replaces the embedded presets.
pub const FILE_NAME: &str = "terrain_styles.ron";
/// The preset a room uses when its style names none.
pub const DEFAULT_PRESET: &str = "stone";
/// Most a preset may scale the speckles or the cracks (the per-solid caps hold regardless).
pub const MAX_DENSITY: f32 = 10.0;
/// Thickest boundary a preset may ask for, in pixels.
pub const MAX_BOUNDARY_THICKNESS: f32 = 512.0;

/// One named look: the palette and numbers a room gets from choosing it.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainPreset {
    pub name: String,
    pub fill: Rgb,
    pub edge: Rgb,
    pub highlight: Rgb,
    pub accent: Rgb,
    pub moss: bool,
    pub speckle_density: f32,
    pub crack_density: f32,
    pub boundary_thickness: f32,
}

impl TerrainPreset {
    /// Checks the name, that every color is finite with each part between 0 and 1, that the
    /// densities lie between 0 and `MAX_DENSITY` and that the thickness is positive and at
    /// most `MAX_BOUNDARY_THICKNESS`.
    pub fn validate(&self) -> Result<(), String> {
        let name = &self.name;
        if name.trim().is_empty() {
            return Err("terrain_styles: every preset needs a name".into());
        }
        for (field, (red, green, blue)) in [
            ("fill", self.fill),
            ("edge", self.edge),
            ("highlight", self.highlight),
            ("accent", self.accent),
        ] {
            if ![red, green, blue]
                .iter()
                .all(|part| part.is_finite() && (0.0..=1.0).contains(part))
            {
                return Err(format!(
                    "terrain_styles {name:?}: {field} red, green and blue must be finite and between 0 and 1"
                ));
            }
        }
        for (field, value) in [
            ("speckle_density", self.speckle_density),
            ("crack_density", self.crack_density),
        ] {
            if !(value.is_finite() && (0.0..=MAX_DENSITY).contains(&value)) {
                return Err(format!(
                    "terrain_styles {name:?}: {field} must be between 0 and {MAX_DENSITY}"
                ));
            }
        }
        let thickness = self.boundary_thickness;
        if !(thickness.is_finite() && thickness > 0.0 && thickness <= MAX_BOUNDARY_THICKNESS) {
            return Err(format!(
                "terrain_styles {name:?}: boundary_thickness must be more than 0 and at most {MAX_BOUNDARY_THICKNESS}"
            ));
        }
        Ok(())
    }
}

/// The presets rooms choose from, in file order.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TerrainStyles {
    presets: Vec<TerrainPreset>,
}

impl Default for TerrainStyles {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/terrain_styles.ron"))
            .expect("valid terrain style defaults")
    }
}

impl TerrainStyles {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let styles: Self =
            ron::from_str(text).map_err(|error| format!("terrain_styles: {error}"))?;
        styles.validate()?;
        Ok(styles)
    }

    /// The `terrain_styles.ron` in `assets` if there is one (it replaces the embedded presets
    /// whole), otherwise the embedded `data/terrain_styles.ron`. A file that exists but cannot
    /// be read or is invalid is an error naming it, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Ok(Self::default()),
        }
    }

    /// At least one preset, each valid, no name used twice.
    pub fn validate(&self) -> Result<(), String> {
        if self.presets.is_empty() {
            return Err("terrain_styles: at least one preset is needed".into());
        }
        for (index, preset) in self.presets.iter().enumerate() {
            preset.validate()?;
            if self.presets[..index].iter().any(|p| p.name == preset.name) {
                return Err(format!(
                    "terrain_styles: the preset name {:?} is used twice",
                    preset.name
                ));
            }
        }
        Ok(())
    }

    pub fn presets(&self) -> &[TerrainPreset] {
        &self.presets
    }

    /// The preset names, in file order.
    pub fn names(&self) -> Vec<&str> {
        self.presets.iter().map(|p| p.name.as_str()).collect()
    }

    pub fn preset(&self, name: &str) -> Option<&TerrainPreset> {
        self.presets.iter().find(|p| p.name == name)
    }

    /// The look a room gets: its preset, with the room's own colors and moss switch winning
    /// over the preset's, and the room's seed offset and boundary switch. An unknown preset or
    /// a bad override color is an error; the unknown-preset message lists the known names.
    pub fn resolve(&self, style: &RoomStyle) -> Result<Look, String> {
        let preset = self.preset(&style.preset).ok_or_else(|| {
            format!(
                "terrain_style: unknown preset {:?}; known presets: {}",
                style.preset,
                self.names().join(", ")
            )
        })?;
        style.validate()?;
        Ok(Look {
            palette: TerrainStyle {
                fill: style.fill.unwrap_or(preset.fill),
                edge: style.edge.unwrap_or(preset.edge),
                highlight: style.highlight.unwrap_or(preset.highlight),
                accent: style.accent.unwrap_or(preset.accent),
                seed_offset: style.seed_offset,
                moss: style.moss.unwrap_or(preset.moss),
            },
            speckle_density: preset.speckle_density,
            crack_density: preset.crack_density,
            boundary_thickness: preset.boundary_thickness,
            draw_boundary: style.draw_boundary,
        })
    }

    /// `resolve`, but a style that cannot be resolved gets the built-in stone look with the
    /// room's seed offset and boundary switch. Rooms are checked when they load, so this is
    /// for drawing code that has nowhere to report an error.
    pub fn resolve_or_default(&self, style: &RoomStyle) -> Look {
        self.resolve(style).unwrap_or_else(|_| Look {
            palette: TerrainStyle {
                seed_offset: style.seed_offset,
                ..TerrainStyle::default()
            },
            draw_boundary: style.draw_boundary,
            ..Look::default()
        })
    }
}

/// A room's terrain look as authored: the room file's `layout.terrain_style`. Every field is
/// optional there and falls back to its default, so older room files load unchanged and a style
/// names only what it changes.
///
/// A color or the moss switch may be written bare (`fill: (0.2, 0.3, 0.4)`, the way a palette
/// used to be) or as RON's own `Some(..)` and `None`. They are saved bare, and left out when
/// the room does not override them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RoomStyle {
    /// A name from `TerrainStyles`.
    pub preset: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "bare",
        deserialize_with = "color"
    )]
    pub fill: Option<Rgb>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "bare",
        deserialize_with = "color"
    )]
    pub edge: Option<Rgb>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "bare",
        deserialize_with = "color"
    )]
    pub highlight: Option<Rgb>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "bare",
        deserialize_with = "color"
    )]
    pub accent: Option<Rgb>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "bare",
        deserialize_with = "flag"
    )]
    pub moss: Option<bool>,
    /// Mixed into every solid's seed: change it to reshuffle a room's speckles and moss.
    #[serde(skip_serializing_if = "is_zero")]
    pub seed_offset: u32,
    /// Whether the slabs around the room are drawn.
    #[serde(skip_serializing_if = "is_true")]
    pub draw_boundary: bool,
}

impl Default for RoomStyle {
    fn default() -> Self {
        Self {
            preset: DEFAULT_PRESET.to_owned(),
            fill: None,
            edge: None,
            highlight: None,
            accent: None,
            moss: None,
            seed_offset: 0,
            draw_boundary: true,
        }
    }
}

impl RoomStyle {
    /// A style that picks `preset` and changes nothing else.
    pub fn named(preset: &str) -> Self {
        Self {
            preset: preset.to_owned(),
            ..Self::default()
        }
    }

    /// True for the style a room gets without one (it is left out of saved room files).
    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Checks that every override color is finite with each part between 0 and 1. Whether the
    /// preset exists is the catalog's business (`TerrainStyles::resolve`).
    pub fn validate(&self) -> Result<(), String> {
        TerrainStyle {
            fill: self.fill.unwrap_or(DEFAULT_FILL),
            edge: self.edge.unwrap_or(DEFAULT_EDGE),
            highlight: self.highlight.unwrap_or(DEFAULT_HIGHLIGHT),
            accent: self.accent.unwrap_or(DEFAULT_ACCENT),
            ..TerrainStyle::default()
        }
        .validate()
    }
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

fn is_true(value: &bool) -> bool {
    *value
}

/// Writes an override as the plain value (the field is skipped when there is none).
fn bare<S, T>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
    T: Serialize,
{
    match value {
        Some(value) => value.serialize(serializer),
        None => serializer.serialize_none(),
    }
}

/// Either spelling of an optional value: RON's `Some(..)` and `None`, or the bare value.
#[derive(Deserialize)]
#[serde(untagged)]
enum Either<T> {
    Wrapped(Option<T>),
    Bare(T),
}

/// Reads an override in either spelling. The error says what was expected, because the
/// untagged enum's own message ("did not match any variant") says nothing useful.
fn lenient<'de, D, T>(deserializer: D, expected: &str) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    match Either::<T>::deserialize(deserializer) {
        Ok(Either::Wrapped(value)) => Ok(value),
        Ok(Either::Bare(value)) => Ok(Some(value)),
        Err(_) => Err(D::Error::custom(format!("expected {expected}"))),
    }
}

fn color<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Rgb>, D::Error> {
    lenient(
        deserializer,
        "a color like (0.2, 0.4, 0.6) with each part 0 to 1, or None",
    )
}

fn flag<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<bool>, D::Error> {
    lenient(deserializer, "true or false, or None")
}
