//! Post-process settings: a vignette and a soft glow drawn as one pass over the finished
//! world picture.
//!
//! Pure rules, no GPU. `data/postfx.ron` holds the settings (an `assets/postfx.ron`
//! replaces it whole); the front end hands `PostFxConfig::uniforms` to the material built
//! from `FRAGMENT_SHADER`. `vignette_factor` and `glow_weight` are the same formulas the
//! shader runs, so tests can pin what the pass does to a pixel without a window.

use std::path::Path;

use macroquad::math::Vec2;
use serde::Deserialize;

use crate::assets::read_override;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "postfx.ron";
/// Largest glow radius a config may ask for, in pixels of the virtual screen.
pub const MAX_GLOW_RADIUS: f32 = 24.0;
/// Largest glow strength a config may ask for.
pub const MAX_GLOW_STRENGTH: f32 = 2.0;
/// Highest glow threshold a config may ask for. Below 1 so the soft knee has a width.
pub const MAX_GLOW_THRESHOLD: f32 = 0.95;
/// Widest vignette fade a config may ask for.
pub const MAX_VIGNETTE_SOFTNESS: f32 = 2.0;
/// Narrowest vignette fade a config may ask for (a zero width would be a hard edge).
pub const MIN_VIGNETTE_SOFTNESS: f32 = 0.05;

/// Names of the uniforms `FRAGMENT_SHADER` reads, besides the picture itself.
pub const UNIFORM_VIGNETTE: &str = "vignette";
pub const UNIFORM_GLOW: &str = "glow";
pub const UNIFORM_TEXEL: &str = "texel";

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VignetteConfig {
    /// 0 leaves the corners alone, 1 turns them black.
    pub strength: f32,
    /// Distance from the middle (0) to the corners (1) where the darkening starts.
    pub radius: f32,
    /// Width of the fade from untouched to darkest, in the same distance units.
    pub softness: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlowConfig {
    /// How much halo is added to the picture; 0 adds none.
    pub strength: f32,
    /// Brightness a pixel needs before it glows.
    pub threshold: f32,
    /// How far the halo reaches, in virtual-screen pixels.
    pub radius: f32,
}

/// The post-process tunables, from the embedded `data/postfx.ron`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostFxConfig {
    /// The data toggle: false skips the pass entirely.
    pub enabled: bool,
    pub vignette: VignetteConfig,
    pub glow: GlowConfig,
}

/// What the shader is given each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    /// strength, radius, softness
    pub vignette: [f32; 3],
    /// strength, threshold, radius in pixels
    pub glow: [f32; 3],
    /// Size of one pixel of the picture in texture coordinates (1 / width, 1 / height).
    pub texel: [f32; 2],
}

impl Default for PostFxConfig {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/postfx.ron")).expect("valid postfx defaults")
    }
}

impl PostFxConfig {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let config: Self = ron::from_str(text).map_err(|error| format!("postfx: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `postfx.ron` in `assets` if there is one (it replaces the defaults whole),
    /// otherwise the embedded `data/postfx.ron`. A file that exists but cannot be read or is
    /// invalid is an error naming it, never a fallback.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Ok(Self::default()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        let in_range = |name: &str, value: f32, min: f32, max: f32| {
            if value.is_finite() && (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(format!("postfx {name} must be between {min} and {max}"))
            }
        };
        in_range("vignette.strength", self.vignette.strength, 0.0, 1.0)?;
        in_range("vignette.radius", self.vignette.radius, 0.0, 1.0)?;
        in_range(
            "vignette.softness",
            self.vignette.softness,
            MIN_VIGNETTE_SOFTNESS,
            MAX_VIGNETTE_SOFTNESS,
        )?;
        in_range("glow.strength", self.glow.strength, 0.0, MAX_GLOW_STRENGTH)?;
        in_range(
            "glow.threshold",
            self.glow.threshold,
            0.0,
            MAX_GLOW_THRESHOLD,
        )?;
        in_range("glow.radius", self.glow.radius, 0.0, MAX_GLOW_RADIUS)?;
        Ok(())
    }

    /// Whether the pass would change the picture at all. A pass that is switched off, or
    /// whose vignette and glow are both at zero, is skipped.
    pub fn is_active(&self) -> bool {
        self.enabled
            && (self.vignette.strength > 0.0
                || (self.glow.strength > 0.0 && self.glow.radius > 0.0))
    }

