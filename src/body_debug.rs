//! Window-free body debug geometry. Presentation only; never steps body motion.
use crate::body::{resolve_point, BodyInstance};
use crate::enemy::Enemy;
use macroquad::math::{Rect, Vec2};

pub const REST_RADIUS: f32 = 2.0;
pub const CURRENT_RADIUS: f32 = 3.0;
pub const TARGET_RADIUS: f32 = 3.0;
pub const LINE_WIDTH: f32 = 1.0;
pub const LABEL_FONT_SIZE: u16 = 12;
pub const FONT_SIZES: [u16; 1] = [LABEL_FONT_SIZE];
pub const LABEL_OFFSET: Vec2 = Vec2::new(6.0, -4.0);

pub const REST_COLOR: (f32, f32, f32, f32) = (0.5, 0.5, 0.5, 1.0);
pub const CURRENT_COLOR: (f32, f32, f32, f32) = (0.0, 1.0, 0.0, 1.0);
pub const TARGET_COLOR: (f32, f32, f32, f32) = (1.0, 1.0, 0.0, 1.0);
pub const HITBOX_COLOR: (f32, f32, f32, f32) = (1.0, 0.0, 0.0, 1.0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorSlot {
    Rest,
    Current,
    Target,
    Hitbox,
}

impl ColorSlot {
    pub fn rgba(self) -> (f32, f32, f32, f32) {
        match self {
            Self::Rest => REST_COLOR,
            Self::Current => CURRENT_COLOR,
            Self::Target => TARGET_COLOR,
            Self::Hitbox => HITBOX_COLOR,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum OverlayPrimitive {
    Circle {
        position: Vec2,
        radius: f32,
        color_slot: ColorSlot,
    },
    Line {
        from: Vec2,
        to: Vec2,
        color_slot: ColorSlot,
    },
    Rect {
        rect: Rect,
        color_slot: ColorSlot,
    },
    Label {
        position: Vec2,
        text: String,
        font_size: u16,
        color_slot: ColorSlot,
    },
}

impl OverlayPrimitive {
    /// Project world geometry separately from generation. Display sizes stay in pixels.
    pub fn project(&self, to_screen: impl Fn(Vec2) -> Vec2, scale: f32) -> Self {
        match self {
            Self::Circle {
                position,
                radius,
                color_slot,
            } => Self::Circle {
                position: to_screen(*position),
                radius: *radius,
                color_slot: *color_slot,
            },
            Self::Line {
                from,
                to,
                color_slot,
            } => Self::Line {
                from: to_screen(*from),
                to: to_screen(*to),
                color_slot: *color_slot,
            },
            Self::Rect { rect, color_slot } => {
                let top_left = to_screen(rect.point());
                Self::Rect {
                    rect: Rect::new(top_left.x, top_left.y, rect.w * scale, rect.h * scale),
                    color_slot: *color_slot,
                }
            }
            Self::Label {
                position,
                text,
                font_size,
                color_slot,
            } => Self::Label {
                position: to_screen(*position - LABEL_OFFSET) + LABEL_OFFSET,
                text: text.clone(),
                font_size: *font_size,
                color_slot: *color_slot,
            },
        }
    }
}

/// The complete cache requests for a pass, consumed before submitting any text.
pub fn label_requests(primitives: &[OverlayPrimitive]) -> impl Iterator<Item = (&str, u16)> {
    primitives.iter().filter_map(|primitive| match primitive {
        OverlayPrimitive::Label {
            text, font_size, ..
        } => Some((text.as_str(), *font_size)),
        _ => None,
    })
}

/// Generate world geometry without a camera or zoom. The gameplay box never
/// inherits the visual body scale; display sizes are applied during projection.
pub fn body_primitives(
    body: &BodyInstance,
    center: Vec2,
    size: Vec2,
    facing_left: bool,
    show_labels: bool,
) -> Vec<OverlayPrimitive> {
    let mut primitives = Vec::new();
    let rest = body.template.rest_positions();
    for name in body.template.points.keys() {
        let position = |points| {
            resolve_point(name, points, center, size, body.visual.scale, facing_left)
                .expect("validated body point")
        };
        let current = position(&body.points);
        let target = position(&body.targets);
        primitives.push(OverlayPrimitive::Line {
            from: current,
            to: target,
            color_slot: ColorSlot::Target,
        });
        for (position, radius, color_slot) in [
            (position(&rest), REST_RADIUS, ColorSlot::Rest),
            (current, CURRENT_RADIUS, ColorSlot::Current),
            (target, TARGET_RADIUS, ColorSlot::Target),
        ] {
            primitives.push(OverlayPrimitive::Circle {
                position,
                radius,
                color_slot,
            });
        }
        if show_labels {
            primitives.push(OverlayPrimitive::Label {
                position: current + LABEL_OFFSET,
                text: name.clone(),
                font_size: LABEL_FONT_SIZE,
                color_slot: ColorSlot::Current,
            });
        }
    }
    let top_left = center - size / 2.0;
    primitives.push(OverlayPrimitive::Rect {
        rect: Rect::new(top_left.x, top_left.y, size.x, size.y),
        color_slot: ColorSlot::Hitbox,
    });
    primitives
}

/// Nearest body-bearing entity by center distance, retaining the first on ties.
/// Bodyless entities cannot take the labels away from a visible body.
pub fn nearest_entity(entities: &[Enemy], player_position: Vec2) -> Option<usize> {
    entities
        .iter()
        .enumerate()
        .filter(|(_, entity)| entity.body().is_some())
        .min_by(|(_, a), (_, b)| {
            a.position()
                .distance_squared(player_position)
                .total_cmp(&b.position().distance_squared(player_position))
        })
        .map(|(index, _)| index)
}

/// Nearest visible body supplies the pose label; font size matches the existing HUD.
pub const LEGEND_FONT_SIZE: u16 = 18;
pub fn pose_legend(enemies: &[Enemy], player_position: Vec2) -> String {
    let pose = nearest_entity(enemies, player_position)
        .and_then(|index| enemies[index].body())
        .and_then(|body| body.pose_name.as_deref())
        .unwrap_or("none");
    format!("F3: rest gray | current green | target yellow | hitbox red | pose: {pose}")
}
