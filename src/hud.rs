//! Heads-up display drawn in virtual-resolution space, anchored to the screen edges.

use hollow_steps::boss_special::SpecialAttack;
use hollow_steps::camera::{anchored, Anchor};
use hollow_steps::game::GameState;
use hollow_steps::health::MAX_HEALTH;
use hollow_steps::hud_layout::{
    boss_caption, boss_fill, pip_center, pip_row_size, pip_states, room_label, PIP_RADIUS,
};
use hollow_steps::room::Room;
use macroquad::prelude::*;

pub fn draw_hud(game: &GameState, view: Vec2) {
    let room = game.room();
    let player = game.player();
    let margin = 24.0;

    // Top left: health masks, then the coin and item counters.
    let masks = pip_states(player.health().current(), MAX_HEALTH);
    let row = anchored(
        view,
        Anchor::TopLeft,
        pip_row_size(masks.len()),
        vec2(margin, 20.0),
    );
    for (index, filled) in masks.into_iter().enumerate() {
        draw_mask(pip_center(row, index), PIP_RADIUS, filled);
    }
    let counters = anchored(view, Anchor::TopLeft, Vec2::ZERO, vec2(margin, 62.0));
    draw_circle(counters.x + 9.0, counters.y + 2.0, 9.0, GOLD);
    draw_circle_lines(counters.x + 9.0, counters.y + 2.0, 6.0, 1.5, YELLOW);
    draw_text(
        game.coin_count().to_string(),
        counters.x + 26.0,
        counters.y + 10.0,
        28.0,
        WHITE,
    );
    draw_text(
        format!("Items {}", player.collected_items().len()),
        counters.x + 90.0,
        counters.y + 8.0,
        20.0,
        LIGHTGRAY,
    );

    // Top right: where you are, and whether the way on is open.
    let title = room_label(room.id);
    let note = if room.exits_locked() {
        "Exits locked until the boss falls"
    } else {
        "Walk into a green exit"
    };
    for (text, size, top, color) in [
        (title.as_str(), 28.0, 38.0, WHITE),
        (note, 18.0, 62.0, LIGHTGRAY),
    ] {
        let width = measure_text(text, None, size as u16, 1.0).width;
        let at = anchored(view, Anchor::TopRight, vec2(width, 0.0), vec2(margin, top));
        draw_text(text, at.x, at.y, size, color);
    }

    // Bottom left: controls, kept small.
    let hint =
        "A/D move   Space/Z jump   X attack   Up pickup   Shift dash   I inventory   F5 reload";
    let at = anchored(view, Anchor::BottomLeft, Vec2::ZERO, vec2(margin, 14.0));
    draw_text(hint, at.x, at.y, 18.0, Color::from_rgba(170, 180, 210, 255));
    draw_boss_hud(room, view);
}

/// A health mask: a pale face with two dark eye holes when full, a dim outline when empty.
fn draw_mask(center: Vec2, radius: f32, filled: bool) {
    if filled {
        draw_ellipse(
            center.x,
            center.y,
            radius * 0.85,
            radius,
            0.0,
            Color::from_rgba(236, 240, 250, 255),
        );
        for side in [-1.0, 1.0] {
            draw_ellipse(
                center.x + side * radius * 0.36,
                center.y - radius * 0.1,
                radius * 0.2,
                radius * 0.3,
                -side * 15.0,
                Color::from_rgba(16, 18, 31, 255),
            );
        }
    } else {
        draw_ellipse_lines(
            center.x,
            center.y,
            radius * 0.85,
            radius,
            0.0,
            2.0,
            Color::from_rgba(90, 96, 130, 255),
        );
    }
}

fn draw_boss_hud(room: &Room, view: Vec2) {
    if let Some(boss) = &room.boss_special {
        draw_boss_health(boss.name(), boss.health(), boss.max_health(), view);
        let cue = match boss.active_attack() {
            Some(SpecialAttack::Slam) => "Slam - move out of the red zone!",
            Some(SpecialAttack::Summon) => "Summoning enemies",
            Some(SpecialAttack::Rings) => "Rings - avoid the red circles!",
            None if boss.is_defeated() => "Defeated - collect the coins!",
            None => boss.active_attack_name().unwrap_or("Recovering"),
        };
        draw_boss_cue(cue, view);
    }
}

const BOSS_BAR_SIZE: Vec2 = vec2(360.0, 12.0);
const BOSS_BAR_TOP: f32 = 60.0;

fn draw_boss_health(name: &str, health: u32, max_health: u32, view: Vec2) {
    let bar = anchored(
        view,
        Anchor::TopCenter,
        BOSS_BAR_SIZE,
        vec2(0.0, BOSS_BAR_TOP),
    );
    draw_text(
        boss_caption(name, health, max_health),
        bar.x,
        bar.y - 10.0,
        26.0,
        WHITE,
    );
    draw_rectangle(bar.x, bar.y, BOSS_BAR_SIZE.x, BOSS_BAR_SIZE.y, DARKGRAY);
    draw_rectangle(
        bar.x,
        bar.y,
        BOSS_BAR_SIZE.x * boss_fill(health, max_health),
        BOSS_BAR_SIZE.y,
        RED,
    );
}

