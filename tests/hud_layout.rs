use hollow_steps::hud_layout::*;
use hollow_steps::room::RoomId;

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
