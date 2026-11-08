use macroquad::math::Vec2;

use crate::player::Player;

pub const COIN_RADIUS: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coin {
    pub position: Vec2,
}

impl Coin {
    pub fn touches(&self, player: &Player) -> bool {
        let gap = ((self.position - player.position()).abs() - player.size() / 2.0).max(Vec2::ZERO);
        gap.length_squared() <= COIN_RADIUS * COIN_RADIUS
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player::PlayerConfig;

    #[test]
    fn circle_contact_includes_edge_but_excludes_gap_and_rectangle_corner() {
        let player = Player::new(Vec2::new(100.0, 100.0), PlayerConfig::default());
        for (offset, expected) in [
            (Vec2::ZERO, true),
            (Vec2::new(24.0, 0.0), true),
            (Vec2::new(24.01, 0.0), false),
            (Vec2::new(24.0, 32.0), false),
        ] {
            let coin = Coin {
                position: player.position() + offset,
            };
            assert_eq!(coin.touches(&player), expected);
        }
    }
}
