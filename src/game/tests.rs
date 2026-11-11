use super::*;

fn test_game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

#[test]
fn enemy_blasts_apply_configured_damage_once_and_respect_player_immunity() {
    use crate::behavior::BehaviorDefinition;
    use crate::enemy::{Enemy, EnemyDefinition};
    use std::sync::Arc;

    let mut game = test_game();
    let bounds = game.world_bounds();
    game.rooms[0].enemies = vec![Enemy::from_definition(
        &EnemyDefinition {
            name: "blaster".into(),
            health: 3,
            size: (36.0, 36.0),
            visual_size: None,
            contact_damage: 0,
            behavior: "blast".into(),
            body: None,
            death_secs: 0.0,
            movement: Default::default(),
        },
        Arc::new(
            BehaviorDefinition::from_ron(
                r#"(initial_state: "blast", states: [
            (name: "blast", actions: [
                area_blast(count: 2, radius: 2000.0, fuse_time: 0.5, damage: 30),
                area_blast(count: 1, radius: 2000.0, fuse_time: 0.5, damage: 30),
                area_blast(count: 1, radius: 2000.0, fuse_time: 1.5, damage: 10)
            ], transitions: [])])"#,
            )
            .unwrap(),
        ),
        600.0,
        600.0,
        bounds,
    )
    .unwrap()];
    game.update(PlayerInput::default(), 0.25);
    assert_eq!(game.player.health().current(), 50);
    game.update(PlayerInput::default(), 0.25);
    assert_eq!(game.player.health().current(), 20);
    game.update(PlayerInput::default(), 0.5);
    assert_eq!(game.player.health().current(), 20);
    game.update(PlayerInput::default(), 1.5);
    assert_eq!(game.player.health().current(), 10);
    game.update(PlayerInput::default(), 2.0);
    assert_eq!(game.player.health().current(), 10);
}

#[test]
fn attack_damage_wins_over_contact_and_immunity_blocks_knockback() {
    use crate::behavior::BehaviorDefinition;
    use crate::enemy::{Enemy, EnemyDefinition};
    use std::sync::Arc;
    let mut game = test_game();
    let bounds = game.world_bounds();
    let x = game.player.position().x;
    game.rooms[0].enemies = vec![Enemy::from_definition(
        &EnemyDefinition { name: "attacker".into(), health: 3, size: (36.0, 36.0), visual_size: None, contact_damage: 10, behavior: "blast".into(), body: None, death_secs: 0.0, movement: Default::default() },
        Arc::new(BehaviorDefinition::from_ron(r#"(initial_state: "fire", states: [(name: "fire", actions: [area_blast(radius: 200.0, damage: 20, knockback: (100.0, -100.0), active_secs: 2.0, telegraph_secs: 0.0)], transitions: [])])"#).unwrap()),
        x, x, bounds).unwrap()];
    game.update(PlayerInput::default(), 0.1);
    assert_eq!(game.player.health().current(), 30);
    assert!(game.player.velocity().y < 0.0);
    game.update(PlayerInput::default(), 0.1);
    assert_eq!(game.player.health().current(), 30);
}

fn special_game() -> GameState {
    let mut game = test_game();
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(240.0, game.world_bounds());
    game
}

#[test]
fn special_attacks_cycle_every_three_seconds_and_apply_health_damage() {
    let mut game = special_game();
    assert!(game.room().exits_locked());
    for (wait, kind, windup) in [
        (3.0, SpecialAttack::Slam, 0.5),
        (2.5, SpecialAttack::Summon, 1.0),
        (2.0, SpecialAttack::Rings, 1.0),
        (2.0, SpecialAttack::Slam, 0.5),
    ] {
        game.update(PlayerInput::default(), wait);
        assert_eq!(
            game.room().boss_special.as_ref().unwrap().active_attack(),
            Some(kind)
        );
        let health = game.player.health().current();
        game.update(PlayerInput::default(), 0.0);
        assert_eq!(game.player.health().current(), health);
        game.update(PlayerInput::default(), windup);
        if kind == SpecialAttack::Slam {
            assert_eq!(game.player.health().current(), health - 10);
            assert!(game.player.health().is_invincible());
        }
        assert!(!game.room().boss_special.as_ref().unwrap().is_telegraphing());
    }
    assert!(game
        .room()
        .boss_special
        .as_ref()
        .unwrap()
        .rings()
        .iter()
        .all(|ring| ring.exploded()));
}

#[test]
fn specials_pause_in_inactive_rooms_and_inventory_and_ignore_invalid_time() {
    let mut game = test_game();
    game.update(PlayerInput::default(), 100.0);
    assert_eq!(game.special_attack_timer, 0.0);
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(240.0, game.world_bounds());
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        game.update(PlayerInput::default(), dt);
    }
    assert_eq!(game.special_attack_timer, 0.0);
    game.update(PlayerInput::default(), 3.0);
    game.toggle_inventory();
    game.update(PlayerInput::default(), 100.0);
    assert!(game.room().boss_special.as_ref().unwrap().is_telegraphing());
    assert_eq!(game.player.health().current(), 50);
    game.toggle_inventory();
    game.update(PlayerInput::default(), 0.5);
    assert_eq!(game.player.health().current(), 40);
    game.update(PlayerInput::default(), f32::MAX);
    assert!(game.special_attack_timer.is_finite());
}

#[test]
fn summoned_enemies_use_room_combat_knockback_contact_and_stay_dead() {
    let mut game = special_game();
    let bounds = game.world_bounds();
    game.rooms[2].boss_special.as_mut().unwrap().start_attack(
        SpecialAttack::Summon,
        game.player.position(),
        bounds,
    );
    game.update(PlayerInput::default(), 1.0);
    assert_eq!(game.room().enemies.len(), 2);
    game.player.place_on_floor(120.0, bounds);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player.health().current(), 40);
    game.player.place_on_floor(80.0, bounds);
    let swing = PlayerInput {
        attack_pressed: true,
        ..PlayerInput::default()
    };
    game.update(swing, 0.0);
    assert_eq!(game.room().enemies[0].health(), 20);
    game.update(PlayerInput::default(), 0.1);
    assert!(game.room().enemies[0].position().x > 120.0);
    game.rooms[2].enemies[0].hit(100, 1.0);
    game.rooms[2].enemies[0].hit(101, 1.0);
    game.update(PlayerInput::default(), 0.0);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().enemies.len(), 1);
}

