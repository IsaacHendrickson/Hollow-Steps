//! Macroquad drawing of the hub's upgrade shop: a panel with the banked coins, one row per
//! upgrade (name, level, next price, what a level gives) with the selected one lit, and under
//! the list the selected upgrade's description and the last result of a purchase. The positions
//! come from `hollow_steps::shop_menu::layout`, which the mouse uses too.

use hollow_steps::charm_menu::{fit_line, wrap_lines};
use hollow_steps::game::GameState;
use hollow_steps::shop_menu::{
    coins_text, row_columns, ShopLayout, ShopRow, DESCRIPTION_LINES, MESSAGE_LINES, ROW_HEIGHT,
};
use macroquad::prelude::*;

use crate::hud::centered_baseline;

const ROW_FONT: u16 = 20;
const DETAIL_FONT: u16 = 20;
const HEADING_FONT: u16 = 18;
const GOOD: Color = Color::new(0.47, 0.86, 0.55, 1.0);
const REFUSAL: Color = Color::new(1.0, 0.55, 0.4, 1.0);

fn measure(text: &str, font_size: u16) -> f32 {
    measure_text(text, None, font_size, 1.0).width
}

/// The shop window over the frozen picture, laid out by `GameState::shop_layout`.
pub fn draw_shop(game: &GameState, view: Vec2) {
    draw_rectangle(0.0, 0.0, view.x, view.y, Color::from_rgba(0, 0, 0, 180));
    let layout = game.shop_layout(view);
    let panel = layout.panel;
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, GRAY);
    draw_text("Upgrade shop", layout.title.x, layout.title.y, 36.0, WHITE);
    draw_text(
        coins_text(game.profile()),
        layout.coins.x,
        layout.coins.y,
        26.0,
        GOLD,
    );
    let rows = game.shop_rows();
    if rows.is_empty() {
        let note = layout.empty_note;
        draw_text("Nothing is for sale yet", note.x, note.y, 22.0, LIGHTGRAY);
    } else {
        draw_list(game, &layout, &rows);
    }
    let hint = "Up/Down: choose  |  Enter or click: buy  |  Esc: close";
    draw_text(hint, layout.footer.x, layout.footer.y, 22.0, LIGHTGRAY);
}

fn draw_list(game: &GameState, layout: &ShopLayout, rows: &[ShopRow]) {
    // Column headings over the columns of the rows below.
    let columns = row_columns(Rect::new(
        layout.header.x,
        layout.header.y,
        layout.text_width,
        ROW_HEIGHT,
    ));
    let heading = f32::from(HEADING_FONT);
    for (text, x) in [
        ("Upgrade", columns.name_x),
        ("Level", columns.level_x),
        ("Next cost", columns.cost_x),
        ("Each level", columns.effect_x),
    ] {
        draw_text(text, x, layout.header.y, heading, GRAY);
    }
    let selected = game.shop_menu().selected(rows.len());
    for placed in &layout.rows {
        draw_row(
            placed.rect,
            Some(placed.index) == selected,
            &rows[placed.index],
        );
    }
    draw_scroll_marks(layout);
    let Some(upgrade) = game.selected_upgrade() else {
        return;
    };
    let measure_detail = |text: &str| measure(text, DETAIL_FONT);
    let lines = wrap_lines(
        &upgrade.description,
        layout.text_width,
        DESCRIPTION_LINES,
        &measure_detail,
    );
    for (line, at) in lines.iter().zip(layout.description) {
        draw_text(line, at.x, at.y, f32::from(DETAIL_FONT), LIGHTGRAY);
    }
    if let Some(note) = game.shop_menu().note() {
        let lines = wrap_lines(
            &note.text,
            layout.text_width,
            MESSAGE_LINES,
            &measure_detail,
        );
        let color = if note.refusal { REFUSAL } else { GOOD };
        for (line, at) in lines.iter().zip(layout.message) {
            draw_text(line, at.x, at.y, f32::from(DETAIL_FONT), color);
        }
    }
}

fn draw_row(rect: Rect, selected: bool, row: &ShopRow) {
    let (fill, border) = if selected {
        (Color::from_rgba(70, 78, 120, 255), GOLD)
    } else {
        (Color::from_rgba(44, 50, 78, 255), GRAY)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
    let columns = row_columns(rect);
    let baseline = centered_baseline(rect.center().y, ROW_FONT);
    let font = f32::from(ROW_FONT);
    let measure_row = |text: &str| measure(text, ROW_FONT);
    let name = fit_line(&row.name, columns.name_width, &measure_row);
    draw_text(&name, columns.name_x, baseline, font, WHITE);
    let level_color = if row.maxed { GOOD } else { LIGHTGRAY };
    draw_text(&row.level, columns.level_x, baseline, font, level_color);
    // Gray at the last level, white when the coins are there, red when they are not.
    let cost_color = match (row.maxed, row.affordable) {
        (true, _) => GRAY,
        (false, true) => WHITE,
        (false, false) => REFUSAL,
    };
    draw_text(&row.cost, columns.cost_x, baseline, font, cost_color);
    let effect = fit_line(&row.effect, columns.effect_width, &measure_row);
    draw_text(&effect, columns.effect_x, baseline, font, GOLD);
}

/// Small triangles at the right of the headings when the list scrolls: up when upgrades are
/// above the first visible row, down when they are below the last.
fn draw_scroll_marks(layout: &ShopLayout) {
    let right = layout.panel.right() - 32.0;
    let center = layout.header.y - 6.0;
    if layout.more_above {
        draw_triangle(
            vec2(right - 20.0, center + 5.0),
            vec2(right - 8.0, center + 5.0),
            vec2(right - 14.0, center - 5.0),
            GOLD,
        );
    }
    if layout.more_below {
        draw_triangle(
            vec2(right - 6.0, center - 5.0),
            vec2(right + 6.0, center - 5.0),
            vec2(right, center + 5.0),
            GOLD,
        );
    }
}
