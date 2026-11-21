use super::*;
use crate::enemy::EnemyDefinitions;
use crate::room::test_rooms;

fn draft() -> RoomDraft {
    let [room, _, _] = test_rooms();
    RoomDraft::new_room(&RoomLayout::from_room(&room))
}

fn no_definitions() -> EnemyDefinitions {
    EnemyDefinitions::from_definitions(Vec::new(), Default::default(), Default::default()).unwrap()
}

#[test]
fn add_puts_a_standard_hazard_on_the_floor_right_of_the_spawn_and_selects_it() {
    let mut draft = draft();
    let (spawn_x, _) = draft.layout.spawn;
    let selection = draft.add_hazard().unwrap();
    assert_eq!(selection, Selection::Hazard(0));
    let hazard = draft.hazard(0).unwrap();
    let (x, y, w, h) = hazard.rect;
    assert_eq!(x, spawn_x + SPAWN_CLEARANCE);
    assert_eq!(y + h, draft.layout.floor_y, "standing on the floor");
    assert_eq!(hazard, &HazardPlacement::standard(x, y));
    assert!(draft.layout.validate_geometry().is_ok());
    // A second one goes to the same spot and is the next index.
    assert_eq!(draft.add_hazard().unwrap(), Selection::Hazard(1));
    assert_eq!(draft.layout.hazards[1].rect, (x, y, w, h));
}

#[test]
fn add_uses_the_left_side_when_the_right_has_no_room() {
    let mut draft = draft();
    let right = draft.layout.rect().right();
    draft.layout.spawn.0 = right - 40.0;
    draft.add_hazard().unwrap();
    let (x, _, w, _) = draft.hazard(0).unwrap().rect;
    assert!(x + w <= draft.layout.spawn.0 - 16.0, "left of the player");
}

#[test]
fn add_is_refused_when_the_room_is_too_narrow_for_it() {
    let mut draft = draft();
    draft.layout.bounds.2 = 64.0;
    draft.layout.spawn.0 = 32.0;
    let before = draft.clone();
    let error = draft.add_hazard().unwrap_err();
    assert!(!error.is_empty());
    assert_eq!(draft, before, "a refused add changes nothing");
}

#[test]
fn move_sets_the_center_and_keeps_the_size() {
    let definitions = no_definitions();
    let mut draft = draft();
    draft.add_hazard().unwrap();
    let (_, _, w, h) = draft.hazard(0).unwrap().rect;
    draft
        .move_to(Selection::Hazard(0), Vec2::new(600.0, 300.0), &definitions)
        .unwrap();
    assert_eq!(
        draft.hazard(0).unwrap().rect,
        (600.0 - w / 2.0, 300.0 - h / 2.0, w, h)
    );
}

#[test]
fn move_is_kept_inside_the_room_and_refuses_bad_input() {
    let definitions = no_definitions();
    let mut draft = draft();
    draft.add_hazard().unwrap();
    let room = draft.layout.rect();
    let (_, _, w, h) = draft.hazard(0).unwrap().rect;
    draft
        .move_to(
            Selection::Hazard(0),
            Vec2::new(-500.0, -500.0),
            &definitions,
        )
        .unwrap();
    assert_eq!(draft.hazard(0).unwrap().rect, (room.x, room.y, w, h));
    draft
        .move_to(Selection::Hazard(0), Vec2::new(1e6, 1e6), &definitions)
        .unwrap();
    assert_eq!(
        draft.hazard(0).unwrap().rect,
        (room.right() - w, room.bottom() - h, w, h)
    );
    let before = draft.clone();
    assert!(draft
        .move_to(Selection::Hazard(0), Vec2::new(f32::NAN, 1.0), &definitions)
        .is_err());
    assert!(draft
        .move_to(Selection::Hazard(9), Vec2::new(1.0, 1.0), &definitions)
        .is_err());
    assert_eq!(draft, before);
}

#[test]
fn a_move_onto_the_spawn_is_refused_and_leaves_the_hazard_where_it_was() {
    let definitions = no_definitions();
    let mut draft = draft();
    draft.add_hazard().unwrap();
    let before = draft.clone();
    let (x, y) = draft.layout.spawn;
    let error = draft
        .move_to(Selection::Hazard(0), Vec2::new(x, y + 8.0), &definitions)
        .unwrap_err();
    assert!(
        error.contains("spawn must not be inside hazard 0"),
        "{error}"
    );
    assert_eq!(draft, before);
}

#[test]
fn replace_changes_the_numbers_and_the_size() {
    let mut draft = draft();
    draft.add_hazard().unwrap();
    draft.layout.hazards[0].rect.0 = 600.0; // clear of the spawn, which a wider one would reach
    let mut edited = draft.hazard(0).unwrap().resized(120.0, 32.0);
    edited.damage = 25;
    edited.knockback_speed = 400.0;
    edited.lift_speed = 100.0;
    edited.knockback_secs = 0.3;
    draft.replace_hazard(0, edited.clone()).unwrap();
    assert_eq!(draft.hazard(0), Some(&edited));
    assert_eq!(edited.rect.2, 120.0);
    assert_eq!(edited.rect.1 + edited.rect.3, draft.layout.floor_y);
}

#[test]
fn growing_a_hazard_into_the_spawn_is_refused() {
    let mut draft = draft();
    draft.add_hazard().unwrap(); // 32 px right of the spawn; growing it 28 px per side reaches it
    let before = draft.clone();
    let wider = draft.hazard(0).unwrap().resized(120.0, 16.0);
    let error = draft.replace_hazard(0, wider).unwrap_err();
    assert!(
        error.contains("spawn must not be inside hazard 0"),
        "{error}"
    );
    assert_eq!(draft, before);
}

#[test]
fn replace_refuses_an_invalid_value_and_changes_nothing() {
    let mut draft = draft();
    draft.add_hazard().unwrap();
    let before = draft.clone();
    let mut bad = draft.hazard(0).unwrap().clone();
    bad.damage = 0;
    assert!(draft.replace_hazard(0, bad).unwrap_err().contains("damage"));
    let too_wide = draft.hazard(0).unwrap().resized(5000.0, 16.0);
    assert!(draft
        .replace_hazard(0, too_wide)
        .unwrap_err()
        .contains("inside the room"));
    assert!(draft
        .replace_hazard(4, HazardPlacement::standard(0.0, 0.0))
        .is_err());
    assert_eq!(draft, before);
}

#[test]
fn delete_removes_only_the_selected_hazard() {
    let mut draft = draft();
    draft.add_hazard().unwrap();
    draft.add_hazard().unwrap();
    draft.layout.hazards[1].damage = 33;
    draft.delete(Selection::Hazard(0)).unwrap();
    assert_eq!(draft.layout.hazards.len(), 1);
    assert_eq!(draft.hazard(0).unwrap().damage, 33);
    assert!(draft.delete(Selection::Hazard(1)).is_err());
}

#[test]
fn a_new_room_starts_without_the_templates_hazards() {
    let mut template = draft().layout;
    template.hazards = vec![HazardPlacement::standard(400.0, 444.0)];
    assert!(RoomDraft::new_room(&template).layout.hazards.is_empty());
}
