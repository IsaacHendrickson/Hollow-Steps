//! Macroquad drawing of the inventory's Charms section: one row per owned charm (color swatch,
//! name, mana cost, effect, an Equipped marker) with the selected one lit, and under the list
//! the selected charm's description, the maximum mana now and after, and the last refusal. The
//! positions come from `hollow_steps::charm_menu::layout`, which the mouse uses too.

use hollow_steps::charm_menu::{
    fit_line, mana_text, row_columns, wrap_lines, InventoryLayout, DESCRIPTION_LINES, MESSAGE_LINES,
};
use hollow_steps::charms::{number_text, Charm};
use hollow_steps::game::GameState;
use macroquad::prelude::*;

use crate::hud::centered_baseline;

const ROW_FONT: u16 = 20;
const DETAIL_FONT: u16 = 20;
const MANA_FONT: u16 = 22;
const HEADING_FONT: u16 = 24;
const EQUIPPED: Color = Color::new(0.47, 0.86, 0.55, 1.0);
const REFUSAL: Color = Color::new(1.0, 0.55, 0.4, 1.0);

fn measure(text: &str, font_size: u16) -> f32 {
    measure_text(text, None, font_size, 1.0).width
}

/// The Charms heading, the list and the details of the selected charm, laid out by `layout`.
pub fn draw_charms(game: &GameState, layout: &InventoryLayout) {
    let heading = layout.charms_header;
    draw_text(
        "Charms",
        heading.x,
        heading.y,
        f32::from(HEADING_FONT),
        WHITE,
    );
    let owned = game.owned_charms();
    if owned.is_empty() {
        let note = layout.empty_note;
        draw_text("No charms owned yet", note.x, note.y, 22.0, LIGHTGRAY);
        return;
    }
    let selected = game.charm_menu().selected(owned.len());
    for row in &layout.rows {
        let charm = owned[row.index];
        draw_row(
            row.rect,
            Some(row.index) == selected,
            charm,
            game.charm_state().is_equipped(&charm.id),
        );
    }
    draw_scroll_marks(layout);
    let Some(charm) = selected.map(|index| owned[index]) else {
        return;
    };
    let measure_detail = |text: &str| measure(text, DETAIL_FONT);
    let lines = wrap_lines(
        &charm.description,
        layout.text_width,
        DESCRIPTION_LINES,
        &measure_detail,
    );
    for (line, at) in lines.iter().zip(layout.description) {
        draw_text(line, at.x, at.y, f32::from(DETAIL_FONT), LIGHTGRAY);
    }
    if let Some((now, after)) = game.charm_mana_preview() {
        let at = layout.mana;
        draw_text(
            mana_text(now, after),
            at.x,
            at.y,
            f32::from(MANA_FONT),
            WHITE,
        );
    }
    if let Some(message) = game.charm_menu().message() {
        let measure_message = |text: &str| measure(text, DETAIL_FONT);
        let lines = wrap_lines(message, layout.text_width, MESSAGE_LINES, &measure_message);
        for (line, at) in lines.iter().zip(layout.message) {
            draw_text(line, at.x, at.y, f32::from(DETAIL_FONT), REFUSAL);
        }
    }
}

fn draw_row(rect: Rect, selected: bool, charm: &Charm, equipped: bool) {
    let (fill, border) = if selected {
        (Color::from_rgba(70, 78, 120, 255), GOLD)
    } else {
        (Color::from_rgba(44, 50, 78, 255), GRAY)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
    let columns = row_columns(rect);
    let [red, green, blue, alpha] = charm.color;
    let swatch = columns.swatch;
    draw_rectangle(
        swatch.x,
        swatch.y,
        swatch.w,
        swatch.h,
        Color::from_rgba(red, green, blue, alpha),
    );
    draw_rectangle_lines(swatch.x, swatch.y, swatch.w, swatch.h, 1.0, WHITE);
    let baseline = centered_baseline(rect.center().y, ROW_FONT);
    let font = f32::from(ROW_FONT);
    let measure_row = |text: &str| measure(text, ROW_FONT);
    let name = fit_line(&charm.name, columns.name_width, &measure_row);
    draw_text(&name, columns.name_x, baseline, font, WHITE);
    let cost = format!("{} mana", number_text(charm.mana_cost));
    draw_text(&cost, columns.cost_x, baseline, font, LIGHTGRAY);
    let effect = fit_line(&charm.effect.text(), columns.effect_width, &measure_row);
    draw_text(&effect, columns.effect_x, baseline, font, GOLD);
    if equipped {
        let marker = "Equipped";
        let width = measure(marker, ROW_FONT);
        draw_text(
            marker,
            columns.marker_right - width,
            baseline,
            font,
            EQUIPPED,
        );
    }
}

/// Small triangles at the right of the heading when the list scrolls: up when charms are above
/// the first visible row, down when they are below the last.
fn draw_scroll_marks(layout: &InventoryLayout) {
    let right = layout.panel.right() - 32.0;
    let center = layout.charms_header.y - 8.0;
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
