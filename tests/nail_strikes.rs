use hollow_steps::combat::{
    Attack, StrikeConfig, StrikeDirection, ATTACK_COOLDOWN, ATTACK_HEIGHT, ATTACK_REACH,
    SWING_SECONDS,
};
use hollow_steps::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use hollow_steps::tuning::{Field, TuningConfig};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const POSITION: Vec2 = Vec2::new(200.0, 300.0);
const SIZE: Vec2 = Vec2::new(32.0, 48.0);
const DT: f32 = 1.0 / 120.0;

/// Sizes that differ in every slot, so a box using the wrong slot is caught.
fn custom() -> StrikeConfig {
    StrikeConfig {
        forward_width: 50.0,
        forward_height: 20.0,
        up_width: 30.0,
        up_height: 60.0,
        down_width: 24.0,
        down_height: 44.0,
        pogo_speed: 300.0,
    }
}

fn swing(strike: StrikeDirection, facing: f32) -> Attack {
    let mut attack = Attack::default();
    attack.update_directed(true, facing, strike, 0.0);
    attack
}

fn aim(attack: bool, up: bool, down: bool) -> PlayerInput {
    PlayerInput {
        attack_pressed: attack,
        up_held: up,
        down_held: down,
        ..PlayerInput::default()
    }
}

fn airborne(config: PlayerConfig) -> Player {
    let mut player = Player::new(Vec2::ZERO, config);
    player.place_at(Vec2::new(200.0, 200.0), BOUNDS);
    assert!(!player.is_grounded());
    player
}

fn on_floor() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(200.0, BOUNDS);
    assert!(player.is_grounded());
    player
}

#[test]
fn each_direction_has_its_own_hitbox_size_and_place() {
    let config = custom();
    let right = swing(StrikeDirection::Forward, 1.0)
        .hitbox_with(POSITION, SIZE, &config)
        .unwrap();
    assert_eq!(
        (right.x, right.y, right.w, right.h),
        (216.0, 290.0, 50.0, 20.0)
    );
    let left = swing(StrikeDirection::Forward, -1.0)
        .hitbox_with(POSITION, SIZE, &config)
        .unwrap();
    assert_eq!((left.x, left.y, left.w, left.h), (134.0, 290.0, 50.0, 20.0));
    // Up: centered over the head, its bottom edge on the top of the body (300 - 24).
    let up = swing(StrikeDirection::Up, 1.0)
        .hitbox_with(POSITION, SIZE, &config)
        .unwrap();
    assert_eq!((up.x, up.y, up.w, up.h), (185.0, 216.0, 30.0, 60.0));
    assert_eq!(up.y + up.h, POSITION.y - SIZE.y / 2.0);
    // Down: centered under the feet, its top edge on the bottom of the body (300 + 24).
    let down = swing(StrikeDirection::Down, 1.0)
        .hitbox_with(POSITION, SIZE, &config)
        .unwrap();
    assert_eq!((down.x, down.y, down.w, down.h), (188.0, 324.0, 24.0, 44.0));
    assert_eq!(down.y, POSITION.y + SIZE.y / 2.0);
}

#[test]
fn up_and_down_boxes_are_centered_whichever_way_the_player_faces() {
    for strike in [StrikeDirection::Up, StrikeDirection::Down] {
        let facing_right = swing(strike, 1.0).hitbox_with(POSITION, SIZE, &custom());
        let facing_left = swing(strike, -1.0).hitbox_with(POSITION, SIZE, &custom());
        assert_eq!(facing_right, facing_left);
    }
}

#[test]
fn no_hitbox_exists_before_a_swing_or_after_it_ends() {
    let config = custom();
    for strike in [
        StrikeDirection::Forward,
        StrikeDirection::Up,
        StrikeDirection::Down,
    ] {
        let mut attack = Attack::default();
        assert!(attack.hitbox_with(POSITION, SIZE, &config).is_none());
        attack.update_directed(true, 1.0, strike, 0.0);
        assert!(attack.hitbox_with(POSITION, SIZE, &config).is_some());
        attack.update_directed(false, 1.0, strike, SWING_SECONDS);
        assert!(attack.hitbox_with(POSITION, SIZE, &config).is_none());
    }
}

#[test]
fn the_shipped_forward_box_is_the_old_swing_and_hitbox_defaults_to_the_shipped_data() {
    let shipped = StrikeConfig::default();
    assert_eq!(shipped, TuningConfig::default().player.strike);
    assert_eq!(shipped.forward_width, ATTACK_REACH);
    assert_eq!(shipped.forward_height, ATTACK_HEIGHT);
    let attack = swing(StrikeDirection::Up, 1.0);
    assert_eq!(
        attack.hitbox(POSITION, SIZE),
        attack.hitbox_with(POSITION, SIZE, &shipped)
    );
    assert!(shipped.pogo_speed > 0.0);
}

