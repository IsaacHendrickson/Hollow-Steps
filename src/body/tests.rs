use super::*;

fn template() -> BodyTemplate {
    ron::from_str(include_str!("../../tests/fixtures/bodies/small.ron")).unwrap()
}

fn near(actual: Vec2, expected: Vec2) {
    assert!(
        actual.distance(expected) < 0.0001,
        "{actual:?} != {expected:?}"
    );
}

#[test]
fn resolves_rest_and_current_points_with_box_scale_and_mirroring() {
    let template = template();
    template.validate().unwrap();
    let mut points = template.rest_positions();
    let center = Vec2::new(100.0, 200.0);
    let size = Vec2::new(80.0, 40.0);
    near(
        resolve_point("root", &points, center, size, 2.0, false).unwrap(),
        Vec2::new(116.0, 184.0),
    );
    near(
        resolve_point("root", &points, center, size, 2.0, true).unwrap(),
        Vec2::new(84.0, 184.0),
    );
    points.insert("root".into(), Vec2::new(-0.2, 0.5));
    near(
        resolve_point("root", &points, center, size, 0.5, false).unwrap(),
        Vec2::new(92.0, 210.0),
    );
    assert_eq!(template.points["root"].rest, (0.1, -0.2));
    assert!(resolve_point("missing", &points, center, size, 1.0, false)
        .unwrap_err()
        .contains("unknown point \"missing\""));
}

#[test]
fn transforms_anchor_offset_size_rotation_and_every_vertex_under_reflection() {
    let template = template();
    let points = template.rest_positions();
    let center = Vec2::new(100.0, 200.0);
    for part in &template.parts {
        let right = resolve_part(part, &points, center, Vec2::new(80.0, 40.0), 2.0, false).unwrap();
        let left = resolve_part(part, &points, center, Vec2::new(80.0, 40.0), 2.0, true).unwrap();
        near(
            left.center,
            Vec2::new(200.0 - right.center.x, right.center.y),
        );
        near(left.size, right.size);
        assert_eq!(left.rotation, -right.rotation);
        for vertex in [
            Vec2::new(-0.5, -0.5),
            Vec2::new(0.5, 0.0),
            Vec2::new(-0.5, 0.5),
        ] {
            let a = right.vertex(vertex);
            near(left.vertex(vertex), Vec2::new(200.0 - a.x, a.y));
        }
    }
    let rect = resolve_part(
        &template.parts[1],
        &points,
        center,
        Vec2::new(80.0, 40.0),
        2.0,
        false,
    )
    .unwrap();
    near(rect.center, Vec2::new(124.0, 192.0));
    near(rect.size, Vec2::new(32.0, 32.0));
    near(rect.vertex(Vec2::new(0.5, 0.0)), Vec2::new(124.0, 208.0));
}

#[test]
fn line_connects_current_points_and_handles_coincident_endpoints_and_rotation() {
    let template = template();
    let mut points = template.rest_positions();
    let mut line = template.parts[0].clone();
    let a = resolve_part(
        &line,
        &points,
        Vec2::ZERO,
        Vec2::new(100.0, 100.0),
        1.0,
        false,
    )
    .unwrap();
    let (from, to) = a.endpoints.unwrap();
    near(from, Vec2::new(10.0, -20.0));
    near(to, Vec2::new(40.0, 30.0));
    line.rotation = 90.0;
    let right = resolve_part(&line, &points, Vec2::ZERO, Vec2::splat(100.0), 1.0, false).unwrap();
    let left = resolve_part(&line, &points, Vec2::ZERO, Vec2::splat(100.0), 1.0, true).unwrap();
    let (r1, r2) = right.endpoints.unwrap();
    let (l1, l2) = left.endpoints.unwrap();
    near(l1, Vec2::new(-r1.x, r1.y));
    near(l2, Vec2::new(-r2.x, r2.y));
    points.insert("tip".into(), points["root"]);
    let same = resolve_part(&line, &points, Vec2::ZERO, Vec2::splat(100.0), 1.0, false).unwrap();
    let (from, to) = same.endpoints.unwrap();
    near(from, to);
}

#[test]
fn rejects_unknown_anchor_and_both_line_references_and_unknown_color_slot() {
    for (index, field) in [(1, "anchor"), (0, "from"), (0, "to")] {
        let mut template = template();
        match field {
            "anchor" => template.parts[index].anchor = Some("absent".into()),
            "from" => template.parts[index].from = Some("absent".into()),
            _ => template.parts[index].to = Some("absent".into()),
        }
        let error = template.validate().unwrap_err();
        assert!(
            error.contains(&template.parts[index].name)
                && error.contains("unknown point \"absent\""),
            "{error}"
        );
    }
    let source = include_str!("../../tests/fixtures/bodies/small.ron")
        .replace("color_slot: eye", "color_slot: unknown");
    let error = ron::from_str::<BodyTemplate>(&source)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("unknown") && error.contains("primary") && error.contains("eye"),
        "{error}"
    );
}

#[test]
fn rejects_nonfinite_geometry_invalid_sizes_and_mixed_line_fields() {
    let mut bad = template();
    bad.parts[0].anchor = Some("root".into());
    assert!(bad
        .validate()
        .unwrap_err()
        .contains("instead of anchor/offset"));
    for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut bad = template();
        bad.parts[1].size.0 = value;
        assert!(bad.validate().unwrap_err().contains("size"));
    }
    let mut bad = template();
    bad.parts[1].rotation = f32::NAN;
    assert!(bad.validate().unwrap_err().contains("rotation"));
    let mut bad = template();
    bad.parts[1].offset = Some((f32::INFINITY, 0.0));
    assert!(bad.validate().unwrap_err().contains("offset"));
    let mut bad = template();
    bad.points.get_mut("root").unwrap().rest.0 = f32::NAN;
    assert!(bad.validate().unwrap_err().contains("root"));
}
