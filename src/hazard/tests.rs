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

/// A body that has just walked into the strip from the left (one pixel in), and from the right.
fn from_the_left() -> Vec2 {
    body_at(400.0 - 16.0 + 1.0, 460.0)
}
fn from_the_right() -> Vec2 {
    body_at(464.0 + 16.0 - 1.0, 460.0)
}
/// A body that has just landed on the strip, feet `depth` pixels into it.
fn landed(depth: f32) -> Vec2 {
    body_at(432.0, 444.0 + depth)
}

#[test]
fn a_body_walking_in_takes_the_damage_and_a_lift() {
    let hit = hazard().hit(from_the_left(), BODY, Vec2::ZERO).unwrap();
    assert_eq!(hit.damage, i32::from(strip().damage));
    assert!(hit.knockback.y < 0.0, "up is negative y");
    assert_eq!(hit.knockback.y, -strip().lift_speed);
    assert_eq!(hit.knockback_secs, strip().knockback_secs);
}

#[test]
fn a_side_hit_points_away_from_the_hazards_center() {
    let hazard = hazard(); // center x = 432
    let left = hazard.hit(from_the_left(), BODY, Vec2::ZERO).unwrap();
    let right = hazard.hit(from_the_right(), BODY, Vec2::ZERO).unwrap();
    assert_eq!(left.knockback.x, -strip().knockback_speed);
    assert_eq!(right.knockback.x, strip().knockback_speed);
}

#[test]
fn a_body_exactly_in_line_with_the_center_of_a_narrow_tall_hazard_is_pushed_right() {
    // 8 wide and 100 tall: the 32 wide body is deeper in vertically than across, so it is a
    // side hit, and exactly in line with the center it goes right.
    let mut post = strip();
    post.rect = (428.0, 360.0, 8.0, 100.0);
    let hit = Hazard::from(&post)
        .hit(body_at(432.0, 460.0), BODY, Vec2::ZERO)
        .unwrap();
    assert_eq!(hit.knockback.x, post.knockback_speed);
}

#[test]
fn zero_lift_gives_no_upward_push_to_a_side_hit() {
    let mut flat = strip();
    flat.lift_speed = 0.0;
    let hit = Hazard::from(&flat)
        .hit(from_the_left(), BODY, Vec2::ZERO)
        .unwrap();
    assert_eq!(hit.knockback.y, 0.0);
    assert_eq!(hit.knockback.x, -flat.knockback_speed);
}

#[test]
fn landing_on_the_top_throws_the_body_straight_up() {
    let hazard = hazard();
    let hit = hazard.hit(landed(3.0), BODY, Vec2::ZERO).unwrap();
    assert_eq!(hit.knockback.x, 0.0, "no sideways push from above");
    assert_eq!(hit.knockback.y, -strip().knockback_speed);
    assert_eq!(hit.damage, i32::from(strip().damage));
}

#[test]
fn the_throw_sets_the_vertical_speed_so_a_fall_does_not_eat_it() {
    let hazard = hazard();
    // Falling at any speed, the body ends up moving up at the knockback speed.
    for fall in [0.0, 100.0, 480.0, 900.0] {
        let velocity = Vec2::new(0.0, fall);
        let hit = hazard.hit(landed(3.0), BODY, velocity).unwrap();
        assert_eq!(
            velocity.y + hit.knockback.y,
            -strip().knockback_speed,
            "{fall}"
        );
    }
    // A sideways hit sets it to the lift speed the same way.
    let velocity = Vec2::new(0.0, 500.0);
    let hit = hazard.hit(from_the_left(), BODY, velocity).unwrap();
    assert_eq!(velocity.y + hit.knockback.y, -strip().lift_speed);
}

#[test]
fn a_body_already_rising_faster_than_the_throw_is_not_slowed() {
    let hazard = hazard();
    let rising = Vec2::new(0.0, -1000.0);
    assert_eq!(
        hazard.hit(landed(3.0), BODY, rising).unwrap().knockback.y,
        0.0
    );
    assert_eq!(
        hazard
            .hit(from_the_left(), BODY, rising)
            .unwrap()
            .knockback
            .y,
        0.0
    );
}

#[test]
fn a_hazard_on_a_ceiling_throws_the_body_straight_down() {
    // The strip hangs at y 100..116; the body's head is three pixels into it, jumping up.
    let mut ceiling = strip();
    ceiling.rect = (400.0, 100.0, 64.0, 16.0);
    let hazard = Hazard::from(&ceiling);
    let head_in = Vec2::new(432.0, 116.0 - 3.0 + BODY.y / 2.0);
    let jumping = Vec2::new(0.0, -480.0);
    let hit = hazard.hit(head_in, BODY, jumping).unwrap();
    assert_eq!(hit.knockback.x, 0.0);
    assert_eq!(
        jumping.y + hit.knockback.y,
        ceiling.knockback_speed,
        "ends up moving down"
    );
    // Already falling faster than the throw: not slowed.
    let falling = Vec2::new(0.0, 900.0);
    assert_eq!(hazard.hit(head_in, BODY, falling).unwrap().knockback.y, 0.0);
}