    /// The shader inputs for a picture `view` pixels across.
    pub fn uniforms(&self, view: Vec2) -> Uniforms {
        let texel = if view.is_finite() && view.x > 0.0 && view.y > 0.0 {
            [1.0 / view.x, 1.0 / view.y]
        } else {
            [0.0, 0.0]
        };
        Uniforms {
            vignette: [
                self.vignette.strength,
                self.vignette.radius,
                self.vignette.softness,
            ],
            glow: [self.glow.strength, self.glow.threshold, self.glow.radius],
            texel,
        }
    }
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// How much of a pixel's brightness the vignette keeps (1 = all of it). `uv` is the pixel's
/// place in the picture, (0, 0) to (1, 1). The distance is 0 in the middle and 1 in the
/// corners, so the fade follows the picture's shape. Same formula as `FRAGMENT_SHADER`.
pub fn vignette_factor(uv: Vec2, vignette: &VignetteConfig) -> f32 {
    let distance = ((uv - Vec2::splat(0.5)) * 2.0).length() / std::f32::consts::SQRT_2;
    let edge = smoothstep(
        vignette.radius,
        vignette.radius + vignette.softness,
        distance,
    );
    1.0 - vignette.strength * edge
}

/// Brightness of a color, as the glow sees it.
pub fn luma(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// How much of a sampled color goes into the glow: nothing at or below `threshold`, all of
/// it at full brightness, rising smoothly between. Same formula as `FRAGMENT_SHADER`.
pub fn glow_weight(luma: f32, threshold: f32) -> f32 {
    ((luma - threshold) / (1.0 - threshold).max(0.001)).clamp(0.0, 1.0)
}

/// Samples around a pixel for the glow: `GLOW_DIRECTIONS` evenly spaced directions on each of
/// `GLOW_RINGS` rings, every ring turned a third of a step from the one inside it.
pub const GLOW_DIRECTIONS: usize = 8;
/// How many rings of samples the glow takes (at a third, two thirds and all of the radius).
pub const GLOW_RINGS: usize = 3;

/// Vertex shader: the plain macroquad one, passing the texture coordinate through.
pub const VERTEX_SHADER: &str = "#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;

varying lowp vec2 uv;
varying lowp vec4 color;

uniform mat4 Model;
uniform mat4 Projection;

void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
";

/// Fragment shader: glow, then vignette, in one pass. Keep in step with `vignette_factor`
/// and `glow_weight`.
pub const FRAGMENT_SHADER: &str = "#version 100
precision mediump float;

varying vec4 color;
varying vec2 uv;

uniform sampler2D Texture;
// strength, radius, softness
uniform vec3 vignette;
// strength, threshold, radius in pixels
uniform vec3 glow;
// one pixel in texture coordinates
uniform vec2 texel;

vec3 glow_sample(vec2 at) {
    vec3 rgb = texture2D(Texture, at).rgb;
    float luma = dot(rgb, vec3(0.2126, 0.7152, 0.0722));
    float weight = clamp((luma - glow.y) / max(1.0 - glow.y, 0.001), 0.0, 1.0);
    return rgb * weight;
}

void main() {
    vec3 base = texture2D(Texture, uv).rgb;
    vec3 halo = vec3(0.0);
    if (glow.x > 0.0 && glow.z > 0.0) {
        for (int i = 0; i < 8; i++) {
            // Three rings, each turned a third of a step from the last, so the samples do not
            // line up in spokes and a thin bright shape does not leave ghost copies.
            float angle = float(i) * 0.785398;
            vec2 inner = vec2(cos(angle), sin(angle)) * texel * glow.z * 0.34;
            vec2 middle = vec2(cos(angle + 0.261799), sin(angle + 0.261799)) * texel * glow.z * 0.67;
            vec2 outer = vec2(cos(angle + 0.523599), sin(angle + 0.523599)) * texel * glow.z;
            halo += glow_sample(uv + inner);
            halo += glow_sample(uv + middle);
            halo += glow_sample(uv + outer);
        }
        halo /= 24.0;
    }
    vec3 lit = base + halo * glow.x;

    float from_middle = length((uv - vec2(0.5)) * 2.0) / 1.41421356;
    float edge = smoothstep(vignette.y, vignette.y + vignette.z, from_middle);
    lit *= 1.0 - vignette.x * edge;

    gl_FragColor = vec4(lit, 1.0);
}
";

#[cfg(test)]
mod tests;