#[test]
fn special_melee_hits_once_per_swing_respects_facing_and_range() {
    let mut game = special_game();
    game.player.place_on_floor(520.0, game.world_bounds());
    let swing = PlayerInput {
        attack_pressed: true,
        ..PlayerInput::default()
    };
    game.update(swing, 0.0);
    game.update(swing, 0.0);
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 990);
    game.update(PlayerInput::default(), crate::combat::ATTACK_COOLDOWN);
    game.update(
        PlayerInput {
            move_axis: -1.0,
            ..swing
        },
        0.0,
    );
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 990);
    game.player.place_on_floor(800.0, game.world_bounds());
    game.update(PlayerInput::default(), crate::combat::ATTACK_COOLDOWN);
    game.update(swing, 0.0);
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 990);
}

#[test]
fn special_death_unlocks_arena_and_coins_are_collected_without_respawning() {
    let mut game = special_game();
    game.rooms[2]
        .boss_special
        .as_mut()
        .unwrap()
        .damage_boss(990);
    game.player.place_on_floor(520.0, game.world_bounds());
    game.update(
        PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert!(game.room().boss_special.as_ref().unwrap().is_defeated());
    assert!(!game.room().exits_locked());
    assert!(game.coins().is_empty());
    game.update(
        PlayerInput::default(),
        crate::boss_special::DEATH_EFFECT_SECONDS,
    );
    assert_eq!(game.coins().len() as u32 + game.coin_count(), 20);
    let position = game.coins()[0].position;
    game.player.place_at(position, game.world_bounds());
    let count = game.coin_count();
    game.update(PlayerInput::default(), 0.0);
    assert!(game.coin_count() > count);
    let remaining = game.coins().len();
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.coins().len(), remaining);
    game.player.place_on_floor(16.0, game.world_bounds());
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().id, RoomId::Second);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Boss);
    game.update(PlayerInput::default(), 100.0);
    assert!(game.room().boss_special.as_ref().unwrap().is_defeated());
    assert_eq!(game.coins().len(), remaining);
    assert_eq!(game.coins().len() as u32 + game.coin_count(), 20);
    assert!(game.room().enemies.is_empty());
}

#[test]
fn contact_collects_all_touching_coins_once_without_pickup_input() {
    let mut game = test_game();
    let position = game.player.position();
    game.rooms[0].coins = vec![Coin { position }; 2];
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.coin_count(), 2);
    assert!(game.coins().is_empty());
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.coin_count(), 2);
}

#[test]
fn distant_coins_are_not_collected_even_with_pickup_input() {
    let mut game = test_game();
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert_eq!(game.coin_count(), 0);
    assert_eq!(game.coins().len(), 3);
}

