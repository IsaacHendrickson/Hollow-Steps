//! Headless layout and input tests for the whole editor panel, one file per area. The helpers
//! here run a frame of the panel and find widgets by the rectangles `widgets::probe` records.
use super::entity::pending_error;
use super::property::{BehaviorCreation, BehaviorSource};
use super::test_support::editing_game as editor;
use super::widgets::{pose_dropdown, searchable_list};
use super::{draw_panel, EditorSession, EditorUi, PanelResponse, PropertyUi};
use crate::SearchList;
use egui_macroquad::egui;

mod behavior;
mod entity;
mod layout;

type ScrollMetrics = (egui::Rect, egui::Vec2, egui::Vec2, egui::Id);

fn frame(
    ctx: &egui::Context,
    size: egui::Vec2,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> PanelResponse {
    let mut response = None;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ctx| {
            response = Some(draw_panel(ctx, editor, state));
        },
    );
    response.unwrap()
}
fn metrics(ctx: &egui::Context) -> ScrollMetrics {
    ctx.data(|data| data.get_temp(egui::Id::new("panel scroll")).unwrap())
}
fn button(ctx: &egui::Context, name: &str) -> egui::Rect {
    ctx.data(|data| data.get_temp(egui::Id::new(("button", name))).unwrap())
}
fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        },
    ]
}

fn click(
    ctx: &egui::Context,
    size: egui::Vec2,
    pos: egui::Pos2,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    frame(ctx, size, pointer(pos, true), editor, state);
    frame(ctx, size, pointer(pos, false), editor, state);
    frame(ctx, size, vec![], editor, state);
}
