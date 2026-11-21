use super::*;
use crate::room::test_rooms;

const BODY: Vec2 = Vec2::new(32.0, 48.0);

/// One way to break a valid hazard.
type Break = Box<dyn Fn(&mut HazardPlacement)>;

/// The first sample room's layout (960 wide, floor at 460, nothing solid, no hazards).
fn layout() -> RoomLayout {
    let [room, _, _] = test_rooms();
    RoomLayout::from_room(&room)
}

/// A spike strip standing on the floor at x 400..464, y 444..460.
fn strip() -> HazardPlacement {
    HazardPlacement::standard(400.0, 444.0)
}

fn hazard() -> Hazard {
    Hazard::from(&strip())
}

/// A body whose bottom rests at `bottom` with its center at `x`.
fn body_at(x: f32, bottom: f32) -> Vec2 {
    Vec2::new(x, bottom - BODY.y / 2.0)
}

#[test]
fn a_body_overlapping_the_hazard_takes_its_damage_and_a_lift() {
    let hit = hazard().hit(body_at(432.0, 460.0), BODY).unwrap();
    assert_eq!(hit.damage, i32::from(strip().damage));
    assert!(hit.knockback.y < 0.0, "up is negative y");
    assert_eq!(hit.knockback.y, -strip().lift_speed);
    assert_eq!(hit.knockback_secs, strip().knockback_secs);
}

#[test]
fn the_knockback_points_away_from_the_hazards_center() {
    let hazard = hazard(); // center x = 432
    let right = hazard.hit(body_at(440.0, 460.0), BODY).unwrap();
    let left = hazard.hit(body_at(424.0, 460.0), BODY).unwrap();
    assert_eq!(right.knockback.x, strip().knockback_speed);
    assert_eq!(left.knockback.x, -strip().knockback_speed);
}

#[test]
fn a_body_exactly_in_line_with_the_center_is_pushed_right() {
    let hit = hazard().hit(body_at(432.0, 460.0), BODY).unwrap();
    assert_eq!(hit.knockback.x, strip().knockback_speed);
}

#[test]
fn zero_lift_gives_no_upward_push() {
    let mut flat = strip();
    flat.lift_speed = 0.0;
    let hit = Hazard::from(&flat)
        .hit(body_at(432.0, 460.0), BODY)
        .unwrap();
    assert_eq!(hit.knockback.y, 0.0);
}

#[test]
fn touching_an_edge_or_standing_on_top_is_safe() {
    let hazard = hazard();
    // Feet exactly on the top edge (a platform's top that is the hazard's top).
    assert!(hazard.hit(body_at(432.0, 444.0), BODY).is_none());
    // Body's right side exactly at the hazard's left side.
    assert!(hazard.hit(body_at(400.0 - 16.0, 460.0), BODY).is_none());
    // Body's left side exactly at the hazard's right side.
    assert!(hazard.hit(body_at(464.0 + 16.0, 460.0), BODY).is_none());
    // Head exactly at the hazard's bottom (a ceiling spike seen from below).
    assert!(hazard
        .hit(Vec2::new(432.0, 460.0 + BODY.y / 2.0), BODY)
        .is_none());
}

#[test]
fn a_body_far_away_or_not_finite_is_never_hit() {
    let hazard = hazard();
    assert!(hazard.hit(body_at(700.0, 460.0), BODY).is_none());
    assert!(hazard.hit(body_at(432.0, 300.0), BODY).is_none());
    assert!(hazard.hit(Vec2::new(f32::NAN, 436.0), BODY).is_none());
    assert!(hazard.hit(Vec2::new(432.0, f32::INFINITY), BODY).is_none());
}

#[test]
fn a_one_pixel_overlap_is_a_hit() {
    // Feet one pixel below the hazard's top.
    assert!(hazard().hit(body_at(432.0, 445.0), BODY).is_some());
}

#[test]
fn resizing_keeps_the_center_line_and_the_bottom_edge() {
    let grown = strip().resized(100.0, 40.0);
    assert_eq!(grown.rect, (382.0, 420.0, 100.0, 40.0));
    assert_eq!(grown.damage, strip().damage);
    let back = grown.resized(64.0, 16.0);
    assert_eq!(back.rect, strip().rect);
}

#[test]
fn the_spikes_fill_the_rectangle_in_whole_triangles_pointing_up() {
    let spikes = hazard().spike_triangles(); // 64 px wide: four 16 px spikes
    assert_eq!(spikes.len(), 4);
    let rect = hazard().rect;
    for [left, tip, right] in &spikes {
        assert_eq!((left.y, right.y), (rect.bottom(), rect.bottom()));
        assert_eq!(tip.y, rect.y);
        assert_eq!(tip.x, (left.x + right.x) / 2.0);
    }
    assert_eq!(spikes[0][0].x, rect.x);
    assert_eq!(spikes[3][2].x, rect.right());
    // Each spike starts where the one before ended.
    assert_eq!(spikes[1][0].x, spikes[0][2].x);
}

#[test]
fn a_narrow_hazard_still_gets_one_spike_and_an_empty_one_none() {
    let mut narrow = hazard();
    narrow.rect.w = 8.0;
    assert_eq!(narrow.spike_triangles().len(), 1);
    narrow.rect.w = 0.0;
    assert!(narrow.spike_triangles().is_empty());
    narrow.rect.w = f32::NAN;
    assert!(narrow.spike_triangles().is_empty());
}

