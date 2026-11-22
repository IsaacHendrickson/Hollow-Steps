//! The air jump of `Ability::DoubleJump`: one extra jump in the air, refilled on landing.
//!
//! It is taken on a fresh jump press (`PlayerInput::jump_pressed`, or the press the jump buffer
//! still holds), never for a held key, and only once the ground jump and the coyote window have
//! had their turn: a press that can still be a normal jump is a normal jump. It sets the upward
//! speed to `PlayerConfig::double_jump_speed`, whatever the speed was before, so a ceiling stops
//! it like any rise (`displace`). A dash ignores jump presses while it lasts, so an air jump waits
//! for the dash to end. The refill is in `resolve_floor` (any floor, the room's or a solid's) and
//! `place_at`; pogo bounces do not refill it.
use super::Player;
use crate::ability::Ability;
use crate::events::GameEvent;

impl Player {
    /// Takes the air jump when a jump press is waiting and nothing else can use it. Returns
    /// whether it did. The caller has already offered the press to the ground jump, which
    /// clears the buffer when it takes it, so one press never makes both jumps.
    pub(super) fn try_air_jump(&mut self) -> bool {
        let available = self.abilities.has(Ability::DoubleJump)
            && !self.air_jump_used
            && !self.grounded
            && self.coyote_remaining == 0.0
            && self.jump_buffer_remaining > 0.0;
        if available {
            self.velocity.y = -self.config.double_jump_speed;
            self.air_jump_used = true;
            self.jump_buffer_remaining = 0.0;
            self.events.push(GameEvent::PlayerJumped);
        }
        available
    }
}