fn draw_boss_cue(cue: &str, view: Vec2) {
    let bar = anchored(
        view,
        Anchor::TopCenter,
        BOSS_BAR_SIZE,
        vec2(0.0, BOSS_BAR_TOP),
    );
    draw_text(cue, bar.x, bar.y + 40.0, 22.0, WHITE);
}

/// Shown while a room is being tested from the editor.
pub fn draw_test_banner(view: Vec2) {
    let text = "TEST ROOM - F2: back to the editor (nothing is saved)";
    let size = 22.0;
    let width = measure_text(text, None, size as u16, 1.0).width;
    let at = anchored(
        view,
        Anchor::BottomCenter,
        vec2(width, 0.0),
        vec2(0.0, 18.0),
    );
    draw_text(text, at.x, at.y, size, Color::from_rgba(255, 214, 120, 255));
}

pub fn draw_inventory(game: &GameState, view: Vec2) {
    draw_rectangle(0.0, 0.0, view.x, view.y, Color::from_rgba(0, 0, 0, 180));
    let size = vec2(560.0, 310.0);
    let panel = anchored(view, Anchor::Center, size, Vec2::ZERO);
    draw_rectangle(
        panel.x,
        panel.y,
        size.x,
        size.y,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, size.x, size.y, 2.0, GRAY);
    let (x, top) = (panel.x + 32.0, panel.y);
    draw_text("Inventory", x, top + 50.0, 36.0, WHITE);
    draw_text(
        format!("Coins: {}", game.coin_count()),
        x,
        top + 94.0,
        26.0,
        GOLD,
    );
    draw_text("Collected items", x, top + 140.0, 24.0, WHITE);
    let items = game.player().collected_items();
    if items.is_empty() {
        draw_text("No items collected yet", x, top + 176.0, 22.0, LIGHTGRAY);
    } else {
        for (index, item) in items.iter().enumerate() {
            draw_text(
                item.name(),
                x,
                top + 176.0 + index as f32 * 28.0,
                22.0,
                GOLD,
            );
        }
    }
    draw_text(
        "Paused  |  I or Esc to return",
        x,
        top + 278.0,
        22.0,
        LIGHTGRAY,
    );
}

pub fn draw_reload_result(
    result: &Result<(), String>,
    failure_title: &str,
    scroll: &mut usize,
    view: Vec2,
) {
    let margin = vec2(16.0, 12.0);
    let Err(error) = result else {
        let size = vec2(view.x - margin.x * 2.0, 36.0);
        let bar = anchored(view, Anchor::BottomLeft, size, margin);
        draw_rectangle(bar.x, bar.y, size.x, size.y, BLACK);
        draw_text("Rooms reloaded", bar.x + 12.0, bar.y + 24.0, 22.0, GREEN);
        return;
    };

    const VISIBLE_LINES: usize = 8;
    const FONT_SIZE: u16 = 18;
    let mut lines = Vec::new();
    for paragraph in error.lines() {
        let mut line = String::new();
        for character in paragraph.chars() {
            line.push(character);
            if measure_text(&line, None, FONT_SIZE, 1.0).width > view.x - 72.0 {
                line.pop();
                lines.push(std::mem::take(&mut line));
                line.push(character);
            }
        }
        lines.push(line);
    }
    let max_scroll = lines.len().saturating_sub(VISIBLE_LINES);
    *scroll = (*scroll as f32 - mouse_wheel().1).clamp(0.0, max_scroll as f32) as usize;
    let panel_size = vec2(view.x - margin.x * 2.0, 248.0);
    let panel = anchored(view, Anchor::BottomLeft, panel_size, margin);
    let (left, top) = (panel.x + 12.0, panel.y);
    draw_rectangle(
        panel.x,
        panel.y,
        panel_size.x,
        panel_size.y,
        Color::from_rgba(28, 12, 18, 245),
    );
    draw_text(failure_title, left, top + 28.0, 22.0, RED);
    for (index, line) in lines.iter().skip(*scroll).take(VISIBLE_LINES).enumerate() {
        draw_text(
            line,
            left,
            top + 56.0 + index as f32 * 22.0,
            FONT_SIZE as f32,
            WHITE,
        );
    }
    let hint = if max_scroll > 0 {
        format!(
            "Mouse wheel: scroll errors ({}/{})",
            *scroll + 1,
            max_scroll + 1
        )
    } else {
        "Fix the files, then press F5 to retry".to_owned()
    };
    draw_text(&hint, left, top + 232.0, 18.0, LIGHTGRAY);
}
