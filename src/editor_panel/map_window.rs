//! The Dungeon map window of the editor (`docs/dungeon-map.md`): every room the session knows,
//! saved or not, in the cell the map layout gives it, drawn with the same `MapGrid` as the map
//! screen in play. The room being edited has a gold outline; a room that shares a cell with
//! another, or that no exit leads to, says so. Clicking a room asks to open it, through the same
//! unsaved-changes guard as the Open room button.
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::dungeon_map::{EditorMap, EditorMapRoom, MapGrid, MapLink};
use hollow_steps::room::RoomId;
use macroquad::math::{vec2, Rect, Vec2};

use super::widgets::{panel_label, probe};

const WINDOW_SIZE: [f32; 2] = [520.0, 380.0];
/// A box narrower than this is too small to hold its name.
const MIN_TEXT_BOX: f32 = 44.0;
const NAME_SIZE: f32 = 13.0;
const TAG_SIZE: f32 = 11.0;

const FILL: egui::Color32 = egui::Color32::from_rgb(52, 60, 96);
const FILL_CONFLICT: egui::Color32 = egui::Color32::from_rgb(110, 48, 56);
const EDGE: egui::Color32 = egui::Color32::from_rgb(150, 160, 205);
const LINK: egui::Color32 = egui::Color32::from_rgb(120, 128, 165);
const CURRENT: egui::Color32 = egui::Color32::GOLD;
const CONFLICT: egui::Color32 = egui::Color32::from_rgb(255, 140, 140);
const NO_ENTRANCE: egui::Color32 = egui::Color32::from_rgb(255, 190, 110);

/// Draws the window while it is open (and no exit link is in progress, when opening another
/// room would break the flow). A click on another room sets `open` to it.
pub(super) fn draw_map_window(
    ctx: &egui::Context,
    editor: &EditorSession,
    state: &mut EditorUi,
    open: &mut Option<RoomId>,
) {
    if !state.checks.map_window || editor.link_active() {
        return;
    }
    let map = EditorMap::new(&editor.dungeon_map(), editor.id);
    let mut shown = true;
    egui::Window::new("Dungeon map")
        .open(&mut shown)
        .default_size(WINDOW_SIZE)
        .min_size([240.0, 200.0])
        .collapsible(false)
        .show(ctx, |ui| {
            if let Some(room) = draw_map(ui, &map) {
                if room != editor.id {
                    *open = Some(room);
                }
            }
        });
    state.checks.map_window = shown;
}

/// The legend and the map; the room clicked this frame, if any.
fn draw_map(ui: &mut egui::Ui, map: &EditorMap) -> Option<RoomId> {
    panel_label(ui, "Click a room to open it. Gold: the room being edited.");
    ui.horizontal_wrapped(|ui| {
        ui.colored_label(CONFLICT, "red: shares a map cell");
        ui.colored_label(NO_ENTRANCE, "orange: no exit leads here");
    });
    if map.rooms.is_empty() {
        panel_label(ui, "No rooms.");
        return None;
    }
    let size = ui.available_size().max(egui::vec2(200.0, 150.0));
    let (response, painter) = ui.allocate_painter(size, egui::Sense::click());
    probe(ui, || (egui::Id::new("map window canvas"), response.rect));
    let grid = MapGrid::fit(to_rect(response.rect), &map.cells());

    for link in &map.links {
        draw_link(&painter, &grid, link);
    }
    for room in &map.rooms {
        let rect = to_egui(grid.room_rect(room.cell));
        probe(ui, || (egui::Id::new(("map room", room.id)), rect));
        draw_room(&painter, rect, room);
    }

    let at = |pos: egui::Pos2| map.room_at(&grid, vec2(pos.x, pos.y));
    let hovered = response.hover_pos().and_then(at);
    if let Some(name) = hovered.and_then(|id| map.rooms.iter().find(|room| room.id == id)) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        response.clone().on_hover_text(&name.name);
    }
    if response.clicked() {
        return response.interact_pointer_pos().and_then(at);
    }
    None
}

fn draw_room(painter: &egui::Painter, rect: egui::Rect, room: &EditorMapRoom) {
    let fill = if room.conflict { FILL_CONFLICT } else { FILL };
    let radius = egui::CornerRadius::same(4);
    painter.rect_filled(rect, radius, fill);
    painter.rect_stroke(
        rect,
        radius,
        egui::Stroke::new(1.5_f32, EDGE),
        egui::StrokeKind::Inside,
    );
    if room.current {
        painter.rect_stroke(
            rect.expand(2.0),
            radius,
            egui::Stroke::new(3.0_f32, CURRENT),
            egui::StrokeKind::Outside,
        );
    }
    if rect.width() < MIN_TEXT_BOX {
        return;
    }
    let width = rect.width() - 8.0;
    let name = painter.layout_job(truncated(
        &room.name,
        NAME_SIZE,
        egui::Color32::WHITE,
        width,
    ));
    painter.galley(
        egui::pos2(rect.center().x - name.size().x / 2.0, rect.top() + 5.0),
        name,
        egui::Color32::WHITE,
    );
    // What is wrong with the room, from the bottom up.
    let tags = [
        (room.conflict, "same cell", CONFLICT),
        (room.no_entrance, "no way in", NO_ENTRANCE),
    ];
    let mut bottom = rect.bottom() - 4.0;
    for (_, text, color) in tags.iter().filter(|tag| tag.0) {
        let tag = painter.layout_job(truncated(text, TAG_SIZE, *color, width));
        bottom -= tag.size().y;
        painter.galley(
            egui::pos2(rect.center().x - tag.size().x / 2.0, bottom),
            tag,
            *color,
        );
    }
}

/// `text` on one line, cut with an ellipsis at `width`.
fn truncated(text: &str, size: f32, color: egui::Color32, width: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple_singleline(
        text.to_owned(),
        egui::FontId::proportional(size),
        color,
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(width.max(1.0));
    job
}

/// An exit between two rooms: a line from box to box, with a head at the target of a one-way one.
fn draw_link(painter: &egui::Painter, grid: &MapGrid, link: &MapLink) {
    let from = grid.center(link.from_cell);
    let to = grid.center(link.to_cell);
    let length = (to - from).length();
    if length <= f32::EPSILON {
        return;
    }
    let stroke = egui::Stroke::new(2.0_f32, LINK);
    painter.line_segment([to_pos(from), to_pos(to)], stroke);
    if link.one_way {
        let way = (to - from) / length;
        let reach = grid.box_size() / 2.0 / way.x.abs().max(way.y.abs());
        let tip = to - way * reach;
        let size = (grid.box_size() * 0.25).clamp(5.0, 12.0);
        let base = tip - way * size;
        let side = vec2(-way.y, way.x) * size * 0.5;
        painter.add(egui::Shape::convex_polygon(
            vec![to_pos(tip), to_pos(base + side), to_pos(base - side)],
            LINK,
            egui::Stroke::NONE,
        ));
    }
}

fn to_pos(point: Vec2) -> egui::Pos2 {
    egui::pos2(point.x, point.y)
}

fn to_rect(rect: egui::Rect) -> Rect {
    Rect::new(rect.min.x, rect.min.y, rect.width(), rect.height())
}

fn to_egui(rect: Rect) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(rect.x, rect.y), egui::vec2(rect.w, rect.h))
}