#[test]
fn fields_read_and_write_every_number_and_a_size_change_keeps_the_bottom() {
    let strip = strip();
    let mut fields = strip.fields();
    assert_eq!((fields.width, fields.height), (64.0, 16.0));
    assert_eq!(
        strip.with_fields(fields),
        strip,
        "unchanged fields change nothing"
    );
    fields.damage = 30;
    fields.knockback_speed = 111.0;
    fields.lift_speed = 222.0;
    fields.knockback_secs = 0.5;
    fields.height = 40.0;
    let edited = strip.with_fields(fields);
    assert_eq!(edited.fields(), fields);
    assert_eq!(edited.rect.1 + edited.rect.3, strip.rect.1 + strip.rect.3);
    assert_eq!(edited.rect.0, strip.rect.0, "width did not change");
}

#[test]
fn a_valid_hazard_passes_and_the_limits_are_inclusive() {
    let room = layout().rect();
    assert!(strip().check(0, room).is_ok());
    let mut edge = strip();
    edge.rect = (
        room.x,
        room.bottom() - MIN_HAZARD_SIZE,
        MIN_HAZARD_SIZE,
        MIN_HAZARD_SIZE,
    );
    edge.damage = 1;
    edge.knockback_speed = 0.0;
    edge.lift_speed = 0.0;
    edge.knockback_secs = 0.0;
    assert!(edge.check(0, room).is_ok());
    edge.damage = MAX_HAZARD_DAMAGE;
    edge.knockback_speed = MAX_HAZARD_SPEED;
    edge.lift_speed = MAX_HAZARD_SPEED;
    edge.knockback_secs = MAX_HAZARD_KNOCKBACK_SECS;
    assert!(edge.check(0, room).is_ok());
}

#[test]
fn every_out_of_range_value_is_refused_and_named() {
    let room = layout().rect();
    let right = room.right();
    let bad: [(&str, Break); 12] = [
        ("finite", Box::new(|h| h.rect.0 = f32::NAN)),
        ("finite", Box::new(|h| h.rect.2 = f32::INFINITY)),
        ("at least", Box::new(|h| h.rect.2 = MIN_HAZARD_SIZE - 1.0)),
        ("at least", Box::new(|h| h.rect.3 = 0.0)),
        ("inside the room", Box::new(|h| h.rect.0 = -1.0)),
        (
            "inside the room",
            Box::new(move |h| h.rect.0 = right - 10.0),
        ),
        ("damage", Box::new(|h| h.damage = 0)),
        ("damage", Box::new(|h| h.damage = MAX_HAZARD_DAMAGE + 1)),
        ("knockback speed", Box::new(|h| h.knockback_speed = -1.0)),
        ("lift speed", Box::new(|h| h.lift_speed = f32::NAN)),
        (
            "lift speed",
            Box::new(|h| h.lift_speed = MAX_HAZARD_SPEED + 1.0),
        ),
        ("knockback time", Box::new(|h| h.knockback_secs = -0.1)),
    ];
    for (word, edit) in bad {
        let mut hazard = strip();
        edit(&mut hazard);
        let error = hazard.check(3, room).unwrap_err();
        assert!(
            error.contains("hazard 3") && error.contains(word),
            "{error}"
        );
    }
}

#[test]
fn a_layout_without_hazards_loads_with_none_and_writes_none() {
    let layout = layout();
    assert!(layout.hazards.is_empty());
    let text = ron::ser::to_string_pretty(&layout, Default::default()).unwrap();
    assert!(!text.contains("hazards"), "{text}");
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, layout);
}

#[test]
fn hazards_round_trip_through_ron() {
    let mut with = layout();
    with.hazards = vec![strip(), HazardPlacement::standard(600.5, 300.25)];
    let text = ron::ser::to_string_pretty(&with, Default::default()).unwrap();
    assert!(text.contains("hazards"), "{text}");
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, with);
}

#[test]
fn a_hazard_file_with_an_unknown_or_missing_field_is_refused() {
    let unknown = "(rect: (0.0, 0.0, 8.0, 8.0), damage: 1, knockback_speed: 0.0, \
                   lift_speed: 0.0, knockback_secs: 0.0, color: 3)";
    assert!(ron::from_str::<HazardPlacement>(unknown).is_err());
    let missing = "(rect: (0.0, 0.0, 8.0, 8.0), damage: 1)";
    assert!(ron::from_str::<HazardPlacement>(missing).is_err());
}

#[test]
fn the_layout_refuses_a_hazard_outside_the_room_or_over_the_spawn() {
    let mut outside = layout();
    outside.hazards = vec![HazardPlacement::standard(-5.0, 444.0)];
    assert!(outside
        .validate_geometry()
        .unwrap_err()
        .contains("inside the room"));

    let mut over_spawn = layout();
    let (x, y) = over_spawn.spawn;
    over_spawn.hazards = vec![HazardPlacement::standard(x - 32.0, y + 8.0)];
    assert!(over_spawn
        .validate_geometry()
        .unwrap_err()
        .contains("spawn must not be inside hazard 0"));

    // The same hazard just to the side of the player's body is fine.
    let mut beside = layout();
    beside.hazards = vec![HazardPlacement::standard(x + 16.0, y + 8.0)];
    assert!(beside.validate_geometry().is_ok());
}

#[test]
fn applying_a_layout_builds_the_hazards_and_from_room_gives_them_back() {
    let [mut room, _, _] = test_rooms();
    let mut with = RoomLayout::from_room(&room);
    with.hazards = vec![strip()];
    with.apply(&mut room).unwrap();
    assert_eq!(room.hazards, vec![hazard()]);
    assert_eq!(RoomLayout::from_room(&room), with);
    // Applying a layout with none clears them again.
    layout().apply(&mut room).unwrap();
    assert!(room.hazards.is_empty());
}
