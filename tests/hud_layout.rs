use hollow_steps::hud_layout::*;
use hollow_steps::room::RoomId;
use macroquad::math::vec2;

#[test]
fn pips_fill_from_the_left() {
    assert_eq!(pip_states(3, 5), [true, true, true, false, false]);
    assert_eq!(pip_states(5, 5), [true; 5]);
}

#[test]
fn pips_at_the_edges() {
    assert_eq!(pip_states(0, 5), [false; 5]);
    assert_eq!(pip_states(9, 3), [true; 3]);
    assert!(pip_states(2, 0).is_empty());
}

#[test]
fn pip_centers_step_evenly_and_fit_the_row() {
    let origin = vec2(10.0, 20.0);
    let first = pip_center(origin, 0);
    let second = pip_center(origin, 1);
    assert_eq!(first, vec2(10.0 + PIP_RADIUS, 20.0 + PIP_RADIUS));
    assert_eq!(second.x - first.x, PIP_RADIUS * 2.0 + PIP_GAP);
    let row = pip_row_size(5);
    assert_eq!(pip_center(origin, 4).x + PIP_RADIUS, origin.x + row.x);
    assert_eq!(pip_row_size(0), vec2(0.0, 0.0));
}

#[test]
fn boss_fill_is_clamped_and_safe() {
    assert_eq!(boss_fill(50, 100), 0.5);
    assert_eq!(boss_fill(0, 100), 0.0);
    assert_eq!(boss_fill(100, 100), 1.0);
    assert_eq!(boss_fill(150, 100), 1.0);
    assert_eq!(boss_fill(5, 0), 0.0);
}

#[test]
fn boss_caption_uses_the_given_name() {
    assert_eq!(boss_caption("Cinder Queen", 7, 10), "Cinder Queen: 7/10");
    assert_eq!(boss_caption("Ash Warden", 20, 10), "Ash Warden: 10/10");
}

#[test]
fn room_labels_are_readable() {
    assert_eq!(room_label(RoomId::First), "First room");
    assert_eq!(room_label(RoomId::Boss), "Boss room");
    assert_eq!(room_label(RoomId::Custom(0)), "Custom room 1");
    assert_ne!(room_label(RoomId::Custom(0)), room_label(RoomId::Custom(1)));
}
