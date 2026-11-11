//! The panel fits the window, scrolls, and its pickers stay on screen.
use super::*;

#[test]
fn panel_fits_viewport_and_reserves_save_outside_scroll_bounds() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = EditorUi {
        new_entity: "雪longname".repeat(512),
        ..Default::default()
    };
    for size in [
        egui::vec2(1920.0, 1080.0),
        egui::vec2(960.0, 540.0),
        egui::vec2(640.0, 360.0),
        egui::vec2(360.0, 240.0),
        egui::vec2(200.0, 100.0),
    ] {
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        let save = button(&ctx, "Save");
        let (viewport, _, _, _) = metrics(&ctx);
        assert!(
            save.min.y >= 0.0 && save.max.y <= size.y,
            "{size:?}: {save:?}"
        );
        assert!(viewport.max.y <= save.min.y, "scroll area covers Save");
        assert!(save.max.x <= size.x && save.min.x >= 0.0);
        let open = button(&ctx, "Open room");
        let room_picker: (egui::Rect, egui::Id) = ctx.data(|d| {
            d.get_temp(egui::Id::new(("rooms",)).with("dropdown"))
                .unwrap()
        });
        assert!(
            room_picker.0.max.y <= open.min.y,
            "room list covers Open room"
        );
        let input = ctx
            .read_response(egui::Id::new(("new entity name",)))
            .unwrap()
            .rect;
        let entity_picker: (egui::Rect, egui::Id) = ctx.data(|d| {
            d.get_temp(egui::Id::new(("definitions",)).with("dropdown"))
                .unwrap()
        });
        assert!(
            entity_picker.0.max.y <= input.min.y,
            "entity list covers name input"
        );
        assert!(input.max.x <= size.x, "long text expands panel");
    }
}

#[test]
fn wheel_and_scrollbar_move_controls_and_resize_clamps_scroll() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = EditorUi::default();
    let size = egui::vec2(960.0, 540.0);
    for _ in 0..3 {
        frame(&ctx, size, vec![], editor, &mut state);
    }
    let (viewport, _, before, _) = metrics(&ctx);
    frame(
        &ctx,
        size,
        vec![
            egui::Event::PointerMoved(viewport.center()),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -28.0),
                modifiers: Default::default(),
            },
        ],
        editor,
        &mut state,
    );
    for _ in 0..20 {
        frame(&ctx, size, vec![], editor, &mut state);
    }
    let after = metrics(&ctx).2.y;
    assert!(
        after > before.y && after <= before.y + 29.0,
        "wheel moved {after}"
    );
    // Drag the real egui scrollbar, rather than injecting its saved state.
    let bar_top = egui::pos2(viewport.right() + 6.0, viewport.top() + 20.0);
    let bar_bottom = egui::pos2(bar_top.x, viewport.bottom() - 12.0);
    frame(&ctx, size, pointer(bar_top, true), editor, &mut state);
    frame(
        &ctx,
        size,
        vec![egui::Event::PointerMoved(bar_bottom)],
        editor,
        &mut state,
    );
    frame(&ctx, size, pointer(bar_bottom, false), editor, &mut state);
    assert!(
        metrics(&ctx).2.y > after + 50.0,
        "scrollbar did not drag: {:?}",
        metrics(&ctx)
    );
    let expanded = egui::vec2(1920.0, 1800.0);
    for _ in 0..3 {
        frame(&ctx, expanded, vec![], editor, &mut state);
    }
    assert_eq!(metrics(&ctx).2.y, 0.0);
}

#[test]
fn searchable_dropdown_has_eight_rows_and_reaches_last_result_near_bottom() {
    let ctx = egui::Context::default();
    let id = egui::Id::new("test picker");
    let names = (0..40)
        .map(|i| format!("Enemy {i}"))
        .chain(["Item: Dash".to_owned()])
        .collect::<Vec<_>>();
    let mut list = SearchList::default();
    let size = egui::vec2(360.0, 360.0);
    let draw = |events: Vec<egui::Event>, list: &mut SearchList| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.add_space(200.0);
                    searchable_list(ui, id, "Entity", &names, list);
                });
            },
        );
    };
    for _ in 0..3 {
        draw(vec![], &mut list);
    }
    let (rect, _): (egui::Rect, egui::Id) = ctx.data(|d| d.get_temp(id.with("dropdown")).unwrap());
    draw(pointer(rect.center(), true), &mut list);
    draw(pointer(rect.center(), false), &mut list);
    for _ in 0..3 {
        draw(vec![], &mut list);
    }
    let (viewport, content, _, scroll_id): ScrollMetrics =
        ctx.data(|d| d.get_temp(id.with("result metrics")).unwrap());
    assert!(viewport.height() >= 8.0 * 24.0, "{viewport:?}");
    assert!(
        viewport.top() >= 0.0 && viewport.bottom() <= size.y,
        "popup offscreen: {viewport:?}"
    );
    let mut scroll = egui::scroll_area::State::load(&ctx, scroll_id).unwrap();
    scroll.offset.y = content.y;
    scroll.store(&ctx, scroll_id);
    for _ in 0..3 {
        draw(vec![], &mut list);
    }
    let last: egui::Rect =
        ctx.data(|d| d.get_temp(egui::Id::new(("choice", "Item: Dash"))).unwrap());
    let (viewport, _, _, _): ScrollMetrics =
        ctx.data(|d| d.get_temp(id.with("result metrics")).unwrap());
    assert!(
        viewport.contains(last.center()),
        "last result unreachable: {last:?} {viewport:?}"
    );
    draw(pointer(last.center(), true), &mut list);
    draw(pointer(last.center(), false), &mut list);
    assert_eq!(list.selected, Some(40));
    list.query = "missing".into();
    draw(vec![], &mut list);
    assert_eq!(list.selected, None);
    draw(pointer(rect.center(), true), &mut list);
    draw(pointer(rect.center(), false), &mut list);
    for _ in 0..3 {
        draw(vec![], &mut list);
    }
    assert!(ctx
        .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("choice", "No matches"))))
        .is_some());
}
