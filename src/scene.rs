//! Macroquad drawing of the world: background layers, floor, terrain, exits, items, coins,
//! enemies, the boss and the player. Call with the world camera set.

use hollow_steps::background::{self, Shape};
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::coin::COIN_RADIUS;
use hollow_steps::game::GameState;
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::player_body::PlayerBody;
use hollow_steps::room::Room;
use hollow_steps::terrain_style::{self, TerrainLook};
use macroquad::prelude::*;

use crate::{body_draw, terrain_draw};

/// Draws the world. `terrain_look` keeps the solids' draw list between frames. `camera_view` is
/// the rectangle the camera shows before screen shake, which the background layers use to work
/// out their parallax.
pub fn draw_scene(
    game: &GameState,
    player_body: Option<&PlayerBody>,
    terrain_look: &mut TerrainLook,
    camera_view: Rect,
) {
    let room = game.room();
    let bounds = game.world_bounds();
    let player = game.player();
    let player_position = player.position();
    let player_size = player.size();

    clear_background(Color::from_rgba(16, 18, 31, 255));
    draw_background(room, camera_view);
    draw_rectangle(
        bounds.left,
        bounds.floor_y,
        bounds.right - bounds.left,
        room.bounds.bottom() - bounds.floor_y,
        Color::from_rgba(34, 37, 58, 255),
    );
    draw_line(
        bounds.left,
        bounds.floor_y,
        bounds.right,
        bounds.floor_y,
        4.0,
        Color::from_rgba(111, 122, 164, 255),
    );
    // The solids' look is rebuilt only when the room, its solids or its palette change.
    let ground = terrain_style::ground_cover(room.bounds, bounds.floor_y);
    let look = terrain_look.shapes(
        room.id,
        room.terrain.solids(),
        Some(ground),
        &room.terrain_style,
    );
    terrain_draw::draw_shapes(look, camera_view);
    draw_rectangle_lines(
        room.bounds.x,
        room.bounds.y,
        room.bounds.w,
        room.bounds.h,
        2.0,
        Color::from_rgba(62, 70, 103, 255),
    );
    for exit in &room.exits {
        let zone = exit.zone(room.bounds);
        draw_rectangle(
            zone.x,
            zone.y,
            zone.w,
            zone.h,
            if room.exits_locked() {
                Color::from_rgba(220, 65, 70, 190)
            } else {
                Color::from_rgba(82, 172, 164, 100)
            },
        );
        if room.exits_locked() {
            draw_rectangle_lines(zone.x, zone.y, zone.w, zone.h, 3.0, RED);
        }
    }
    for item in game.items() {
        let nearby = item.can_pick_up(player);
        let top_left = item.position - ITEM_SIZE / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            ITEM_SIZE.x,
            ITEM_SIZE.y,
            if nearby { YELLOW } else { GOLD },
        );
        if nearby {
            draw_rectangle_lines(
                top_left.x - 3.0,
                top_left.y - 3.0,
                ITEM_SIZE.x + 6.0,
                ITEM_SIZE.y + 6.0,
                2.0,
                WHITE,
            );
            draw_text(
                format!("Up: pick up {}", item.item.name()),
                top_left.x - 80.0,
                top_left.y - 16.0,
                20.0,
                WHITE,
            );
        }
    }
    for coin in game.coins() {
        draw_circle(coin.position.x, coin.position.y, COIN_RADIUS, GOLD);
        draw_circle_lines(
            coin.position.x,
            coin.position.y,
            COIN_RADIUS - 2.0,
            1.0,
            YELLOW,
        );
    }
    for enemy in &game.room().enemies {
        if enemy.is_dead() {
            // A dying enemy only fades: its attacks, blasts and health bar are gone.
            body_draw::draw_enemy_body(enemy);
            continue;
        }
        for blast in enemy
            .behavior()
            .blasts()
            .iter()
            .chain(enemy.behavior().rings())
        {
            let color = if blast.exploded { RED } else { ORANGE };
            draw_circle_lines(blast.position.x, blast.position.y, blast.radius, 2.0, color);
        }
        // The health bar sits over the drawn picture, not the hitbox.
        let (size, top_left) = (
            enemy.visual_size(),
            enemy.visual_center() - enemy.visual_size() / 2.0,
        );
        body_draw::draw_enemy_body(enemy);
        body_draw::draw_enemy_attacks(enemy);
        draw_rectangle(top_left.x, top_left.y - 8.0, size.x, 4.0, DARKGRAY);
        draw_rectangle(
            top_left.x,
            top_left.y - 8.0,
            size.x * f32::from(enemy.health()) / f32::from(enemy.max_health()),
            4.0,
            RED,
        );
    }
    if let Some(boss) = &room.boss_special {
        draw_boss_special(boss, bounds.floor_y);
    }
    if let Some(hitbox) = player.strike_hitbox() {
        draw_rectangle(
            hitbox.x,
            hitbox.y,
            hitbox.w,
            hitbox.h,
            Color::from_rgba(255, 231, 160, 150),
        );
        draw_rectangle_lines(hitbox.x, hitbox.y, hitbox.w, hitbox.h, 2.0, GOLD);
    }
    match player_body {
        Some(body) => body_draw::draw_player(body, player),
        None => draw_rectangle(
            player_position.x - player_size.x / 2.0,
            player_position.y - player_size.y / 2.0,
            player_size.x,
            player_size.y,
            if player.health().flash_on() {
                WHITE
            } else {
                Color::from_rgba(110, 160, 230, 255)
            },
        ),
    }
}

