//! The colors a look uses, worked out once from a palette.

use super::{Rgb, Rgba, TerrainStyle};

const BLACK: Rgb = (0.0, 0.0, 0.0);
const WHITE: Rgb = (1.0, 1.0, 1.0);

/// Blends `a` toward `b`; `amount` 0 gives `a`, 1 gives `b`.
pub(super) fn mix(a: Rgb, b: Rgb, amount: f32) -> Rgb {
    (
        a.0 + (b.0 - a.0) * amount,
        a.1 + (b.1 - a.1) * amount,
        a.2 + (b.2 - a.2) * amount,
    )
}

/// Every color the draw list uses. The middle band is exactly the palette's fill.
pub(super) struct Colors {
    pub band_top: Rgba,
    pub band_mid: Rgba,
    pub band_low: Rgba,
    pub outline: Rgba,
    pub highlight: Rgba,
    pub lip: Rgba,
    pub speck_light: Rgba,
    pub speck_dark: Rgba,
    pub speck_accent: Rgba,
    pub crack: Rgba,
    pub moss: Rgba,
    pub moss_dark: Rgba,
    pub stalactite: Rgba,
}

impl Colors {
    pub(super) fn from_style(style: &TerrainStyle) -> Self {
        let fill = style.fill;
        Self {
            band_top: Rgba::opaque(mix(fill, WHITE, 0.07)),
            band_mid: Rgba::opaque(fill),
            band_low: Rgba::opaque(mix(fill, BLACK, 0.14)),
            outline: Rgba::opaque(style.edge),
            highlight: Rgba::opaque(style.highlight),
            lip: Rgba::opaque(mix(mix(fill, style.highlight, 0.55), BLACK, 0.05)),
            speck_light: Rgba::new(mix(fill, style.highlight, 0.30), 0.8),
            speck_dark: Rgba::new(mix(fill, style.edge, 0.55), 0.8),
            speck_accent: Rgba::new(style.accent, 0.45),
            crack: Rgba::new(style.edge, 0.6),
            moss: Rgba::opaque(style.accent),
            moss_dark: Rgba::opaque(mix(style.accent, BLACK, 0.3)),
            stalactite: Rgba::opaque(mix(mix(fill, BLACK, 0.14), style.edge, 0.35)),
        }
    }
}
