//! Macroquad front end for the post-process pass.
//!
//! `PostPass` draws the finished world picture through the vignette-and-glow material into a
//! second render target of the same size, and hands that target back so the HUD is drawn on
//! top of it, crisp and untouched. The settings and the shader's formulas live in
//! `hollow_steps::postfx`.

use std::path::Path;

use hollow_steps::postfx::{
    PostFxConfig, FRAGMENT_SHADER, UNIFORM_GLOW, UNIFORM_TEXEL, UNIFORM_VIGNETTE, VERTEX_SHADER,
};
use macroquad::miniquad::{ShaderSource, UniformDesc, UniformType};
use macroquad::prelude::*;

use crate::startup::screen_camera;

pub struct PostPass {
    config: PostFxConfig,
    /// `None` when the shader did not build (the reason was reported at load): the picture
    /// then goes to the window unprocessed.
    material: Option<Material>,
    output: RenderTarget,
}

impl PostPass {
    /// Builds the pass for a picture `view` pixels across. Invalid settings fall back to the
    /// built-in defaults, and a shader that does not build turns the pass off; the reasons
    /// come back so they can be shown on screen.
    pub fn load(assets: &Path, view: Vec2) -> (Self, Option<String>) {
        let mut errors = Vec::new();
        let config = PostFxConfig::load(assets).unwrap_or_else(|error| {
            errors.push(error);
            PostFxConfig::default()
        });
        let material = match load_material(
            ShaderSource::Glsl {
                vertex: VERTEX_SHADER,
                fragment: FRAGMENT_SHADER,
            },
            MaterialParams {
                uniforms: vec![
                    UniformDesc::new(UNIFORM_VIGNETTE, UniformType::Float3),
                    UniformDesc::new(UNIFORM_GLOW, UniformType::Float3),
                    UniformDesc::new(UNIFORM_TEXEL, UniformType::Float2),
                ],
                ..Default::default()
            },
        ) {
            Ok(material) => Some(material),
            Err(error) => {
                errors.push(format!("post-process shader did not build: {error}"));
                None
            }
        };
        let output = render_target(view.x as u32, view.y as u32);
        output.texture.set_filter(FilterMode::Linear);
        let error = (!errors.is_empty()).then(|| errors.join("\n"));
        (
            Self {
                config,
                material,
                output,
            },
            error,
        )
    }

    /// Re-reads the settings. Nothing changes if they fail to load.
    pub fn reload(&mut self, assets: &Path) -> Result<(), String> {
        self.config = PostFxConfig::load(assets)?;
        Ok(())
    }

    /// Runs `input` through the pass and returns the target to keep drawing on (the HUD goes
    /// there). With the pass off, or its shader missing, that is `input` itself and nothing
    /// is drawn, so the frame is exactly what it was without the pass.
    pub fn process(&self, input: &RenderTarget, view: Vec2) -> RenderTarget {
        let Some(material) = self.material.as_ref().filter(|_| self.config.is_active()) else {
            return input.clone();
        };
        set_camera(&screen_camera(
            Rect::new(0.0, 0.0, view.x, view.y),
            Some(self.output.clone()),
        ));
        clear_background(BLACK);
        gl_use_material(material);
        let uniforms = self.config.uniforms(view);
        material.set_uniform(UNIFORM_VIGNETTE, uniforms.vignette);
        material.set_uniform(UNIFORM_GLOW, uniforms.glow);
        material.set_uniform(UNIFORM_TEXEL, uniforms.texel);
        draw_texture_ex(
            &input.texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(view),
                // Render targets come out upside down otherwise, as in `startup::present`.
                flip_y: true,
                ..Default::default()
            },
        );
        gl_use_default_material();
        self.output.clone()
    }
}
