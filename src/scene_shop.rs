//! Macroquad drawing of the hub's shop in the world: a small stall where `run.ron` puts it, and
//! the "Up: shop" prompt while the player stands in reach. The prompt follows the rule that opens
//! the shop (`GameState::shop_in_reach`), so the two cannot disagree. Called from `scene.rs`.

use hollow_steps::game::GameState;
use hollow_steps::shop_menu::SHOP_SIZE;
use macroquad::prelude::*;

const PROMPT: &str = "Up: shop";
const PROMPT_SIZE: u16 = 20;

/// Draws the stall and, in reach, its prompt. Draws nothing where there is no shop to use (in a
/// run, on the result screen, in a test room, in any room but the hub).
pub fn draw_shop(game: &GameState) {
    let Some(position) = game.shop_position() else {
        return;
    };
    let top_left = position - SHOP_SIZE / 2.0;
    let (width, height) = (SHOP_SIZE.x, SHOP_SIZE.y);
    let awning = height * 0.3;
    // The counter, its awning and a coin on top of it.
    draw_rectangle(
        top_left.x,
        top_left.y + awning,
        width,
        height - awning,
        Color::from_rgba(120, 84, 40, 255),
    );
    draw_rectangle(
        top_left.x - 4.0,
        top_left.y,
        width + 8.0,
        awning,
        Color::from_rgba(196, 64, 72, 255),
    );
    draw_circle(position.x, top_left.y + height * 0.62, 7.0, GOLD);
    draw_circle_lines(position.x, top_left.y + height * 0.62, 7.0, 1.5, YELLOW);
    if !game.shop_in_reach() {
        return;
    }
    draw_rectangle_lines(
        top_left.x - 7.0,
        top_left.y - 3.0,
        width + 14.0,
        height + 6.0,
        2.0,
        WHITE,
    );
    let text_width = measure_text(PROMPT, None, PROMPT_SIZE, 1.0).width;
    draw_text(
        PROMPT,
        position.x - text_width / 2.0,
        top_left.y - 14.0,
        f32::from(PROMPT_SIZE),
        WHITE,
    );
}
