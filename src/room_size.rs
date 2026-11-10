//! Editing a room's bounds and floor height. A resize is only accepted if everything
//! already placed in the room (spawn, items, exits, terrain, enemies, boss) still fits.
use crate::boss_definition::BossDefinitions;
use crate::editor::{EditorSession, RoomDraft, RoomLayout};
use crate::enemy::EnemyDefinitions;

/// A room's position, size and floor line, as edited in the panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RoomSize {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub floor_y: f32,
}

impl RoomSize {
    pub fn of(layout: &RoomLayout) -> Self {
        let (x, y, width, height) = layout.bounds;
        Self {
            x,
            y,
            width,
            height,
            floor_y: layout.floor_y,
        }
    }

    pub fn apply_to(self, layout: &mut RoomLayout) {
        layout.bounds = (self.x, self.y, self.width, self.height);
        layout.floor_y = self.floor_y;
    }
}

/// Returns a copy of `draft` with the new size, or the reason it is refused. `draft` is
/// never modified. `bosses` is only needed (and only checked) when the draft has a boss.
pub fn resized(
    draft: &RoomDraft,
    size: RoomSize,
    definitions: &EnemyDefinitions,
    bosses: Option<&BossDefinitions>,
) -> Result<RoomDraft, String> {
    let mut next = draft.clone();
    size.apply_to(&mut next.layout);
    next.validate_local(definitions)?;
    if next.boss.is_some() {
        next.validate_boss(bosses.ok_or("boss definitions are unavailable")?)?;
    }
    Ok(next)
}

/// Applies a resize to the open editor session, marking it unsaved. On refusal the draft
/// is untouched and the reason is returned (the caller shows it).
pub fn resize_session(session: &mut EditorSession, size: RoomSize) -> Result<(), String> {
    if size == RoomSize::of(&session.draft.layout) {
        return Ok(());
    }
    let bosses = if session.draft.boss.is_some() {
        Some(session.content.boss_definitions()?)
    } else {
        None
    };
    session.draft = resized(&session.draft, size, &session.definitions, bosses.as_ref())?;
    session.dirty = true;
    Ok(())
}
