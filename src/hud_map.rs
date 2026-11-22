//! Macroquad drawing of the map screen over the frozen world: the rooms the player has seen as
//! filled boxes with their names, the rooms one exit away as empty boxes, the exits between them
//! (an arrow head on a one-way exit, the key number on a locked one) and a mark on the room the
//! player is in. Rooms not seen and not next to one that was are not drawn at all.
//!
//! What goes where comes from `hollow_steps::dungeon_map` (`screen_layout`, `MapGrid`, `PlayMap`),
//! which is tested without a window; this file only paints it, in virtual-screen coordinates.

use hollow_steps::charm_menu::fit_line;
use hollow_steps::dungeon_map::{screen_layout, MapGrid, MapLink, PlayMap, PlayRoom};
use hollow_steps::game::GameState;
use hollow_steps::keybinds::{key_label, Action, Slot};
use macroquad::prelude::*;

use crate::hud::centered_baseline;

const VISITED_FILL: Color = Color::new(0.27, 0.31, 0.47, 1.0);
const VISITED_EDGE: Color = Color::new(0.62, 0.67, 0.85, 1.0);
const UNSEEN_EDGE: Color = Color::new(0.45, 0.48, 0.60, 1.0);
const LINK: Color = Color::new(0.55, 0.60, 0.78, 1.0);
const NAME_FONT: u16 = 18;
/// A box narrower than this has no room for a name.
const MIN_NAME_BOX: f32 = 44.0;

fn measure(text: &str, font_size: u16) -> f32 {
    measure_text(text, None, font_size, 1.0).width
}

pub fn draw_map(game: &GameState, view: Vec2) {
    draw_rectangle(0.0, 0.0, view.x, view.y, Color::from_rgba(0, 0, 0, 180));
    let screen = screen_layout(view);
    let panel = screen.panel;
    draw_rectangle(
        panel.x,
        panel.y,
        panel.w,
        panel.h,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(panel.x, panel.y, panel.w, panel.h, 2.0, GRAY);
    draw_text("Map", screen.title.x, screen.title.y, 36.0, WHITE);

    let play = game.play_map();
    if play.rooms.is_empty() {
        let note = "Nothing explored yet";
        let at = screen.area.center();
        draw_text(note, at.x - measure(note, 24) / 2.0, at.y, 24.0, LIGHTGRAY);
    } else {
        draw_play_map(&play, &MapGrid::fit(screen.area, &play.cells()));
    }

    let key = key_label(game.keybinds().key(Action::Map, Slot::Primary));
    let hint = format!("Paused  |  {key} or Esc to return");
    draw_text(&hint, screen.footer.x, screen.footer.y, 22.0, LIGHTGRAY);
}

fn draw_play_map(play: &PlayMap, grid: &MapGrid) {
    // Exits first, so the boxes sit on top of their ends.
    for link in &play.links {
        draw_link(grid, link);
    }
    for room in &play.rooms {
        draw_room(grid, room);
    }
    for link in play.links.iter().filter(|link| link.lock.is_some()) {
        draw_key_number(grid, link);
    }
}

/// The unit direction from one box center to the other, and how far from a center the box's edge
/// is along it.
fn direction(grid: &MapGrid, link: &MapLink) -> Option<(Vec2, Vec2, f32)> {
    let from = grid.center(link.from_cell);
    let to = grid.center(link.to_cell);
    let length = (to - from).length();
    if length <= f32::EPSILON {
        return None;
    }
    let way = (to - from) / length;
    let edge = grid.box_size() / 2.0 / way.x.abs().max(way.y.abs());
    Some((from, way, edge))
}

fn draw_link(grid: &MapGrid, link: &MapLink) {
    let to = grid.center(link.to_cell);
    let Some((from, way, edge)) = direction(grid, link) else {
        return;
    };
    draw_line(
        from.x,
        from.y,
        to.x,
        to.y,
        (grid.pitch() * 0.04).max(2.0),
        LINK,
    );
    if link.one_way {
        // The head points into the target, at the edge of its box.
        let tip = to - way * edge;
        let size = (grid.box_size() * 0.28).clamp(5.0, 14.0);
        let base = tip - way * size;
        let side = vec2(-way.y, way.x) * size * 0.5;
        draw_triangle(tip, base + side, base - side, GOLD);
    }
}

fn draw_room(grid: &MapGrid, room: &PlayRoom) {
    let rect = grid.room_rect(room.cell);
    if room.visited {
        draw_rectangle(rect.x, rect.y, rect.w, rect.h, VISITED_FILL);
    }
    let edge = if room.visited {
        VISITED_EDGE
    } else {
        UNSEEN_EDGE
    };
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, edge);
    if let Some(name) = room.name.as_deref().filter(|_| rect.w >= MIN_NAME_BOX) {
        let fitted = fit_line(name, rect.w - 8.0, &|text| measure(text, NAME_FONT));
        let width = measure(&fitted, NAME_FONT);
        let baseline = centered_baseline(rect.center().y, NAME_FONT);
        draw_text(
            &fitted,
            rect.center().x - width / 2.0,
            baseline,
            f32::from(NAME_FONT),
            WHITE,
        );
    }
    if room.current {
        draw_rectangle_lines(
            rect.x - 3.0,
            rect.y - 3.0,
            rect.w + 6.0,
            rect.h + 6.0,
            3.0,
            GOLD,
        );
        let radius = (rect.w * 0.07).clamp(3.0, 7.0);
        draw_circle(rect.center().x, rect.bottom() - radius - 4.0, radius, GOLD);
    }
}

/// The key number of a locked exit, in a small badge on the exit, nearer its own room so the two
/// exits of a pair do not cover each other.
fn draw_key_number(grid: &MapGrid, link: &MapLink) {
    let (Some(key), Some((from, way, edge))) = (link.lock, direction(grid, link)) else {
        return;
    };
    let length = (grid.center(link.to_cell) - from).length();
    let gap = (length - 2.0 * edge).max(0.0);
    let at = from + way * (edge + gap * 0.3);
    let radius = (grid.pitch() * 0.1).clamp(7.0, 11.0);
    draw_circle(at.x, at.y, radius, Color::from_rgba(28, 32, 50, 255));
    draw_circle_lines(at.x, at.y, radius, 2.0, GOLD);
    let text = key.to_string();
    let font = 16;
    let baseline = centered_baseline(at.y, font);
    draw_text(
        &text,
        at.x - measure(&text, font) / 2.0,
        baseline,
        f32::from(font),
        GOLD,
    );
}
