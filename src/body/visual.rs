//! The authored look of one body: template name, palette colors, scale and hurt feedback.
use serde::{Deserialize, Serialize};

use super::template::{BodyTemplates, ColorSlot};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyVisual {
    #[serde(default)]
    pub hurt: crate::pose::HurtFeedback,
    pub template: String,
    pub primary: (f32, f32, f32, f32),
    pub secondary: (f32, f32, f32, f32),
    pub accent: (f32, f32, f32, f32),
    pub eye: (f32, f32, f32, f32),
    #[serde(default = "unit_scale")]
    pub scale: f32,
}

fn unit_scale() -> f32 {
    1.0
}

impl BodyVisual {
    /// Data-backed authoring defaults; the selected catalog name is always explicit.
    pub fn for_template(template: &str) -> Self {
        let mut visual: Self = ron::from_str(include_str!("../../data/body_visual_defaults.ron"))
            .expect("valid body visual defaults");
        visual.template = template.to_owned();
        visual
    }

    pub fn validate(&self) -> Result<(), String> {
        self.hurt
            .validate()
            .map_err(|e| format!("hurt feedback: {e}"))?;
        if self.template.trim().is_empty() {
            return Err("missing body template name".into());
        }
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err("body scale must be finite and positive".into());
        }
        for slot in [
            ColorSlot::Primary,
            ColorSlot::Secondary,
            ColorSlot::Accent,
            ColorSlot::Eye,
        ] {
            let (r, g, b, a) = self.color(slot);
            if [r, g, b, a]
                .iter()
                .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
            {
                return Err(format!(
                    "body {slot:?} color channels must be finite and in 0..=1"
                ));
            }
        }
        Ok(())
    }

    pub fn validate_template(&self, templates: &BodyTemplates) -> Result<(), String> {
        self.validate()?;
        if !templates.contains_key(&self.template) {
            return Err(format!("unknown body template {:?}", self.template));
        }
        Ok(())
    }

    pub fn color(&self, slot: ColorSlot) -> (f32, f32, f32, f32) {
        match slot {
            ColorSlot::Primary => self.primary,
            ColorSlot::Secondary => self.secondary,
            ColorSlot::Accent => self.accent,
            ColorSlot::Eye => self.eye,
        }
    }
}