#[test]
fn held_keys_pick_the_direction_and_down_needs_the_air() {
    use StrikeDirection::{Down, Forward, Up};
    // (up, down, airborne) -> direction
    for (up, down, airborne, expected) in [
        (false, false, false, Forward),
        (false, false, true, Forward),
        (true, false, false, Up),
        (true, false, true, Up),
        (false, true, true, Down),
        (false, true, false, Forward),
        (true, true, true, Up),
        (true, true, false, Up),
    ] {
        assert_eq!(
            StrikeDirection::from_aim(up, down, airborne),
            expected,
            "up {up} down {down} airborne {airborne}"
        );
    }
}

#[test]
fn a_swing_keeps_its_direction_and_only_the_next_swing_can_change_it() {
    let mut attack = swing(StrikeDirection::Up, 1.0);
    let id = attack.id();
    attack.update_directed(false, 1.0, StrikeDirection::Down, 0.0);
    assert_eq!(attack.strike(), StrikeDirection::Up);
    // A press during the cooldown starts nothing.
    attack.update_directed(true, 1.0, StrikeDirection::Down, SWING_SECONDS);
    assert_eq!((attack.strike(), attack.id()), (StrikeDirection::Up, id));
    attack.update_directed(true, 1.0, StrikeDirection::Down, ATTACK_COOLDOWN);
    assert_eq!(attack.strike(), StrikeDirection::Down);
    assert_ne!(attack.id(), id);
    // The old entry point is a forward swing.
    attack.update(true, 1.0, ATTACK_COOLDOWN);
    assert_eq!(attack.strike(), StrikeDirection::Forward);
}

#[test]
fn knockback_follows_the_strike_direction() {
    // Forward swings push the way they point, wherever the target is.
    assert_eq!(
        swing(StrikeDirection::Forward, 1.0).knockback_direction(200.0, 100.0),
        1.0
    );
    assert_eq!(
        swing(StrikeDirection::Forward, -1.0).knockback_direction(200.0, 300.0),
        -1.0
    );
    // Up and down swings push enemies sideways, away from the player.
    for strike in [StrikeDirection::Up, StrikeDirection::Down] {
        let attack = swing(strike, -1.0);
        assert_eq!(attack.knockback_direction(200.0, 230.0), 1.0, "{strike:?}");
        assert_eq!(attack.knockback_direction(200.0, 170.0), -1.0, "{strike:?}");
        // Exactly level, or a bad number: fall back to the facing side, never zero.
        assert_eq!(attack.knockback_direction(200.0, 200.0), -1.0, "{strike:?}");
        assert_eq!(
            attack.knockback_direction(200.0, f32::NAN),
            -1.0,
            "{strike:?}"
        );
    }
    assert_eq!(
        swing(StrikeDirection::Down, 1.0).knockback_direction(200.0, 200.0),
        1.0
    );
}

#[test]
fn an_attack_gives_one_pogo_per_downward_swing_only() {
    let mut down = swing(StrikeDirection::Down, 1.0);
    assert!(down.take_pogo());
    assert!(!down.take_pogo(), "one bounce per swing");
    down.update_directed(true, 1.0, StrikeDirection::Down, ATTACK_COOLDOWN);
    assert!(down.take_pogo(), "a new swing bounces again");
    down.update_directed(false, 1.0, StrikeDirection::Down, SWING_SECONDS);
    // Spent anyway; check the ended-swing case on a fresh one.
    let mut ended = swing(StrikeDirection::Down, 1.0);
    ended.update_directed(false, 1.0, StrikeDirection::Down, SWING_SECONDS);
    assert!(!ended.take_pogo(), "no bounce once the swing is over");
    for strike in [StrikeDirection::Forward, StrikeDirection::Up] {
        assert!(!swing(strike, 1.0).take_pogo(), "{strike:?}");
    }
    assert!(!Attack::default().take_pogo());
}

#[test]
fn the_player_aims_a_new_swing_from_the_held_keys() {
    use StrikeDirection::{Down, Forward, Up};
    for (mut player, input, expected) in [
        (
            airborne(PlayerConfig::default()),
            aim(true, false, true),
            Down,
        ),
        (airborne(PlayerConfig::default()), aim(true, true, true), Up),
        (
            airborne(PlayerConfig::default()),
            aim(true, false, false),
            Forward,
        ),
        (on_floor(), aim(true, false, true), Forward),
        (on_floor(), aim(true, true, false), Up),
        (on_floor(), aim(false, false, true), Forward),
    ] {
        let id = player.attack().id();
        player.update(input, 0.0, BOUNDS);
        assert_eq!(player.attack().strike(), expected, "{input:?}");
        assert_eq!(player.attack().id() != id, input.attack_pressed);
    }
}

