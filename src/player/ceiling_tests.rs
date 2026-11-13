use super::*;

const DT: f32 = 1.0 / 60.0;
/// Underside of the block below (y 340..360).
const UNDERSIDE: f32 = 360.0;

fn block() -> Terrain {
    Terrain::from_tuples(&[(100.0, 340.0, 400.0, 20.0)])
}

fn head(player: &Player) -> f32 {
    player.position.y - player.config.size.y / 2.0
}

fn dash() -> PlayerInput {
    PlayerInput {
        dash_pressed: true,
        ..PlayerInput::default()
    }
}

/// An equipped player on the floor at `x`, with the jump key held until its head stops at
/// `ceiling_y` (the room's top or the block's underside).
fn bumped_at(x: f32, bounds: WorldBounds, terrain: &Terrain, ceiling_y: f32) -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(x, bounds);
    player.collect_item(Item::Dash);
    let jump = PlayerInput {
        jump_pressed: true,
        jump_held: true,
        ..PlayerInput::default()
    };
    player.update_with_terrain(jump, DT, bounds, terrain);
    for _ in 0..60 {
        let held = PlayerInput {
            jump_held: true,
            ..PlayerInput::default()
        };
        player.update_with_terrain(held, DT, bounds, terrain);
        if player.velocity.y == 0.0 && (head(&player) - ceiling_y).abs() < 0.01 {
            return player;
        }
    }
    panic!("never bumped the ceiling at {ceiling_y}");
}

fn dash_along(player: &mut Player, bounds: WorldBounds, terrain: &Terrain, ceiling_y: f32) {
    player.update_with_terrain(dash(), DT, bounds, terrain);
    let started_x = player.position.x;
    for frame in 0..20 {
        player.update_with_terrain(PlayerInput::default(), DT, bounds, terrain);
        assert!(!terrain.overlaps_body(player.position, player.config.size));
        if frame < 10 {
            assert!(
                (head(player) - ceiling_y).abs() < 0.01,
                "frame {frame}: the dash left the ceiling ({})",
                head(player)
            );
        }
    }
    assert!(
        player.position.x > started_x + 100.0,
        "the dash did not move"
    );
}

#[test]
fn dashing_along_under_a_ceiling_stays_flush_and_uses_the_air_dash() {
    let bounds = WorldBounds::new(0.0, 960.0, 460.0);
    let terrain = block();
    let mut player = bumped_at(150.0, bounds, &terrain, UNDERSIDE);
    assert!(!player.grounded && !player.air_dash_used);
    dash_along(&mut player, bounds, &terrain, UNDERSIDE);
    // The dash was an air dash: it stays spent until the player lands, then comes back.
    assert!(player.air_dash_used);
    for _ in 0..120 {
        player.update_with_terrain(PlayerInput::default(), DT, bounds, &terrain);
    }
    assert!(player.grounded && !player.air_dash_used);
    assert!(!terrain.overlaps_body(player.position, player.config.size));
}

#[test]
fn a_second_air_dash_under_a_ceiling_is_refused_until_landing() {
    let bounds = WorldBounds::new(0.0, 960.0, 460.0);
    let terrain = block();
    let mut player = bumped_at(150.0, bounds, &terrain, UNDERSIDE);
    player.update_with_terrain(dash(), DT, bounds, &terrain);
    let used = player.air_dash_used;
    // Wait out the cooldown while still in the air (hold up with a second bump), then try again.
    player.dash_cooldown = 0.0;
    player.dash_remaining = 0.0;
    let x = player.position.x;
    player.update_with_terrain(dash(), DT, bounds, &terrain);
    assert!(used && !player.is_grounded());
    assert!(player.position.x <= x + 0.01, "a second air dash started");
    // Landing gives the air dash back.
    for _ in 0..120 {
        player.update_with_terrain(PlayerInput::default(), DT, bounds, &terrain);
    }
    assert!(player.is_grounded());
    let x = player.position.x;
    player.update_with_terrain(dash(), DT, bounds, &terrain);
    assert!(player.position.x > x + 1.0, "no dash after landing");
}

#[test]
fn dashing_along_the_room_top_never_leaves_the_room() {
    // A low room: top y=400, floor y=460. Once with no solids, once with a solid far away
    // (so both movement paths are used).
    let bounds = WorldBounds::new(0.0, 960.0, 460.0).with_top(400.0);
    for terrain in [
        Terrain::default(),
        Terrain::from_tuples(&[(800.0, 440.0, 40.0, 20.0)]),
    ] {
        let mut player = bumped_at(150.0, bounds, &terrain, 400.0);
        assert_eq!(player.velocity.y, 0.0);
        dash_along(&mut player, bounds, &terrain, 400.0);
        assert!(head(&player) >= 400.0 - 0.01);
    }
}

#[test]
fn rising_knockback_stops_at_the_room_top_and_wipes_no_other_state() {
    let bounds = WorldBounds::new(0.0, 960.0, 460.0).with_top(400.0);
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(300.0, bounds);
    let hit = crate::enemy_attack::DamageHit {
        damage: 1,
        knockback: Vec2::new(0.0, -900.0),
        knockback_secs: 0.16,
    };
    assert!(player.take_attack_damage(hit));
    for _ in 0..30 {
        player.update(PlayerInput::default(), DT, bounds);
        assert!(head(&player) >= 400.0 - 0.01);
    }
    assert!(player.grounded && !player.air_dash_used);
}