#[test]
fn airborne_contact_collects_coins() {
    let mut game = test_game();
    game.player
        .place_at(Vec2::new(360.0, 250.0), game.world_bounds());
    game.rooms[0].coins = vec![Coin {
        position: game.player.position(),
    }];
    assert!(!game.player.is_grounded());
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.coin_count(), 1);
}

#[test]
fn coins_persist_through_room_travel_and_respawn() {
    let mut game = test_game();
    game.rooms[0].coins = vec![Coin {
        position: game.player.position(),
    }];
    game.update(PlayerInput::default(), 0.0);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.coin_count(), 1);
    for _ in 0..5 {
        game.player.take_damage();
        game.player
            .update(PlayerInput::default(), 1.0, game.world_bounds());
    }
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.coin_count(), 1);
    assert_eq!(game.player.health().current(), 50);
    game.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::First);
    assert!(game.coins().is_empty());
    assert_eq!(game.coin_count(), 1);
}

#[test]
fn inventory_pauses_simulation_until_closed() {
    let mut game = test_game();
    game.rooms[0].coins = vec![Coin {
        position: game.player.position(),
    }];
    let player = game.player.clone();
    let enemy_position = game.room().enemies[0].position();
    game.toggle_inventory();
    assert!(game.inventory_open());
    game.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: true,
            pickup_pressed: true,
            attack_pressed: true,
            jump_held: true,
            dash_pressed: true,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.player, player);
    assert_eq!(game.room().enemies[0].position(), enemy_position);
    assert_eq!(game.coin_count(), 0);
    assert_eq!(game.room().id, RoomId::First);
    game.toggle_inventory();
    assert!(!game.inventory_open());
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.coin_count(), 1);
}

#[test]
fn new_game_starts_with_player_on_the_floor() {
    let game = test_game();

    assert!(game.player().is_grounded());
    assert_eq!(game.player().position().y, FLOOR_Y - 24.0);
}

#[test]
fn game_forwards_input_to_the_player() {
    let mut game = test_game();

    game.update(
        PlayerInput {
            move_axis: -1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        0.25,
    );

    assert_eq!(game.player().position().x, 125.0);
}

#[test]
fn death_uses_current_room_spawn_and_inactive_enemies_stay_frozen() {
    let mut game = test_game();
    game.current = RoomId::Second.index();
    let first_enemy_position = game.rooms[RoomId::First.index()].enemies[0].position();
    let bounds = game.world_bounds();
    let room = &mut game.rooms[RoomId::Second.index()];
    room.spawn = Vec2::new(350.0, 200.0);
    room.enemies
        .push(crate::enemy::Enemy::new(500.0, 500.0, 0.0, 0.0, bounds));
    game.player.place_on_floor(500.0, bounds);
    for expected in [40, 30, 20, 10, 50] {
        game.update(PlayerInput::default(), 1.0);
        assert_eq!(game.player.health().current(), expected);
    }
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.player.position(), game.room().spawn);
    assert!(!game.player.is_grounded());
    assert_eq!(game.player.velocity(), Vec2::ZERO);
    assert_eq!(
        game.rooms[RoomId::First.index()].enemies[0].position(),
        first_enemy_position
    );
}

