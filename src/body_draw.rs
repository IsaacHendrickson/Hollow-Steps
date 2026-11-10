//! Macroquad adapter; all content geometry is resolved in the pure body module.
use hollow_steps::body::Shape;
use hollow_steps::body_debug::{
    body_primitives, nearest_entity, ColorSlot, OverlayPrimitive, LABEL_SIZE, LINE_WIDTH,
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

/// Called after scene/UI drawing. Canvas zoom changes geometry, not debug dot sizes.
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
    for (index, enemy) in enemies.iter().enumerate() {
        let Some(body) = enemy.body() else {
            continue;
        };
        for primitive in body_primitives(
            body,
            to_screen(enemy.position()),
            enemy.size() * scale,
            enemy.facing_left(),
            nearest == Some(index),
        ) {
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
                    color_slot,
                } => {
                    draw_text(&text, position.x, position.y, LABEL_SIZE, color(color_slot));
                }
            }
        }
    }
}