#[test]
fn zero_knockback_speed_gives_no_throw_from_above() {
    let mut soft = strip();
    soft.knockback_speed = 0.0;
    let hit = Hazard::from(&soft)
        .hit(landed(3.0), BODY, Vec2::new(0.0, 600.0))
        .unwrap();
    assert_eq!((hit.knockback.x, hit.knockback.y), (0.0, 0.0));
    assert_eq!(hit.damage, i32::from(soft.damage), "still hurts");
}

#[test]
fn which_side_was_touched_is_the_side_the_body_is_least_far_into() {
    let hazard = hazard(); // 64 wide, 16 tall, on the floor
    let side = |center: Vec2| hazard.hit(center, BODY, Vec2::ZERO).unwrap().knockback.x != 0.0;
    assert!(
        side(from_the_left()) && side(from_the_right()),
        "walking in"
    );
    assert!(!side(landed(3.0)), "landing");
    // Standing in the middle of the strip, 16 deep and 32 across: the top.
    assert!(!side(body_at(432.0, 460.0)));
    // Near the far end of a long strip, a landing is still from above: it is 7 px deep and
    // about 26 across, even though the body is far from the strip's center.
    let mut long = strip();
    long.rect = (200.0, 444.0, 400.0, 16.0);
    let near_the_end = Vec2::new(590.0 - 4.0, 444.0 + 7.0 - BODY.y / 2.0);
    let hit = Hazard::from(&long)
        .hit(near_the_end, BODY, Vec2::new(0.0, 600.0))
        .unwrap();
    assert_eq!(hit.knockback.x, 0.0);
    assert!(hit.knockback.y < 0.0);
}

#[test]
fn the_thinnest_hazard_is_still_landed_on_from_above_at_full_fall_speed() {
    // 8 wide; the fastest fall (900 px/s) is 7.5 px per tick, so the first tick in is 7.5 deep.
    let mut thin = strip();
    thin.rect = (428.0, 444.0, 8.0, 16.0);
    let hit = Hazard::from(&thin)
        .hit(body_at(432.0, 444.0 + 7.5), BODY, Vec2::new(0.0, 900.0))
        .unwrap();
    assert_eq!(hit.knockback.x, 0.0);
    assert!(hit.knockback.y < 0.0);
}

#[test]
fn a_tall_hazard_against_a_wall_pushes_sideways_from_the_side_and_up_from_its_top() {
    // 16 wide and 400 tall, against the wall at x 800 (the room's right edge is further on).
    let mut post = strip();
    post.rect = (784.0, 60.0, 16.0, 400.0);
    let hazard = Hazard::from(&post);
    let walking_in = hazard
        .hit(Vec2::new(784.0 - 16.0 + 1.0, 300.0), BODY, Vec2::ZERO)
        .unwrap();
    assert_eq!(walking_in.knockback.x, -post.knockback_speed);
    let landing = hazard
        .hit(
            Vec2::new(792.0, 60.0 + 2.0 - BODY.y / 2.0),
            BODY,
            Vec2::new(0.0, 500.0),
        )
        .unwrap();
    assert_eq!(landing.knockback.x, 0.0);
    assert!(landing.knockback.y < 0.0);
}

#[test]
fn touching_an_edge_or_standing_on_top_is_safe() {
    let hazard = hazard();
    // Feet exactly on the top edge (a platform's top that is the hazard's top).
    assert!(hazard
        .hit(body_at(432.0, 444.0), BODY, Vec2::ZERO)
        .is_none());
    // Body's right side exactly at the hazard's left side.
    assert!(hazard
        .hit(body_at(400.0 - 16.0, 460.0), BODY, Vec2::ZERO)
        .is_none());
    // Body's left side exactly at the hazard's right side.
    assert!(hazard
        .hit(body_at(464.0 + 16.0, 460.0), BODY, Vec2::ZERO)
        .is_none());
    // Head exactly at the hazard's bottom (a ceiling spike seen from below).
    assert!(hazard
        .hit(Vec2::new(432.0, 460.0 + BODY.y / 2.0), BODY, Vec2::ZERO)
        .is_none());
}

