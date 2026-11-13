//! Window-free HUD numbers and text: health pips, boss bar fill and labels, room names.
//! `hud.rs` draws them; keeping the rules here lets them be tested without a window.

use macroquad::math::{vec2, Vec2};

use crate::health::HIT_DAMAGE;
use crate::room::RoomId;

/// Radius of one health mask and the space between neighbours, in virtual-resolution pixels.
pub const PIP_RADIUS: f32 = 13.0;
pub const PIP_GAP: f32 = 8.0;

/// Which of `max` health pips are full for `current` health. `current` above `max` fills all.
pub fn pip_states(current: u16, max: u16) -> Vec<bool> {
    (0..max).map(|index| index < current).collect()
}

/// How many health masks show `health`: one per standard hit, a part-used hit still counts.
/// The HUD now draws health bars (`hud_bars`); this and the pip helpers are no longer used by it.
pub fn masks_for(health: u16) -> u16 {
    health.div_ceil(HIT_DAMAGE)
}

/// Center of pip `index` for a row whose first pip's top-left corner is `origin`.
pub fn pip_center(origin: Vec2, index: usize) -> Vec2 {
    origin
        + vec2(
            PIP_RADIUS + index as f32 * (PIP_RADIUS * 2.0 + PIP_GAP),
            PIP_RADIUS,
        )
}

/// Total size of a row of `count` pips (zero for none).
pub fn pip_row_size(count: usize) -> Vec2 {
    if count == 0 {
        return Vec2::ZERO;
    }
    vec2(
        count as f32 * PIP_RADIUS * 2.0 + (count - 1) as f32 * PIP_GAP,
        PIP_RADIUS * 2.0,
    )
}

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
