//! Macroquad adapter for the hub-and-run loop's screens: the result screen and the key or click
//! that leaves it, and the coin counters of the HUD and the inventory. The text and the
//! layout are pure (`hollow_steps::run`); everything here is in virtual-screen coordinates.

use hollow_steps::charm_menu::{wrap_lines, InventoryLayout};
use hollow_steps::game::GameState;
use hollow_steps::run::{
    hud_counter, inventory_coins, is_tier_line, result_layout, result_lines_with_seed,
    result_title, tier_label, tier_label_origin, Counter, RunOutcome, RETURN_HINT, TIER_LABEL_SIZE,
};
use macroquad::prelude::*;

const TITLE_SIZE: f32 = 42.0;
const LINE_SIZE: f32 = 28.0;
const HINT_SIZE: f32 = 24.0;
const MESSAGE_SIZE: u16 = 18;
const MESSAGE_LINES: usize = 2;
const MESSAGE_STEP: f32 = 20.0;
/// How the result screen's seed line begins (`hollow_steps::run::seed_line`).
const SEED_PREFIX: &str = "Seed:";
const WARNING: Color = Color::new(0.94, 0.78, 0.47, 1.0);

/// Leaves the result screen on Enter or a left click and returns true when it was left.
/// `keyboard` and `pointer` are false while a window of another tool has the keys or the mouse.
pub fn handle_result_input(game: &mut GameState, keyboard: bool, pointer: bool) -> bool {
    if game.run_result().is_none() {
        return false;
    }
    let enter = keyboard && (is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter));
    let click = pointer && is_mouse_button_pressed(MouseButton::Left);
    if !(enter || click) {
        return false;
    }
    game.continue_after_result();
    game.run_result().is_none()
}

/// The coin counter of the HUD for the game as it stands.
pub fn counter(game: &GameState) -> Counter {
    hud_counter(game.run_phase(), game.coin_count(), game.profile())
}

/// The caption under the HUD's coin number (banked and tier in the hub, "this run" in a run).
/// `origin` is where the counter row is anchored.
pub fn draw_caption(counter: &Counter, origin: Vec2) {
    if let Some(caption) = &counter.caption {
        draw_text(caption, origin.x + 26.0, origin.y + 30.0, 18.0, LIGHTGRAY);
    }
}

/// The small "Tier N" under the room's title and note, top right: in the hub and in a run, not on
/// the result screen and not with no hub.
pub fn draw_tier(game: &GameState, view: Vec2) {
    let Some(tier) = game.hud_tier() else {
        return;
    };
    let text = tier_label(tier);
    let width = measure_text(&text, None, TIER_LABEL_SIZE as u16, 1.0).width;
    let at = tier_label_origin(view, width);
    draw_text(&text, at.x, at.y, TIER_LABEL_SIZE, LIGHTGRAY);
}

/// The inventory's coin line, and the banked coins and tier at the right end of it when a hub
/// is set.
pub fn draw_inventory_coins(game: &GameState, layout: &InventoryLayout) {
    let (coins, banked) = inventory_coins(game.run_phase(), game.coin_count(), game.profile());
    draw_text(&coins, layout.coins.x, layout.coins.y, 26.0, GOLD);
    if let Some(banked) = banked {
        let width = measure_text(&banked, None, 22, 1.0).width;
        let x = layout.coins.x + layout.text_width - width;
        draw_text(&banked, x, layout.coins.y, 22.0, LIGHTGRAY);
    }
}

/// The result screen over the frozen picture. Draws nothing while no result shows.
pub fn draw_result(game: &GameState, view: Vec2) {
    let Some(result) = game.run_result() else {
        return;
    };
    draw_rectangle(0.0, 0.0, view.x, view.y, Color::from_rgba(0, 0, 0, 200));
    let lines = result_lines_with_seed(
        &result,
        game.profile(),
        game.result_bonus(),
        game.run_seed(),
    );
    let layout = result_layout(view, lines.len());
    let panel = layout.panel;
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, GRAY);

    let title_color = match result.outcome {
        RunOutcome::Cleared => GOLD,
        RunOutcome::Died => Color::from_rgba(230, 90, 90, 255),
        RunOutcome::Abandoned => LIGHTGRAY,
    };
    draw_centered(
        result_title(result.outcome),
        panel,
        layout.title.y,
        TITLE_SIZE,
        title_color,
    );
    for (line, at) in lines.iter().zip(&layout.lines) {
        // After a clear the tier going up is gold; the seed line under it is dimmer than the rest.
        let color = if result.outcome == RunOutcome::Cleared && is_tier_line(line) {
            GOLD
        } else if line.starts_with(SEED_PREFIX) {
            LIGHTGRAY
        } else {
            WHITE
        };
        draw_text(line, at.x, at.y, LINE_SIZE, color);
    }
    if let Some(message) = game.run_message() {
        let measure = |text: &str| measure_text(text, None, MESSAGE_SIZE, 1.0).width;
        let wrapped = wrap_lines(message, layout.text_width, MESSAGE_LINES, &measure);
        for (index, line) in wrapped.iter().enumerate() {
            let y = layout.message.y - 10.0 + index as f32 * MESSAGE_STEP;
            draw_text(line, layout.text_x, y, f32::from(MESSAGE_SIZE), WARNING);
        }
    }
    draw_centered(RETURN_HINT, panel, layout.hint.y, HINT_SIZE, LIGHTGRAY);
}

/// `text` centered across `panel` on the baseline `y`.
fn draw_centered(text: &str, panel: Rect, y: f32, size: f32, color: Color) {
    let width = measure_text(text, None, size as u16, 1.0).width;
    draw_text(text, panel.center().x - width / 2.0, y, size, color);
}