#[test]
fn a_body_far_away_or_not_finite_is_never_hit() {
    let hazard = hazard();
    assert!(hazard
        .hit(body_at(700.0, 460.0), BODY, Vec2::ZERO)
        .is_none());
    assert!(hazard
        .hit(body_at(432.0, 300.0), BODY, Vec2::ZERO)
        .is_none());
    assert!(hazard
        .hit(Vec2::new(f32::NAN, 436.0), BODY, Vec2::ZERO)
        .is_none());
    assert!(hazard
        .hit(Vec2::new(432.0, f32::INFINITY), BODY, Vec2::ZERO)
        .is_none());
}

#[test]
fn a_one_pixel_overlap_is_a_hit() {
    // Feet one pixel below the hazard's top.
    assert!(hazard()
        .hit(body_at(432.0, 445.0), BODY, Vec2::ZERO)
        .is_some());
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
fn a_hazard_draws_in_its_own_style_facing_and_color_or_the_styles_color() {
    let mut lava = strip();
    lava.style = HazardStyle::Lava;
    lava.facing = HazardFacing::Left;
    let plain = Hazard::from(&lava);
    assert_eq!(plain.draw_color(), HazardStyle::Lava.default_color());
    assert_eq!(
        plain.look(3.0),
        hazard_look::shapes(
            plain.rect,
            HazardStyle::Lava,
            HazardFacing::Left,
            HazardStyle::Lava.default_color(),
            3.0
        )
    );
    lava.color = Some((0.1, 0.2, 0.3));
    let tinted = Hazard::from(&lava);
    assert_eq!(tinted.draw_color(), (0.1, 0.2, 0.3));
    assert_ne!(tinted.look(3.0), plain.look(3.0));
}

#[test]
fn a_new_hazard_is_spikes_facing_up_in_the_default_color() {
    let new = strip();
    assert_eq!(
        (new.style, new.facing, new.color),
        (HazardStyle::Spikes, HazardFacing::Up, None)
    );
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
    fields.style = HazardStyle::Thorns;
    fields.facing = HazardFacing::Right;
    fields.color = Some((0.25, 0.5, 0.75));
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
    edge.color = Some((0.0, 1.0, 0.5));
    assert!(edge.check(0, room).is_ok(), "0 and 1 are inside the range");
}

#[test]
fn every_out_of_range_value_is_refused_and_named() {
    let room = layout().rect();
    let right = room.right();
    let bad: [(&str, Break); 15] = [
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
        ("color", Box::new(|h| h.color = Some((1.5, 0.0, 0.0)))),
        ("color", Box::new(|h| h.color = Some((0.0, -0.1, 0.0)))),
        ("color", Box::new(|h| h.color = Some((0.0, 0.0, f32::NAN)))),
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
    let mut lava = HazardPlacement::standard(300.0, 300.0);
    lava.style = HazardStyle::Lava;
    lava.facing = HazardFacing::Left;
    lava.color = Some((0.5, 0.25, 0.125));
    with.hazards = vec![strip(), HazardPlacement::standard(600.5, 300.25), lava];
    let text = ron::ser::to_string_pretty(&with, Default::default()).unwrap();
    assert!(text.contains("hazards"), "{text}");
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, with);
}

#[test]
fn a_hazard_file_from_before_styles_loads_as_spikes_facing_up_in_the_default_color() {
    let old = "(rect: (0.0, 0.0, 8.0, 8.0), damage: 1, knockback_speed: 0.0, \
               lift_speed: 0.0, knockback_secs: 0.0)";
    let loaded: HazardPlacement = ron::from_str(old).unwrap();
    assert_eq!(
        (loaded.style, loaded.facing, loaded.color),
        (HazardStyle::Spikes, HazardFacing::Up, None)
    );
}

#[test]
fn a_default_colored_hazard_writes_its_style_and_facing_but_no_color() {
    let text = ron::ser::to_string(&strip()).unwrap();
    assert!(
        text.contains("style:Spikes") && text.contains("facing:Up"),
        "{text}"
    );
    assert!(!text.contains("color"), "{text}");
    let mut tinted = strip();
    tinted.color = Some((1.0, 0.0, 0.0));
    let text = ron::ser::to_string(&tinted).unwrap();
    assert!(text.contains("color"), "{text}");
    assert_eq!(ron::from_str::<HazardPlacement>(&text).unwrap(), tinted);
}

#[test]
fn a_hazard_file_with_an_unknown_missing_or_misspelled_field_is_refused() {
    let unknown = "(rect: (0.0, 0.0, 8.0, 8.0), damage: 1, knockback_speed: 0.0, \
                   lift_speed: 0.0, knockback_secs: 0.0, glow: 3)";
    assert!(ron::from_str::<HazardPlacement>(unknown).is_err());
    let bad_style = "(rect: (0.0, 0.0, 8.0, 8.0), damage: 1, knockback_speed: 0.0, \
                     lift_speed: 0.0, knockback_secs: 0.0, style: Slime)";
    assert!(ron::from_str::<HazardPlacement>(bad_style).is_err());
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