#[test]
fn melee_respects_range_facing_and_hits_each_enemy_once_per_swing() {
    let mut game = test_game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![
        crate::enemy::Enemy::new(250.0, 250.0, 0.0, 0.0, bounds),
        crate::enemy::Enemy::new(150.0, 150.0, 0.0, 0.0, bounds),
        crate::enemy::Enemy::new(400.0, 400.0, 0.0, 0.0, bounds),
    ];
    let swing = PlayerInput {
        attack_pressed: true,
        ..PlayerInput::default()
    };
    game.update(swing, 0.0);
    game.update(swing, 0.0);
    assert_eq!(game.room().enemies[0].health(), 20);
    assert_eq!(game.room().enemies[1].health(), 30);
    assert_eq!(game.room().enemies[2].health(), 30);
    game.update(PlayerInput::default(), crate::combat::ATTACK_COOLDOWN);
    game.update(
        PlayerInput {
            move_axis: -1.0,
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert_eq!(game.room().enemies[1].health(), 20);
}

#[test]
fn lethal_swing_removes_enemy_before_contact_damage_and_death_persists() {
    let mut game = test_game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![crate::enemy::Enemy::new(230.0, 230.0, 0.0, 0.0, bounds)];
    game.rooms[0].enemies[0].hit(100, 1.0);
    game.rooms[0].enemies[0].hit(101, 1.0);
    game.update(
        PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert!(game.room().enemies.is_empty());
    assert_eq!(game.player.health().current(), 50);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Second);
    game.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::First);
    assert!(game.room().enemies.is_empty());
}

#[test]
fn room_transition_clears_active_dash_and_preserves_ability() {
    let mut game = test_game();
    game.player.collect_item(crate::item::Item::Dash);
    game.player.place_on_floor(900.0, game.world_bounds());
    game.update(
        PlayerInput {
            dash_pressed: true,
            ..PlayerInput::default()
        },
        0.05,
    );

    assert_eq!(game.room().id, RoomId::Second);
    let spawn = game.player.position();
    game.update(PlayerInput::default(), 0.05);
    assert_eq!(game.player.position(), spawn);
    assert_eq!(game.player.velocity(), Vec2::ZERO);
    game.update(
        PlayerInput {
            dash_pressed: true,
            ..PlayerInput::default()
        },
        0.05,
    );
    assert_eq!(game.player.position().x, spawn.x + 40.0);
}

#[test]
fn death_clears_active_dash_and_preserves_collected_ability() {
    let mut game = test_game();
    let bounds = game.world_bounds();
    for _ in 0..4 {
        assert!(game.player.take_damage());
        game.player.update(PlayerInput::default(), 1.0, bounds);
    }
    game.player.collect_item(crate::item::Item::Dash);
    game.player.place_on_floor(500.0, bounds);
    game.rooms[0].enemies = vec![crate::enemy::Enemy::new(500.0, 500.0, 0.0, 0.0, bounds)];
    game.update(
        PlayerInput {
            dash_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );

    assert_eq!(game.player.health().current(), crate::health::MAX_HEALTH);
    assert!(game.player.health().is_invincible());
    let spawn = game.room().spawn;
    assert_eq!(game.player.position(), spawn);
    game.update(PlayerInput::default(), 0.05);
    assert_eq!(game.player.position(), spawn);
    assert_eq!(game.player.velocity(), Vec2::ZERO);
    assert_eq!(game.player.collected_items(), &[crate::item::Item::Dash]);
    game.update(
        PlayerInput {
            dash_pressed: true,
            ..PlayerInput::default()
        },
        0.05,
    );
    assert_eq!(game.player.position().x, spawn.x + 40.0);
}

/// Arena boss from the `ground_pound` fixture (100 health, 20 contact damage) at the
/// Ash Warden spot, with its attack pause pushed out so only contact can hurt.
fn pound_game() -> GameState {
    let mut game = special_game();
    let mut definition = crate::boss_definition::BossDefinition::from_ron(include_str!(
        "../../tests/fixtures/bosses/ground_pound.ron"
    ))
    .unwrap();
    definition.pause_seconds = 1_000.0;
    game.rooms[2].boss_special = Some(
        crate::boss_special::BossSpecials::from_definition(
            std::sync::Arc::new(definition),
            Vec2::new(560.0, FLOOR_Y - 24.0),
        )
        .unwrap(),
    );
    game
}

#[test]
fn lethal_special_swing_prevents_contact_unlocks_exit_and_death_persists() {
    let mut game = pound_game();
    game.rooms[2].boss_special.as_mut().unwrap().damage_boss(90);
    game.player.place_on_floor(520.0, game.world_bounds());
    assert!(game.room().exits_locked());
    game.update(
        PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert_eq!(game.player.health().current(), 50);
    assert!(game.room().boss_special.as_ref().unwrap().is_defeated());
    assert!(!game.room().exits_locked());
    game.player.place_on_floor(16.0, game.world_bounds());
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().id, RoomId::Second);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Boss);
    assert!(game.room().boss_special.as_ref().unwrap().is_defeated());
    assert!(!game.room().exits_locked());
    game.player.place_on_floor(560.0, game.world_bounds());
    game.update(PlayerInput::default(), 10.0);
    assert_eq!(game.player.health().current(), 50);
}

#[test]
fn special_contact_deals_configured_damage_respects_immunity_and_respawn_keeps_lock() {
    let mut game = pound_game();
    game.player.place_on_floor(560.0, game.world_bounds());
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player.health().current(), 30);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player.health().current(), 30);
    for expected in [10, 50] {
        game.update(PlayerInput::default(), 1.0);
        assert_eq!(game.player.health().current(), expected);
    }
    assert_eq!(game.player.position(), game.room().spawn);
    assert_eq!(game.room().boss_special.as_ref().unwrap().health(), 100);
    assert!(game.room().exits_locked());
}

#[test]
fn transition_honors_an_airborne_spawn() {
    let mut game = test_game();
    let spawn = Vec2::new(300.0, 200.0);
    game.rooms[RoomId::First.index()].exits[0].spawn = spawn;

    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );

    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.player().position(), spawn);
    assert_eq!(game.player().velocity(), Vec2::ZERO);
    assert!(!game.player().is_grounded());
}
