//! Macroquad adapter; all content geometry is resolved in the pure body module.
use hollow_steps::body::Shape;
use hollow_steps::body_debug::{
    body_primitives, label_requests, nearest_entity, ColorSlot, OverlayPrimitive, LINE_WIDTH,
};
use hollow_steps::enemy::Enemy;
use macroquad::prelude::*;

pub fn draw_enemy_body(enemy: &Enemy) {
    let Some(body) = enemy.body() else {
        let top_left = enemy.position() - enemy.size() / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            enemy.size().x,
            enemy.size().y,
            enemy_tint(enemy, MAROON),
        );
        return;
    };
    for part in &body.template.parts {
        let transform = body
            .part_transform(part, enemy.position(), enemy.size(), enemy.facing_left())
            .expect("validated body points and geometry");
        let (r, g, b, a) = body.visual.color(part.color_slot);
        let color = enemy_tint(enemy, Color::new(r, g, b, a));
        match part.shape {
            Shape::Ellipse => draw_ellipse(
                transform.center.x,
                transform.center.y,
                transform.size.x / 2.0,
                transform.size.y / 2.0,
                transform.rotation,
                color,
            ),
            Shape::Triangle => draw_triangle(
                transform.vertex(vec2(-0.5, -0.5)),
                transform.vertex(vec2(0.5, 0.0)),
                transform.vertex(vec2(-0.5, 0.5)),
                color,
            ),
            Shape::Rect => {
                let a = transform.vertex(vec2(-0.5, -0.5));
                let b = transform.vertex(vec2(0.5, -0.5));
                let c = transform.vertex(vec2(0.5, 0.5));
                let d = transform.vertex(vec2(-0.5, 0.5));
                draw_triangle(a, b, c, color);
                draw_triangle(a, c, d, color);
            }
            Shape::Line => {
                let (from, to) = transform.endpoints.expect("Line endpoints");
                draw_line(
                    from.x,
                    from.y,
                    to.x,
                    to.y,
                    transform.size.min_element(),
                    color,
                );
            }
        }
    }
}

fn enemy_tint(enemy: &Enemy, color: Color) -> Color {
    let Some(warning) = enemy.behavior().warning(enemy.size()) else {
        return color;
    };
    let tint = warning.feedback.tint;
    Color::new(
        color.r + (tint.0 - color.r) * warning.strength,
        color.g + (tint.1 - color.g) * warning.strength,
        color.b + (tint.2 - color.b) * warning.strength,
        color.a,
    )
}

pub fn draw_enemy_attacks(enemy: &Enemy) {
    if let Some(warning) = enemy.behavior().warning(enemy.size()) {
        let center = warning.attack.center(enemy.position());
        let size = Vec2::from(warning.attack.size);
        let feedback = warning.feedback;
        let color = Color::new(
            feedback.tint.0,
            feedback.tint.1,
            feedback.tint.2,
            feedback.outline_alpha,
        );
        match warning.attack.shape {
            hollow_steps::enemy_attack::AttackShape::Rect => draw_rectangle_lines(
                center.x - size.x / 2.0,
                center.y - size.y / 2.0,
                size.x,
                size.y,
                feedback.outline_width,
                color,
            ),
            hollow_steps::enemy_attack::AttackShape::Circle => draw_circle_lines(
                center.x,
                center.y,
                size.x / 2.0,
                feedback.outline_width,
                color,
            ),
        }
    }
    let tint = enemy.behavior().projectile_feedback().tint;
    for projectile in enemy.behavior().projectiles() {
        let top_left = projectile.position - projectile.size / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            projectile.size.x,
            projectile.size.y,
            Color::new(tint.0, tint.1, tint.2, 1.0),
        );
    }
}

fn debug_font_data() -> std::borrow::Cow<'static, [u8]> {
    use egui_macroquad::egui::{FontDefinitions, FontFamily};
    let fonts = FontDefinitions::default();
    let name = &fonts.families[&FontFamily::Monospace][0];
    fonts.font_data[name].font.clone()
}

/// Own a separate font atlas: cloning the default Font would still share the
/// HUD atlas. Reuse egui's bundled monospace font bytes, not its GPU texture.
pub struct BodyDebugOverlay {
    font: Font,
}

