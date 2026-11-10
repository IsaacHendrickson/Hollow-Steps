mod support;

use std::sync::Arc;

use hollow_steps::body::{BodyInstance, BodyTemplate, BodyTemplates, BodyVisual};
use hollow_steps::body_debug::{
    body_primitives, label_requests, nearest_entity, ColorSlot, OverlayPrimitive, CURRENT_RADIUS,
    FONT_SIZES, LABEL_FONT_SIZE, LABEL_OFFSET, REST_RADIUS, TARGET_RADIUS,
};
use hollow_steps::enemy::Enemy;
use hollow_steps::player::WorldBounds;
use macroquad::math::{Rect, Vec2};

fn body() -> BodyInstance {
    let template: BodyTemplate = ron::from_str(include_str!("fixtures/bodies/small.ron")).unwrap();
    let visual: BodyVisual = ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap();
    BodyInstance::new(
        visual,
        &BodyTemplates::from([("small".into(), Arc::new(template))]),
    )
    .unwrap()
}

#[test]
fn every_overlay_font_request_uses_the_fixed_integer_size_set() {
    assert_eq!(FONT_SIZES, [LABEL_FONT_SIZE]);
    assert!(FONT_SIZES.iter().all(|size| *size > 0));
    let body = body();
    for facing_left in [false, true] {
        for show_labels in [false, true] {
            let primitives = body_primitives(
                &body,
                Vec2::new(100.0, 200.0),
                Vec2::new(40.0, 60.0),
                facing_left,
                show_labels,
            );
            let requests = label_requests(&primitives).collect::<Vec<_>>();
            assert_eq!(
                requests.len(),
                if show_labels { body.points.len() } else { 0 }
            );
            for primitive in &primitives {
                if let OverlayPrimitive::Label {
                    text, font_size, ..
                } = primitive
                {
                    assert!(FONT_SIZES.contains(font_size));
                    assert!(requests.contains(&(text.as_str(), *font_size)));
                }
            }
        }
    }
}

#[test]
fn camera_zoom_only_projects_primitives_and_never_changes_font_requests() {
    let body = body();
    let center = Vec2::new(100.0, 200.0);
    let size = Vec2::new(40.0, 60.0);
    for facing_left in [false, true] {
        let primitives = body_primitives(&body, center, size, facing_left, true);
        let requests = label_requests(&primitives).collect::<Vec<_>>();
        for zoom in [0.125, 0.5, 1.0, 1.37, 4.0, 16.0] {
            let origin = Vec2::new(20.0, 30.0);
            let screen = |point: Vec2| origin + point * zoom;
            let projected = primitives
                .iter()
                .map(|p| p.project(screen, zoom))
                .collect::<Vec<_>>();
            assert_eq!(label_requests(&projected).collect::<Vec<_>>(), requests);
            for (world, projected) in primitives.iter().zip(&projected) {
                match (world, projected) {
                    (
                        OverlayPrimitive::Circle {
                            position,
                            radius,
                            color_slot,
                        },
                        OverlayPrimitive::Circle {
                            position: p,
                            radius: r,
                            color_slot: c,
                        },
                    ) => {
                        assert_eq!(*p, screen(*position));
                        assert_eq!((r, c), (radius, color_slot));
                    }
                    (
                        OverlayPrimitive::Line {
                            from,
                            to,
                            color_slot,
                        },
                        OverlayPrimitive::Line {
                            from: f,
                            to: t,
                            color_slot: c,
                        },
                    ) => {
                        assert_eq!((*f, *t, c), (screen(*from), screen(*to), color_slot));
                    }
                    (
                        OverlayPrimitive::Rect { rect, color_slot },
                        OverlayPrimitive::Rect {
                            rect: r,
                            color_slot: c,
                        },
                    ) => {
                        assert_eq!(r.point(), screen(rect.point()));
                        assert_eq!(r.size(), rect.size() * zoom);
                        assert_eq!(c, color_slot);
                    }
                    (
                        OverlayPrimitive::Label { position, .. },
                        OverlayPrimitive::Label { position: p, .. },
                    ) => {
                        assert_eq!(*p, screen(*position - LABEL_OFFSET) + LABEL_OFFSET);
                    }
                    _ => panic!("projection changed primitive kind"),
                }
            }
            // Generation receives world geometry only; projection cannot feed zoom back into it.
            assert_eq!(
                body_primitives(&body, center, size, facing_left, true),
                primitives
            );
        }
    }
}

