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
            MAROON,
        );
        return;
    };
    for part in &body.template.parts {
        let transform = body
            .part_transform(part, enemy.position(), enemy.size(), enemy.facing_left())
            .expect("validated body points and geometry");
        let (r, g, b, a) = body.visual.color(part.color_slot);
        let color = Color::new(r, g, b, a);
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

/// Called after scene/UI drawing. Projection is CPU-only: camera, viewport,
/// material and texture filters are left untouched.
pub fn draw_body_debug_overlay(
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
    // Labels share the HUD's default font atlas. Lazy glyph insertion during
    // drawing can grow/repack it, invalidating queued text UVs and textures.
    // Finish earlier batches, cache the entire pass (including DPI handling),
    // then submit text only after the atlas layout has settled.
    unsafe { get_internal_gl().flush() };
    for (text, font_size) in label_requests(&primitives) {
        measure_text(text, None, font_size, 1.0);
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
