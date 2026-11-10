//! Macroquad adapter; all content geometry is resolved in the pure body module.
use hollow_steps::body::Shape;
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
