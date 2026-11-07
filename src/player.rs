use macroquad::math::Vec2;

/// A single frame of player intent.
///
/// `jump_pressed` is an edge-triggered input: it should be true only on the
/// frame the jump key was pressed, not while the key is held down.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerInput {
    pub move_axis: f32,
    pub jump_pressed: bool,
}

/// Tunable values that define how the player feels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerConfig {
    pub move_speed: f32,
    pub jump_speed: f32,
    pub gravity: f32,
    pub max_fall_speed: f32,
    pub size: Vec2,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            move_speed: 220.0,
            jump_speed: 480.0,
            gravity: 1_400.0,
            max_fall_speed: 900.0,
            size: Vec2::new(32.0, 48.0),
        }
    }
}
/// The playable rectangular area for the first prototype.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldBounds {
    pub left: f32,
    pub right: f32,
    /// The y coordinate of the top of the floor.
    pub floor_y: f32,
}

impl WorldBounds {
    pub const fn new(left: f32, right: f32, floor_y: f32) -> Self {
        Self {
            left,
            right,
            floor_y,
        }
    }
}

/// Player gameplay state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Player {
    position: Vec2,
    velocity: Vec2,
    config: PlayerConfig,
    grounded: bool,
}

impl Player {
    /// Creates a player whose position is the center of its body.
    pub fn new(position: Vec2, config: PlayerConfig) -> Self {
        Self {
            position,
            velocity: Vec2::ZERO,
            config,
            grounded: false,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn velocity(&self) -> Vec2 {
        self.velocity
    }

    pub fn size(&self) -> Vec2 {
        self.config.size
    }

    pub fn is_grounded(&self) -> bool {
        self.grounded
    }

    /// Places the player on the floor and clears movement left over from a
    /// previous life or room transition.
    pub fn place_on_floor(&mut self, x: f32, bounds: WorldBounds) {
        self.position = Vec2::new(
            x.clamp(
                bounds.left + self.config.size.x / 2.0,
                bounds.right - self.config.size.x / 2.0,
            ),
            bounds.floor_y - self.config.size.y / 2.0,
        );
        self.velocity = Vec2::ZERO;
        self.grounded = true;
    }

    /// Advances the player by one simulation step.
    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32, bounds: WorldBounds) {
        let delta_seconds = delta_seconds.max(0.0);
        let move_axis = input.move_axis.clamp(-1.0, 1.0);

        self.velocity.x = move_axis * self.config.move_speed;

        if input.jump_pressed && self.grounded {
            self.velocity.y = -self.config.jump_speed;
            self.grounded = false;
        }

        self.velocity.y =
            (self.velocity.y + self.config.gravity * delta_seconds).min(self.config.max_fall_speed);
        self.position += self.velocity * delta_seconds;

        self.resolve_horizontal_bounds(bounds);
        self.resolve_floor(bounds);
    }

    fn resolve_horizontal_bounds(&mut self, bounds: WorldBounds) {
        let half_width = self.config.size.x / 2.0;
        let min_x = bounds.left + half_width;
        let max_x = bounds.right - half_width;

        if self.position.x < min_x {
            self.position.x = min_x;
        } else if self.position.x > max_x {
            self.position.x = max_x;
        }
    }

    fn resolve_floor(&mut self, bounds: WorldBounds) {
        let floor_position = bounds.floor_y - self.config.size.y / 2.0;

        if self.position.y >= floor_position && self.velocity.y >= 0.0 {
            self.position.y = floor_position;
            self.velocity.y = 0.0;
            self.grounded = true;
        } else {
            self.grounded = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

    fn player_on_floor() -> Player {
        let config = PlayerConfig::default();
        let mut player = Player::new(Vec2::ZERO, config);
        player.place_on_floor(200.0, BOUNDS);
        player
    }

    #[test]
    fn horizontal_input_moves_at_configured_speed() {
        let mut player = player_on_floor();

        player.update(
            PlayerInput {
                move_axis: 1.0,
                jump_pressed: false,
            },
            0.5,
            BOUNDS,
        );

        assert!((player.position().x - 310.0).abs() < f32::EPSILON);
        assert!((player.velocity().x - 220.0).abs() < f32::EPSILON);
    }

    #[test]
    fn horizontal_input_is_clamped_to_the_world() {
        let mut player = player_on_floor();

        player.update(
            PlayerInput {
                move_axis: 1.0,
                jump_pressed: false,
            },
            10.0,
            BOUNDS,
        );

        assert_eq!(player.position().x, 944.0);
    }

    #[test]
    fn jump_applies_upward_velocity_and_gravity() {
        let mut player = player_on_floor();

        player.update(
            PlayerInput {
                move_axis: 0.0,
                jump_pressed: true,
            },
            0.0,
            BOUNDS,
        );

        assert!(!player.is_grounded());
        assert!((player.velocity().y + 480.0).abs() < f32::EPSILON);

        player.update(PlayerInput::default(), 0.1, BOUNDS);

        assert!((player.velocity().y + 340.0).abs() < f32::EPSILON);
        assert!(player.position().y < 436.0);
    }

    #[test]
    fn jump_input_is_ignored_while_airborne() {
        let mut player = player_on_floor();
        player.update(
            PlayerInput {
                move_axis: 0.0,
                jump_pressed: true,
            },
            0.0,
            BOUNDS,
        );

        player.update(
            PlayerInput {
                move_axis: 0.0,
                jump_pressed: true,
            },
            0.1,
            BOUNDS,
        );

        assert!((player.velocity().y + 340.0).abs() < f32::EPSILON);
    }

    #[test]
    fn falling_player_lands_on_the_floor() {
        let mut player = player_on_floor();
        player.update(PlayerInput::default(), 1.0, BOUNDS);

        assert_eq!(player.position().y, 436.0);
        assert_eq!(player.velocity().y, 0.0);
        assert!(player.is_grounded());
    }
}
