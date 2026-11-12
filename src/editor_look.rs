//! Pure editing of a room's terrain look (`layout.terrain_style`) for the room editor: pick a
//! preset, override one of its colors or its moss, switch the boundary, set or reroll the seed
//! offset. Every edit is on `RoomDraft` and checked against the preset catalog, so a refused
//! edit leaves the draft as it was; `EditorSession::edit_look` marks the session unsaved when an
//! edit changed something. The canvas preview (`LookPreview`) draws the room with the very
//! code the game draws it with, rebuilt only when the look or the terrain changes.
use macroquad::math::Rect;

use crate::editor::{ExitPlacement, RoomDraft, RoomLayout};
use crate::editor_session::EditorSession;
use crate::room::{Exit, RoomId};
use crate::terrain_style::{Look, Rgb, RoomInputs, RoomShapes, TerrainLook};
use crate::terrain_styles::{RoomStyle, TerrainPreset, TerrainStyles};

/// One of the four colors of a look.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookColor {
    Fill,
    Edge,
    Highlight,
    Accent,
}

impl LookColor {
    pub const ALL: [LookColor; 4] = [Self::Fill, Self::Edge, Self::Highlight, Self::Accent];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fill => "Fill",
            Self::Edge => "Edge",
            Self::Highlight => "Highlight",
            Self::Accent => "Accent",
        }
    }

    /// The room's override of this color, if it has one.
    pub fn overridden(self, style: &RoomStyle) -> Option<Rgb> {
        match self {
            Self::Fill => style.fill,
            Self::Edge => style.edge,
            Self::Highlight => style.highlight,
            Self::Accent => style.accent,
        }
    }

    /// This color in `preset`.
    pub fn of_preset(self, preset: &TerrainPreset) -> Rgb {
        match self {
            Self::Fill => preset.fill,
            Self::Edge => preset.edge,
            Self::Highlight => preset.highlight,
            Self::Accent => preset.accent,
        }
    }

    /// This color in a resolved look (the preset's, or the room's override of it).
    pub fn of_look(self, look: &Look) -> Rgb {
        match self {
            Self::Fill => look.palette.fill,
            Self::Edge => look.palette.edge,
            Self::Highlight => look.palette.highlight,
            Self::Accent => look.palette.accent,
        }
    }

    fn slot(self, style: &mut RoomStyle) -> &mut Option<Rgb> {
        match self {
            Self::Fill => &mut style.fill,
            Self::Edge => &mut style.edge,
            Self::Highlight => &mut style.highlight,
            Self::Accent => &mut style.accent,
        }
    }
}

impl RoomDraft {
    /// The look the draft draws with: its style resolved against `styles`. A style that does
    /// not resolve (unknown preset) draws as the default look, the way the game's front end does.
    pub fn look(&self, styles: &TerrainStyles) -> Look {
        styles.resolve_or_default(&self.layout.terrain_style)
    }

    /// The preset named by the style, or the error that says which presets there are.
    fn look_preset<'a>(
        &self,
        styles: &'a TerrainStyles,
        style: &RoomStyle,
    ) -> Result<&'a TerrainPreset, String> {
        match styles.preset(&style.preset) {
            Some(preset) => Ok(preset),
            None => styles
                .resolve(style)
                .and_then(|_| Err("missing preset".into())),
        }
    }

    /// Switches to preset `name`. The preset's own colors and moss apply again, so the room's
    /// color and moss overrides are cleared; the seed offset and the boundary switch stay.
    pub fn set_look_preset(&mut self, styles: &TerrainStyles, name: &str) -> Result<(), String> {
        styles.resolve(&RoomStyle::named(name))?;
        let old = &self.layout.terrain_style;
        self.layout.terrain_style = RoomStyle {
            seed_offset: old.seed_offset,
            draw_boundary: old.draw_boundary,
            ..RoomStyle::named(name)
        };
        Ok(())
    }

    /// Overrides one color of the preset. A color equal to the preset's is no override, and a
    /// color that is not finite or not between 0 and 1 is refused.
    pub fn set_look_color(
        &mut self,
        styles: &TerrainStyles,
        color: LookColor,
        rgb: Rgb,
    ) -> Result<(), String> {
        let mut style = self.layout.terrain_style.clone();
        let preset = self.look_preset(styles, &style)?;
        *color.slot(&mut style) = (rgb != color.of_preset(preset)).then_some(rgb);
        style.validate()?;
        self.layout.terrain_style = style;
        Ok(())
    }

    /// Back to the preset's color.
    pub fn reset_look_color(&mut self, color: LookColor) {
        *color.slot(&mut self.layout.terrain_style) = None;
    }

    /// Switches moss on or off. The same as the preset's is no override.
    pub fn set_look_moss(&mut self, styles: &TerrainStyles, moss: bool) -> Result<(), String> {
        let mut style = self.layout.terrain_style.clone();
        let preset = self.look_preset(styles, &style)?;
        style.moss = (moss != preset.moss).then_some(moss);
        self.layout.terrain_style = style;
        Ok(())
    }

    /// Whether the slabs around the room are drawn.
    pub fn set_draw_boundary(&mut self, draw: bool) {
        self.layout.terrain_style.draw_boundary = draw;
    }

    /// Sets the number mixed into every solid's seed.
    pub fn set_seed_offset(&mut self, seed: u32) {
        self.layout.terrain_style.seed_offset = seed;
    }

    /// Reshuffles the speckles, cracks and moss: the next seed offset.
    pub fn reroll_seed(&mut self) {
        let style = &mut self.layout.terrain_style;
        style.seed_offset = style.seed_offset.wrapping_add(1);
    }
}

impl EditorSession {
    /// The look the open draft draws with.
    pub fn look(&self) -> Look {
        self.draft.look(&self.styles)
    }

    /// Runs a look edit on the draft with the preset catalog, and marks the session unsaved
    /// when the style changed. A refused edit changes nothing.
    pub fn edit_look<R>(
        &mut self,
        edit: impl FnOnce(&mut RoomDraft, &TerrainStyles) -> Result<R, String>,
    ) -> Result<R, String> {
        let before = self.draft.layout.terrain_style.clone();
        let result = edit(&mut self.draft, &self.styles)?;
        if self.draft.layout.terrain_style != before {
            self.dirty = true;
        }
        Ok(result)
    }
}

/// The canvas preview of a room's look: the room's draw list, kept between frames and rebuilt
/// only when the room's terrain, bounds, exits or look change.
#[derive(Debug, Default)]
pub struct LookPreview {
    cache: TerrainLook,
}

impl LookPreview {
    /// The draw list of `layout` as room `room`, built exactly like the game builds it (so the
    /// preview, the test room and the played room agree), in world coordinates.
    pub fn shapes(
        &mut self,
        room: RoomId,
        layout: &RoomLayout,
        styles: &TerrainStyles,
    ) -> RoomShapes<'_> {
        let exits: Vec<Exit> = layout.exits.iter().map(ExitPlacement::exit).collect();
        let solids: Vec<Rect> = layout
            .terrain
            .iter()
            .map(|&(x, y, w, h)| Rect::new(x, y, w, h))
            .collect();
        let look = styles.resolve_or_default(&layout.terrain_style);
        let inputs = RoomInputs {
            room,
            bounds: layout.rect(),
            floor_y: layout.floor_y,
            exits: &exits,
            solids: &solids,
        };
        self.cache.room_shapes(&inputs, &look)
    }

    /// How many times the list has been built (for tests).
    pub fn builds(&self) -> usize {
        self.cache.builds()
    }
}
