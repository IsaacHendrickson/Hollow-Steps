//! Labeled scalar controls for every registry field, including nested attack data.
use crate::enemy_attack::{
    Aim, AttackBox, AttackFeedback, AttackShape, AttackSize, AttackVector, OptionalAttack,
};

pub(crate) trait EditorParameter: Copy {
    const LABELS: &'static [&'static str];
    fn editor_values(self) -> Vec<String>;
    fn edit(&mut self, index: usize, text: &str) -> Result<(), String>;
    fn editor_labels(label: &str) -> Vec<String> {
        Self::LABELS
            .iter()
            .map(|suffix| {
                if suffix.is_empty() {
                    label.into()
                } else {
                    format!("{label}: {suffix}")
                }
            })
            .collect()
    }
}
fn parse<T: std::str::FromStr>(text: &str) -> Result<T, String>
where
    T::Err: std::fmt::Display,
{
    text.trim().parse().map_err(|e| format!("{e}"))
}
macro_rules! scalar {
    ($($ty:ty),+) => { $(impl EditorParameter for $ty {
        const LABELS: &'static [&'static str] = &[""];
        fn editor_values(self) -> Vec<String> { vec![self.to_string()] }
        fn edit(&mut self, index: usize, text: &str) -> Result<(), String> {
            if index != 0 { return Err("missing parameter".into()); }
            *self = parse(text)?; Ok(())
        }
    })+ };
}
scalar!(f32, i32, u8, u16, Aim);

impl EditorParameter for AttackBox {
    const LABELS: &'static [&'static str] = &[
        "shape (Rect or Circle)",
        "offset x (pixels)",
        "offset y (pixels)",
        "width / circle diameter (pixels)",
        "height (Rect, pixels)",
        "damage (integer, at least 0)",
        "knockback x (pixels/second)",
        "knockback y (pixels/second)",
        "active duration (seconds)",
    ];
    fn editor_values(self) -> Vec<String> {
        vec![
            match self.shape {
                AttackShape::Rect => "Rect",
                AttackShape::Circle => "Circle",
            }
            .into(),
            self.offset.0.to_string(),
            self.offset.1.to_string(),
            self.size.0.to_string(),
            self.size.1.to_string(),
            self.damage.to_string(),
            self.knockback.0.to_string(),
            self.knockback.1.to_string(),
            self.active_secs.to_string(),
        ]
    }
    fn edit(&mut self, index: usize, text: &str) -> Result<(), String> {
        match index {
            0 => {
                self.shape = match text.trim() {
                    "Rect" => AttackShape::Rect,
                    "Circle" => AttackShape::Circle,
                    _ => return Err("expected Rect or Circle".into()),
                }
            }
            1 => self.offset.0 = parse(text)?,
            2 => self.offset.1 = parse(text)?,
            3 => self.size.0 = parse(text)?,
            4 => self.size.1 = parse(text)?,
            5 => self.damage = parse(text)?,
            6 => self.knockback.0 = parse(text)?,
            7 => self.knockback.1 = parse(text)?,
            8 => self.active_secs = parse(text)?,
            _ => return Err("missing attack parameter".into()),
        }
        Ok(())
    }
}
fn default_attack() -> AttackBox {
    ron::from_str(include_str!("../data/attack_box_defaults.ron"))
        .expect("valid attack box defaults")
}
impl EditorParameter for OptionalAttack {
    const LABELS: &'static [&'static str] = &[
        "enabled (true or false)",
        "shape (Rect or Circle)",
        "offset x (pixels)",
        "offset y (pixels)",
        "width / circle diameter (pixels)",
        "height (Rect, pixels)",
        "damage (integer, at least 0)",
        "knockback x (pixels/second)",
        "knockback y (pixels/second)",
        "active duration (seconds)",
    ];
    fn editor_values(self) -> Vec<String> {
        let mut values = vec![self.0.is_some().to_string()];
        values.extend(self.0.unwrap_or_else(default_attack).editor_values());
        values
    }
    fn edit(&mut self, index: usize, text: &str) -> Result<(), String> {
        if index == 0 {
            self.0 = if parse::<bool>(text)? {
                Some(self.0.unwrap_or_else(default_attack))
            } else {
                None
            };
        } else {
            let mut attack = self.0.unwrap_or_else(default_attack);
            attack.edit(index - 1, text)?;
            attack.validate()?;
            if self.0.is_some() {
                self.0 = Some(attack);
            }
        }
        Ok(())
    }
}
impl EditorParameter for AttackSize {
    const LABELS: &'static [&'static str] = &["width (pixels)", "height (pixels)"];
    fn editor_values(self) -> Vec<String> {
        vec![self.0 .0.to_string(), self.0 .1.to_string()]
    }
    fn edit(&mut self, index: usize, text: &str) -> Result<(), String> {
        match index {
            0 => self.0 .0 = parse(text)?,
            1 => self.0 .1 = parse(text)?,
            _ => return Err("missing size parameter".into()),
        }
        Ok(())
    }
}
impl EditorParameter for AttackVector {
    const LABELS: &'static [&'static str] = &["x (pixels/second)", "y (pixels/second)"];
    fn editor_values(self) -> Vec<String> {
        vec![self.0 .0.to_string(), self.0 .1.to_string()]
    }
    fn edit(&mut self, index: usize, text: &str) -> Result<(), String> {
        match index {
            0 => self.0 .0 = parse(text)?,
            1 => self.0 .1 = parse(text)?,
            _ => return Err("missing vector parameter".into()),
        }
        Ok(())
    }
}
impl EditorParameter for AttackFeedback {
    const LABELS: &'static [&'static str] = &[
        "outline alpha (0..1)",
        "outline width (pixels)",
        "flash cycles per wind-up",
        "red tint (0..1)",
        "green tint (0..1)",
        "blue tint (0..1)",
    ];
    fn editor_values(self) -> Vec<String> {
        vec![
            self.outline_alpha.to_string(),
            self.outline_width.to_string(),
            self.flash_cycles.to_string(),
            self.tint.0.to_string(),
            self.tint.1.to_string(),
            self.tint.2.to_string(),
        ]
    }
    fn edit(&mut self, index: usize, text: &str) -> Result<(), String> {
        match index {
            0 => self.outline_alpha = parse(text)?,
            1 => self.outline_width = parse(text)?,
            2 => self.flash_cycles = parse(text)?,
            3 => self.tint.0 = parse(text)?,
            4 => self.tint.1 = parse(text)?,
            5 => self.tint.2 = parse(text)?,
            _ => return Err("missing feedback parameter".into()),
        }
        Ok(())
    }
}
