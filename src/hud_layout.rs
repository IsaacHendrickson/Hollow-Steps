//! Window-free HUD numbers and text: boss bar fill and caption, room labels.
//! `hud.rs` draws them; keeping the rules here lets them be tested without a window.
//! The health and mana bars are laid out by `hud_bars`.

use crate::room::RoomId;

/// How much of the boss bar is filled, 0 to 1. An empty or zero-max bar is empty.
pub fn boss_fill(health: u32, max_health: u32) -> f32 {
    if max_health == 0 {
        return 0.0;
    }
    (health.min(max_health) as f32 / max_health as f32).clamp(0.0, 1.0)
}

/// The boss bar caption: the boss definition's own name and its health.
pub fn boss_caption(name: &str, health: u32, max_health: u32) -> String {
    format!("{name}: {}/{max_health}", health.min(max_health))
}

/// A readable label for a room id alone. The game shows `room_name::room_title`, which
/// prefers the room's own name.
pub fn room_label(id: RoomId) -> String {
    match id {
        RoomId::First => "First room".to_owned(),
        RoomId::Second => "Second room".to_owned(),
        RoomId::Boss => "Boss room".to_owned(),
        RoomId::Custom(index) => format!("Custom room {}", index + 1),
    }
}