impl BodyDebugOverlay {
    pub fn new() -> Self {
        Self {
            font: load_ttf_font_from_bytes(&debug_font_data()).expect("bundled debug font"),
        }
    }
}

/// Called after scene/UI drawing. Projection is CPU-only: camera, viewport,
/// material and shared texture filters are left untouched.
pub fn draw_body_debug_overlay(
    overlay: &BodyDebugOverlay,
    enemies: &[Enemy],
    player_position: Vec2,
    to_screen: impl Fn(Vec2) -> Vec2,
    scale: f32,
) {
    let nearest = nearest_entity(enemies, player_position);
    let color = |slot: ColorSlot| {
        let (r, g, b, a) = slot.rgba();
        Color::new(r, g, b, a)
    };
    let mut primitives = Vec::new();
    for (index, enemy) in enemies.iter().enumerate() {
        let Some(body) = enemy.body() else {
            continue;
        };
        primitives.extend(body_primitives(
            body,
            enemy.position(),
            enemy.size(),
            enemy.facing_left(),
            nearest == Some(index),
        ));
    }

    // F3 used to grow the shared HUD atlas. Miniquad can reuse its deleted GL
    // texture ID while retaining the old cached binding, leaving the replacement
    // texture empty. This font owns its atlas and never mutates the HUD font.
    // Finish earlier batches, cache the entire pass (including DPI handling),
    // then submit text only after the atlas layout has settled.
    unsafe { get_internal_gl().flush() };
    for (text, font_size) in label_requests(&primitives) {
        measure_text(text, Some(&overlay.font), font_size, 1.0);
    }
    for primitive in primitives {
        let primitive = primitive.project(&to_screen, scale);
        match primitive {
            OverlayPrimitive::Circle {
                position,
                radius,
                color_slot,
            } => {
                draw_circle(position.x, position.y, radius, color(color_slot));
            }
            OverlayPrimitive::Line {
                from,
                to,
                color_slot,
            } => {
                draw_line(from.x, from.y, to.x, to.y, LINE_WIDTH, color(color_slot));
            }
            OverlayPrimitive::Rect { rect, color_slot } => {
                draw_rectangle_lines(
                    rect.x,
                    rect.y,
                    rect.w,
                    rect.h,
                    LINE_WIDTH,
                    color(color_slot),
                );
            }
            OverlayPrimitive::Label {
                position,
                text,
                font_size,
                color_slot,
            } => {
                draw_text_ex(
                    &text,
                    position.x,
                    position.y,
                    TextParams {
                        font: Some(&overlay.font),
                        font_size,
                        color: color(color_slot),
                        ..Default::default()
                    },
                );
            }
        }
    }
    // Later callers may cache new glyphs too; don't leave overlay text queued.
    unsafe { get_internal_gl().flush() };
}

#[cfg(test)]
mod tests {
    use super::debug_font_data;
    use egui_macroquad::egui::{Color32, Context, FontData, FontDefinitions, FontFamily, FontId};
    use hollow_steps::body_debug::FONT_SIZES;

    #[test]
    fn bundled_overlay_font_rasterizes_labels_without_a_window_or_font_file() {
        let mut fonts = FontDefinitions::empty();
        fonts.font_data.insert(
            "debug".into(),
            std::sync::Arc::new(FontData::from_owned(debug_font_data().into_owned())),
        );
        fonts
            .families
            .insert(FontFamily::Monospace, vec!["debug".into()]);
        let context = Context::default();
        context.set_fonts(fonts);
        let _ = context.run(Default::default(), |ctx| {
            for font_size in FONT_SIZES {
                let text = "root_head_hand_l_foot_r";
                let galley = ctx.fonts(|fonts| {
                    fonts.layout_no_wrap(
                        text.into(),
                        FontId::monospace(font_size.into()),
                        Color32::GREEN,
                    )
                });
                assert_eq!(galley.rows.len(), 1);
                assert_eq!(galley.rows[0].glyphs.len(), text.len());
                assert!(galley.rows[0].glyphs.iter().all(|glyph| {
                    glyph.advance_width > 0.0 && glyph.uv_rect.max != glyph.uv_rect.min
                }));
            }
        });
    }
}
