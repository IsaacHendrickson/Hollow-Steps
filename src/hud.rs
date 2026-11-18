//! Heads-up display drawn in virtual-resolution space, anchored to the screen edges.

use std::path::Path;

use hollow_steps::boss_special::SpecialAttack;
use hollow_steps::camera::{anchored, Anchor};
use hollow_steps::events::LocatedEvent;
use hollow_steps::game::GameState;
use hollow_steps::hud_bars::{fillable_bar, layout_bars, HudConfig};
use hollow_steps::hud_damage::HudFeedback;
use hollow_steps::hud_layout::{boss_caption, boss_fill};
use hollow_steps::keybinds::{hint_line, key_label, Action, Keybinds, Slot};
use hollow_steps::particles::ParticleConfig;
use hollow_steps::pause_menu::{
    button_rects, panel_rect, slot_rects, status_rect, title_baseline, PauseItem, PauseMenu,
    PauseScreen,
};
use hollow_steps::room::Room;
use hollow_steps::room_name::room_title;
use macroquad::prelude::*;

use crate::effects::draw_particles;
use crate::hud_bars_draw::draw_bars;
use crate::hud_charms::draw_charms;

/// The gold coin is drawn 9 pixels in radius with its center 2 pixels below the counter row's
/// anchor, so its top is 7 pixels above the anchor; lowering the row by that much leaves
/// exactly one `gap` between the mana bar and the coin.
const COUNTER_INSET: f32 = 7.0;

/// The HUD's settings (`data/hud.ron`) and the health bar's damage feedback: its shake and a
/// particle emitter of its own that draws in screen space, never through the world camera.
pub struct Hud {
    config: HudConfig,
    feedback: HudFeedback,
}

impl Hud {
    /// Loads `hud.ron` from `assets` (or the defaults); `particles` are the particle settings
    /// the world's effects run on, which hold the `hud_damage` burst. An invalid file falls
    /// back to the defaults and the reason comes back so it can be shown on screen.
    pub fn load(assets: &Path, particles: &ParticleConfig) -> (Self, Option<String>) {
        let (config, error) = match HudConfig::load(assets) {
            Ok(config) => (config, None),
            Err(error) => (HudConfig::default(), Some(error)),
        };
        let hud = Self {
            config,
            feedback: HudFeedback::new(particles.clone()),
        };
        (hud, error)
    }

    /// Re-reads the settings and takes the current particle settings. Nothing changes if the
    /// file is invalid.
    pub fn reload(&mut self, assets: &Path, particles: &ParticleConfig) -> Result<(), String> {
        self.config = HudConfig::load(assets)?;
        self.feedback.set_particle_config(particles.clone());
        Ok(())
    }

    /// Reacts to a frame's events: damage shakes the health bar and throws off particles.
    /// `base_max_health` is the maximum health without charms: with the maximum below it the end
    /// of the bar is locked, and the burst is measured against the part that can hold health.
    pub fn handle(
        &mut self,
        events: &[LocatedEvent],
        view: Vec2,
        max_health: f32,
        base_max_health: f32,
    ) {
        let bar = fillable_bar(self.config.health_anchor(view), max_health, base_max_health);
        self.feedback.handle(&self.config, bar, max_health, events);
    }

    /// Moves the shake and the particles on by one fixed tick.
    pub fn update(&mut self, delta_seconds: f32) {
        self.feedback.update(&self.config, delta_seconds);
    }

    /// Stops the shake and drops the particles, for when the world changes under them.
    pub fn clear(&mut self) {
        self.feedback.clear();
    }

    /// Draws the damage particles. Call after the HUD, with the HUD's camera set.
    pub fn draw_particles(&self) {
        draw_particles(self.feedback.particles());
    }
}

pub fn draw_hud(game: &GameState, hud: &Hud, view: Vec2) {
    let room = game.room();
    let player = game.player();
    let margin = 24.0;

    // Top left: the health and mana bars, then the coin and item counters under them.
    let config = &hud.config;
    let layout = layout_bars(config, game.bar_values(), config.health_anchor(view));
    // The counters go under the bars where they rest; only the health bar shakes.
    let counters = anchored(
        view,
        Anchor::TopLeft,
        Vec2::ZERO,
        vec2(
            config.margin.0,
            layout.bounds().bottom() + config.gap + COUNTER_INSET,
        ),
    );
    let shaken = layout.with_health_offset(hud.feedback.bar_offset(config));
    draw_bars(&shaken, &config.colors);
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
    let title = room_title(room.id, room.name.as_deref());
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
    let hint = hint_line(game.keybinds());
    let at = anchored(view, Anchor::BottomLeft, Vec2::ZERO, vec2(margin, 14.0));
    draw_text(
        &hint,
        at.x,
        at.y,
        18.0,
        Color::from_rgba(170, 180, 210, 255),
    );
    draw_boss_hud(room, view);
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
    // The panel is as tall as its content needs; the same layout places the mouse's rows.
    let layout = game.inventory_layout(view);
    let panel = layout.panel;
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, GRAY);
    draw_text("Inventory", layout.title.x, layout.title.y, 36.0, WHITE);
    draw_text(
        format!("Coins: {}", game.coin_count()),
        layout.coins.x,
        layout.coins.y,
        26.0,
        GOLD,
    );
    let header = layout.items_header;
    draw_text("Collected items", header.x, header.y, 24.0, WHITE);
    let items = game.player().collected_items();
    if items.is_empty() {
        let at = layout.item(0);
        draw_text("No items collected yet", at.x, at.y, 22.0, LIGHTGRAY);
    } else {
        for (index, item) in items.iter().enumerate() {
            let at = layout.item(index);
            draw_text(item.name(), at.x, at.y, 22.0, GOLD);
        }
    }
    draw_charms(game, &layout);
    let hint = if game.owned_charms().is_empty() {
        "Paused  |  I or Esc to return"
    } else {
        "Paused  |  Up/Down, Enter: equip  |  I or Esc to return"
    };
    draw_text(hint, layout.footer.x, layout.footer.y, 22.0, LIGHTGRAY);
}