#[test]
fn the_player_hitbox_is_sized_by_its_own_strike_config() {
    let config = PlayerConfig {
        strike: custom(),
        ..PlayerConfig::default()
    };
    let mut player = airborne(config);
    assert!(player.strike_hitbox().is_none());
    player.update(aim(true, false, true), 0.0, BOUNDS);
    let hitbox = player.strike_hitbox().unwrap();
    assert_eq!((hitbox.w, hitbox.h), (24.0, 44.0));
    assert_eq!(hitbox.y, player.position().y + player.size().y / 2.0);
    // The same swing sized by the shipped data differs, so the config really is what is read.
    let shipped = player
        .attack()
        .hitbox(player.position(), player.size())
        .unwrap();
    assert_ne!((shipped.w, shipped.h), (hitbox.w, hitbox.h));
}

#[test]
fn a_downward_hit_in_the_air_bounces_the_player_up_once() {
    let config = PlayerConfig {
        strike: custom(),
        ..PlayerConfig::default()
    };
    let mut player = airborne(config);
    player.update(aim(true, false, true), 0.0, BOUNDS);
    assert!(player.pogo());
    assert_eq!(player.velocity().y, -300.0);
    assert!(!player.is_grounded());
    assert!(
        !player.pogo(),
        "a second hit of the same swing does not bounce again"
    );
    // The bounce rises under gravity like any jump.
    player.update(PlayerInput::default(), DT, BOUNDS);
    assert!(player.velocity().y > -300.0 && player.velocity().y < 0.0);
    assert!(player.position().y < 200.0);
}

#[test]
fn there_is_no_pogo_without_a_downward_swing_or_on_the_ground() {
    // In the air but swinging forward or up.
    for input in [aim(true, false, false), aim(true, true, false)] {
        let mut player = airborne(PlayerConfig::default());
        player.update(input, 0.0, BOUNDS);
        assert!(!player.pogo(), "{input:?}");
        assert_eq!(player.velocity().y, 0.0);
    }
    // No swing at all.
    let mut player = airborne(PlayerConfig::default());
    assert!(!player.pogo());
    // On the floor, Down + X is a forward swing and cannot bounce.
    let mut player = on_floor();
    player.update(aim(true, false, true), 0.0, BOUNDS);
    assert_eq!(player.attack().strike(), StrikeDirection::Forward);
    assert!(!player.pogo());
    assert_eq!(player.velocity().y, 0.0);
    assert!(player.is_grounded());
}

#[test]
fn a_downward_swing_that_lands_before_it_hits_does_not_bounce() {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    let floor_position = BOUNDS.floor_y - player.size().y / 2.0;
    player.place_at(Vec2::new(200.0, floor_position - 2.0), BOUNDS);
    assert!(!player.is_grounded());
    player.update(aim(true, false, true), 0.0, BOUNDS);
    assert_eq!(player.attack().strike(), StrikeDirection::Down);
    for _ in 0..10 {
        player.update(PlayerInput::default(), DT, BOUNDS);
    }
    assert!(player.is_grounded());
    // 10 ticks (0.083 s) is inside the 0.12 s swing, so only the floor is stopping the bounce.
    assert!(player.strike_hitbox().is_some());
    assert!(!player.pogo());
    assert_eq!(player.velocity().y, 0.0);
}

#[test]
fn strike_numbers_are_in_player_ron_with_ranges_and_round_trip() {
    let config = TuningConfig::default();
    config.validate().unwrap();
    let strike_fields: Vec<_> = Field::ALL
        .iter()
        .filter(|field| field.spec().group == "Strike")
        .collect();
    assert_eq!(strike_fields.len(), 7, "six box sizes and the pogo speed");
    for field in strike_fields {
        assert!(field.spec().key.starts_with("strike."));
        assert!(
            field.spec().min > 0.0,
            "{} must stay positive",
            field.spec().key
        );
    }
    for (field, bad) in [
        (Field::PogoSpeed, 0.0),
        (Field::StrikeUpHeight, 0.0),
        (Field::StrikeDownWidth, 10_000.0),
        (Field::StrikeForwardWidth, f32::NAN),
    ] {
        assert!(config.with_field(field, bad).is_err(), "{field:?} {bad}");
    }
    let edited = config
        .with_field(Field::StrikeUpWidth, 60.0)
        .and_then(|c| c.with_field(Field::PogoSpeed, 640.0))
        .unwrap();
    assert_eq!(edited.player.strike.up_width, 60.0);
    assert_eq!(edited.player.strike.pogo_speed, 640.0);
    assert_eq!(
        edited.player.strike.up_height,
        config.player.strike.up_height
    );
    let text = edited.to_ron().unwrap();
    assert_eq!(TuningConfig::parse(&text, "round trip").unwrap(), edited);
}

#[test]
fn a_player_file_without_the_strike_section_is_an_error_naming_it() {
    let shipped = include_str!("../data/player.ron");
    let mut skipping = false;
    let without: String = shipped
        .lines()
        .filter(|line| {
            let line = line.trim();
            if line == "strike: (" {
                skipping = true;
            }
            let keep = !skipping;
            if skipping && line == ")," {
                skipping = false;
            }
            keep
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!without.contains("pogo_speed"));
    let error = TuningConfig::parse(&without, "player.ron").unwrap_err();
    assert!(
        error.contains("player.ron") && error.contains("strike"),
        "{error}"
    );
}