#[test]
fn known_template_has_three_dots_and_one_line_per_point_and_one_hitbox() {
    assert_eq!(
        (REST_RADIUS, CURRENT_RADIUS, TARGET_RADIUS),
        (2.0, 3.0, 3.0)
    );
    let body = body();
    let before = format!("{body:?}");
    let center = Vec2::new(100.0, 200.0);
    let size = Vec2::new(40.0, 60.0);
    for labels in [false, true] {
        let primitives = body_primitives(&body, center, size, false, labels);
        assert_eq!(primitives.len(), if labels { 11 } else { 9 });
        for (slot, radius) in [
            (ColorSlot::Rest, REST_RADIUS),
            (ColorSlot::Current, CURRENT_RADIUS),
            (ColorSlot::Target, TARGET_RADIUS),
        ] {
            assert_eq!(primitives.iter().filter(|p| matches!(p,
                OverlayPrimitive::Circle { radius: r, color_slot, .. } if *r == radius && *color_slot == slot
            )).count(), 2);
        }
        assert_eq!(
            primitives
                .iter()
                .filter(|p| matches!(p, OverlayPrimitive::Line { .. }))
                .count(),
            2
        );
        assert_eq!(
            primitives.last(),
            Some(&OverlayPrimitive::Rect {
                rect: Rect::new(80.0, 170.0, 40.0, 60.0),
                color_slot: ColorSlot::Hitbox,
            })
        );
        let names = primitives
            .iter()
            .filter_map(|p| match p {
                OverlayPrimitive::Label { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(names, if labels { vec!["root", "tip"] } else { vec![] });
    }
    assert_eq!(format!("{body:?}"), before);
}

#[test]
fn rest_current_target_lines_and_labels_follow_the_part_mirror_and_scale() {
    let mut body = body();
    body.visual.scale = 2.0;
    body.points.insert("root".into(), Vec2::new(0.2, -0.1));
    body.set_target("root", Vec2::new(0.3, 0.1)).unwrap();
    let center = Vec2::new(100.0, 200.0);
    let size = Vec2::new(40.0, 60.0);
    for facing_left in [false, true] {
        let primitives = body_primitives(&body, center, size, facing_left, true);
        let sign = if facing_left { -1.0 } else { 1.0 };
        let current = center + Vec2::new(sign * 16.0, -12.0);
        let target = center + Vec2::new(sign * 24.0, 12.0);
        for (position, radius, color_slot) in [
            (
                center + Vec2::new(sign * 8.0, -24.0),
                REST_RADIUS,
                ColorSlot::Rest,
            ),
            (current, CURRENT_RADIUS, ColorSlot::Current),
            (target, TARGET_RADIUS, ColorSlot::Target),
        ] {
            assert!(primitives.contains(&OverlayPrimitive::Circle {
                position,
                radius,
                color_slot
            }));
        }
        assert!(primitives.contains(&OverlayPrimitive::Line {
            from: current,
            to: target,
            color_slot: ColorSlot::Target,
        }));
        assert!(primitives.contains(&OverlayPrimitive::Label {
            position: current + LABEL_OFFSET,
            text: "root".into(),
            font_size: LABEL_FONT_SIZE,
            color_slot: ColorSlot::Current,
        }));
        // The eye part is anchored directly to root with no offset.
        let eye = body
            .template
            .parts
            .iter()
            .find(|p| p.name == "eye")
            .unwrap();
        assert_eq!(
            body.part_transform(eye, center, size, facing_left)
                .unwrap()
                .center,
            current
        );
        assert_eq!(
            primitives.last(),
            Some(&OverlayPrimitive::Rect {
                rect: Rect::new(80.0, 170.0, 40.0, 60.0),
                color_slot: ColorSlot::Hitbox,
            })
        );
        // Editor zoom scales geometry, while pixel radii and label offsets stay fixed.
        let zoomed = body_primitives(&body, center * 0.5, size * 0.5, facing_left, true);
        assert!(zoomed.contains(&OverlayPrimitive::Circle {
            position: current * 0.5,
            radius: CURRENT_RADIUS,
            color_slot: ColorSlot::Current,
        }));
        assert!(zoomed.contains(&OverlayPrimitive::Label {
            position: current * 0.5 + LABEL_OFFSET,
            text: "root".into(),
            font_size: LABEL_FONT_SIZE,
            color_slot: ColorSlot::Current,
        }));
    }
}

fn enemy(position: Vec2, with_body: bool) -> Enemy {
    let mut draft =
        hollow_steps::definition_editor::DefinitionDraft::load(&support::fixture_assets()).unwrap();
    let path = draft.entity_path("chase").unwrap();
    let definition = draft.entities.get_mut(&path).unwrap();
    definition.body = with_body.then(|| body().visual);
    let bounds = WorldBounds::new(0.0, 960.0, position.y + definition.size.1 / 2.0);
    draft
        .definitions()
        .unwrap()
        .spawn("chase", position.x, position.x, bounds)
        .unwrap()
}

#[test]
fn nearest_selection_uses_player_distance_and_ignores_bodyless_entities() {
    let entities = [
        enemy(Vec2::new(100.0, 400.0), true),
        enemy(Vec2::new(200.0, 400.0), false),
        enemy(Vec2::new(300.0, 400.0), true),
    ];
    let y = entities[0].position().y;
    assert_eq!(nearest_entity(&entities, Vec2::new(280.0, y)), Some(2));
    assert_eq!(nearest_entity(&entities, Vec2::new(120.0, y)), Some(0));
    assert_eq!(nearest_entity(&entities, Vec2::new(200.0, y)), Some(0));
    assert_eq!(nearest_entity(&entities, entities[2].position()), Some(2));
    assert_eq!(nearest_entity(&[], Vec2::ZERO), None);
    assert_eq!(nearest_entity(&entities[1..2], Vec2::ZERO), None);
    let vertical = [
        enemy(Vec2::new(100.0, 100.0), true),
        enemy(Vec2::new(110.0, 400.0), true),
    ];
    assert_eq!(nearest_entity(&vertical, Vec2::new(100.0, 390.0)), Some(1));
    let primitives = entities
        .iter()
        .enumerate()
        .flat_map(|(i, entity)| {
            entity
                .body()
                .map(|body| {
                    body_primitives(
                        body,
                        entity.position(),
                        entity.size(),
                        entity.facing_left(),
                        Some(i) == nearest_entity(&entities, Vec2::new(280.0, y)),
                    )
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        primitives
            .iter()
            .filter(|p| matches!(p, OverlayPrimitive::Label { .. }))
            .count(),
        2
    );
}

#[test]
fn pose_legend_uses_an_existing_hud_size_and_handles_no_body() {
    use hollow_steps::body_debug::{pose_legend, LEGEND_FONT_SIZE};
    assert_eq!(LEGEND_FONT_SIZE, 18);
    assert!(pose_legend(&[], Vec2::ZERO).ends_with("pose: none"));
}