/// Draws the room's background layers back to front, each moved by its parallax offset.
fn draw_background(room: &Room, camera_view: Rect) {
    for layer in &room.background {
        let offset = background::parallax_offset(
            camera_view.point(),
            room.bounds,
            camera_view.size(),
            layer.depth,
        );
        let (red, green, blue, alpha) = layer.color;
        let color = Color::from_rgba(red, green, blue, alpha);
        for shape in &layer.shapes {
            match *shape {
                Shape::Rect { x, y, w, h } => {
                    draw_rectangle(x + offset.x, y + offset.y, w, h, color)
                }
                Shape::Circle { x, y, radius } => {
                    draw_circle(x + offset.x, y + offset.y, radius, color)
                }
                Shape::Triangle { a, b, c } => draw_triangle(
                    vec2(a.0, a.1) + offset,
                    vec2(b.0, b.1) + offset,
                    vec2(c.0, c.1) + offset,
                    color,
                ),
            }
        }
    }
}

fn draw_boss_special(boss: &BossSpecials, floor_y: f32) {
    if !boss.is_defeated() {
        body_draw::draw_boss_attacks(boss);
        if let Some((target_x, radius)) = boss.landing_preview() {
            draw_rectangle(target_x - radius, floor_y - 12.0, radius * 2.0, 12.0, RED);
        }
        for ring in boss.rings() {
            let color = if ring.exploded() { ORANGE } else { RED };
            draw_circle_lines(
                ring.position().x,
                ring.position().y,
                ring.radius(),
                3.0,
                color,
            );
        }
        if boss.body().is_some() {
            body_draw::draw_boss_body(boss);
        } else {
            // Drawn in the visual box; the hitbox (`boss.size()`) is only gameplay.
            let top_left = boss.visual_center() - boss.visual_size() / 2.0;
            draw_rectangle(
                top_left.x,
                top_left.y,
                boss.visual_size().x,
                boss.visual_size().y,
                if boss.is_telegraphing() {
                    ORANGE
                } else {
                    PURPLE
                },
            );
        }
    } else if boss.is_death_effect_active() {
        draw_circle(
            boss.visual_center().x,
            boss.visual_center().y,
            boss.visual_size().x,
            WHITE,
        );
        // A boss with a body fades out over the flash; without one there is nothing to draw.
        body_draw::draw_boss_body(boss);
    }
}