/// The pause menu over the frozen picture: a dim veil, the panel, its title and the buttons of
/// the open screen with the selected one lit. Draws nothing while the menu is closed. The
/// layout comes from `pause_menu`, which the mouse hit-testing uses too.
pub fn draw_pause_menu(menu: &PauseMenu, binds: &Keybinds, view: Vec2) {
    let Some(screen) = menu.screen() else { return };
    draw_rectangle(0.0, 0.0, view.x, view.y, Color::from_rgba(0, 0, 0, 180));
    let panel = panel_rect(screen, view);
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, GRAY);
    let title = screen.title();
    let title_size = measure_text(title, None, 36, 1.0);
    draw_text(
        title,
        panel.center().x - title_size.width / 2.0,
        panel.y + title_baseline(),
        36.0,
        WHITE,
    );
    // The Settings screen lists many rows, so its text is smaller.
    let font_size = match screen {
        PauseScreen::Main => 28,
        PauseScreen::Settings => 22,
    };
    for (item, rect) in button_rects(screen, view) {
        let selected = menu.selected_item() == Some(item);
        match item {
            PauseItem::Bind(action) => {
                draw_bind_row(menu, binds, action, rect, selected, font_size);
            }
            _ => {
                draw_pause_button(rect, selected);
                draw_text_centered(item.label(), rect, font_size, WHITE);
            }
        }
    }
    if screen == PauseScreen::Settings {
        draw_settings_status(menu, view);
    }
}

fn draw_pause_button(rect: Rect, selected: bool) {
    let (fill, border) = if selected {
        (Color::from_rgba(70, 78, 120, 255), GOLD)
    } else {
        (Color::from_rgba(44, 50, 78, 255), GRAY)
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, border);
}

/// One keybind row: the action's name on the left and its primary and alternate key buttons on
/// the right. The highlighted key button of the selected row is gold; the one waiting for a key
/// is orange and says so.
fn draw_bind_row(
    menu: &PauseMenu,
    binds: &Keybinds,
    action: Action,
    row: Rect,
    selected: bool,
    font_size: u16,
) {
    let label_color = if selected { GOLD } else { WHITE };
    draw_text(
        action.label(),
        row.x + 8.0,
        centered_baseline(row.center().y, font_size),
        f32::from(font_size),
        label_color,
    );
    for slot in Slot::ALL {
        let rect = slot_rects(row)[slot.index()];
        let waiting = menu.listening() == Some((action, slot));
        if waiting {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::from_rgba(96, 62, 40, 255),
            );
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, ORANGE);
            draw_text_centered("Press a key", rect, font_size, WHITE);
        } else {
            draw_pause_button(rect, selected && menu.slot() == slot);
            let key = binds.key(action, slot);
            let color = if key.is_some() { WHITE } else { GRAY };
            draw_text_centered(key_label(key), rect, font_size, color);
        }
    }
}

/// The Settings screen's message line: the message if there is one, else what to do next.
fn draw_settings_status(menu: &PauseMenu, view: Vec2) {
    let area = status_rect(view);
    let (text, color) = match (menu.note(), menu.listening()) {
        (Some(note), _) => (note.to_owned(), Color::from_rgba(240, 200, 120, 255)),
        (None, Some((action, slot))) => {
            let clear = if slot == Slot::Alternate {
                ", Delete clears"
            } else {
                ""
            };
            (
                format!("{}: press a key. Esc cancels{clear}.", action.label()),
                WHITE,
            )
        }
        (None, None) => (
            "Enter or click to rebind a key. Left/Right switch keys.".to_owned(),
            LIGHTGRAY,
        ),
    };
    let text = fit_text(&text, area.w, 18);
    draw_text(
        &text,
        area.x,
        centered_baseline(area.center().y, 18),
        18.0,
        color,
    );
}

/// `text` cut short with "..." so it is no wider than `max_width` at `font_size`.
fn fit_text(text: &str, max_width: f32, font_size: u16) -> String {
    let fits = |candidate: &str| measure_text(candidate, None, font_size, 1.0).width <= max_width;
    if fits(text) {
        return text.to_owned();
    }
    let mut shortened = text.to_owned();
    while shortened.pop().is_some() {
        let candidate = format!("{}...", shortened.trim_end());
        if fits(&candidate) {
            return candidate;
        }
    }
    "...".to_owned()
}

/// `text` centered in `rect`.
fn draw_text_centered(text: &str, rect: Rect, font_size: u16, color: Color) {
    let width = measure_text(text, None, font_size, 1.0).width;
    draw_text(
        text,
        rect.center().x - width / 2.0,
        centered_baseline(rect.center().y, font_size),
        f32::from(font_size),
        color,
    );
}

/// The baseline that centers a line of text on `center_y`. It is the same for every string at a
/// font size, so a word with descenders (g, j, p) does not sit higher than one without.
pub(crate) fn centered_baseline(center_y: f32, font_size: u16) -> f32 {
    let reference = measure_text("Ag", None, font_size, 1.0);
    center_y - reference.height / 2.0 + reference.offset_y
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
