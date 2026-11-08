use hollow_steps::boss::{Boss, BossAttack, BossState, BOSS_PAUSE, WARDEN_MAX_HEALTH};
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::{test_rooms, RoomEdge, RoomId};
use macroquad::math::{Rect, Vec2};

fn warden() -> Boss {
    Boss::ash_warden(Vec2::new(560.0, 420.0))
}

#[test]
fn state_transitions_include_telegraph_active_cooldown_and_pause() {
    let mut boss = warden();
    assert_eq!(boss.state(), BossState::Idle);
    assert!(boss.attack_area().is_none());
    boss.update(BOSS_PAUSE);
    assert_eq!(boss.state(), BossState::Telegraphing);
    let preview = boss.attack_area().unwrap();
    boss.update(0.5);
    assert_eq!(boss.state(), BossState::Telegraphing);
    boss.update(0.25);
    assert_eq!(boss.state(), BossState::Attacking);
    assert_eq!(boss.attack_area(), Some(preview));
    boss.update(0.25);
    assert_eq!(boss.state(), BossState::Idle);
    assert!(boss.attack_area().is_none());
    boss.update(0.5); // Outgoing attack cooldown, then a separate pause.
    assert_eq!(boss.state(), BossState::Idle);
    boss.update(BOSS_PAUSE);
    assert_eq!(boss.state(), BossState::Telegraphing);
}

#[test]
fn attacks_cycle_in_order_and_wrap_to_the_first() {
    let mut boss = warden();
    assert_eq!(boss.current_attack().name, "Low sweep");
    boss.update(BOSS_PAUSE + 0.75 + 0.25);
    assert_eq!(boss.current_attack().name, "Crown flare");
    boss.update(0.5 + BOSS_PAUSE + 0.75);
    assert_eq!(boss.state(), BossState::Attacking);
    assert_eq!(boss.current_attack().name, "Crown flare");
    boss.update(0.5 + 0.75 + BOSS_PAUSE);
    assert_eq!(boss.state(), BossState::Telegraphing);
    assert_eq!(boss.current_attack().name, "Low sweep");
}

#[test]
fn custom_attack_list_reuses_the_same_machine_and_single_attack_wraps() {
    let mut boss = Boss::new(
        Vec2::ZERO,
        Vec2::ONE,
        2,
        vec![BossAttack {
            name: "Pulse",
            area: Rect::new(-5.0, -5.0, 10.0, 10.0),
            telegraph_seconds: 0.25,
            active_seconds: 0.125,
            cooldown_seconds: 0.125,
        }],
    );
    let mut small_step = boss.clone();
    small_step.update(0.75);
    boss.update(1_000_000.75); // One-second full cycles plus the initial pause.
    assert_eq!(boss.state(), BossState::Attacking);
    assert_eq!(boss.state(), small_step.state());
    assert_eq!(boss.current_attack().name, "Pulse");
    boss.update(0.125);
    small_step.update(0.125);
    assert_eq!(boss.state(), small_step.state());
    boss.update(f32::MAX); // Must finish in bounded time.
    assert!(!boss.is_dead());
}

#[test]
fn overshoot_matches_stepped_time_and_invalid_time_does_not_advance() {
    let mut boss = warden();
    boss.update(BOSS_PAUSE);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        boss.update(dt);
        assert_eq!(boss.state(), BossState::Telegraphing);
        assert_eq!(boss.current_attack().name, "Low sweep");
    }
    let mut stepped = boss.clone();
    boss.update(32.125);
    for _ in 0..257 {
        stepped.update(0.125);
    }
    for _ in 0..40 {
        assert_eq!(boss.state(), stepped.state());
        assert_eq!(boss.current_attack().name, stepped.current_attack().name);
        boss.update(0.125);
        stepped.update(0.125);
    }
}

#[test]
fn body_contact_is_live_but_only_active_attack_areas_damage() {
    let mut boss = warden();
    let target = Vec2::new(440.0, 446.0); // In sweep, outside the boss body.
    assert!(boss.damages(boss.position(), Vec2::ONE));
    assert!(!boss.damages(target, Vec2::ONE));
    boss.update(BOSS_PAUSE);
    assert!(boss.attack_area().unwrap().contains(target));
    assert!(!boss.damages(target, Vec2::ONE));
    boss.update(0.75);
    assert!(boss.damages(target, Vec2::ONE));
    assert!(!boss.damages(Vec2::new(200.0, 446.0), Vec2::ONE));
    boss.update(0.25);
    assert!(!boss.damages(target, Vec2::ONE));
    assert!(boss.damages(boss.position(), Vec2::ONE));
}

#[test]
fn thirty_distinct_swings_kill_once_and_death_stops_attacks_and_contact() {
    let mut boss = warden();
    assert_eq!(boss.health(), WARDEN_MAX_HEALTH);
    assert_eq!(boss.max_health(), 30);
    boss.update(BOSS_PAUSE + 0.75);
    let area = boss.attack_area().unwrap();
    for id in 1..=30 {
        assert!(boss.hit(id));
        assert!(!boss.hit(id));
        assert_eq!(boss.health(), 30 - id as u8);
        assert_eq!(boss.is_dead(), id == 30);
    }
    assert_eq!(boss.state(), BossState::Dead);
    assert!(!boss.hit(31));
    assert!(!boss.touches(boss.position(), boss.size()));
    assert!(!boss.damages(area.center(), area.size()));
    assert!(boss.attack_area().is_none());
    boss.update(f32::MAX);
    assert_eq!(boss.state(), BossState::Dead);
    assert_eq!(boss.health(), 0);
}

#[test]
fn every_exit_edge_locks_while_alive_and_unlocks_immediately_on_death() {
    let [_, _, mut room] = test_rooms();
    for (edge, position) in [
        (RoomEdge::Left, Vec2::new(0.0, 100.0)),
        (RoomEdge::Right, Vec2::new(960.0, 100.0)),
        (RoomEdge::Top, Vec2::new(100.0, 0.0)),
        (RoomEdge::Bottom, Vec2::new(100.0, 460.0)),
    ] {
        room.exits[0].edge = edge;
        room.exits[0].offset = 60.0;
        room.boss = Some(warden());
        assert!(room.exits_locked());
        assert!(room.exit_at(position, Vec2::ONE).is_none());
        for id in 1..=30 {
            room.boss.as_mut().unwrap().hit(id);
        }
        assert!(!room.exits_locked());
        assert!(room.exit_at(position, Vec2::ONE).is_some());
        assert!(room.exit_at(Vec2::new(500.0, 200.0), Vec2::ONE).is_none());
    }
    room.boss = None;
    assert!(!room.exits_locked());
}

#[test]
fn arena_is_reachable_from_second_room_and_live_boss_blocks_return() {
    let mut game = GameState::new();
    for expected in [RoomId::Second, RoomId::Boss] {
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            10.0,
        );
        assert_eq!(game.room().id, expected);
    }
    assert_eq!(game.room().boss.as_ref().unwrap().health(), 30);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().id, RoomId::Boss);
    game.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Boss);
    assert!(game.room().exits_locked());
}

#[test]
#[should_panic]
fn empty_attack_lists_are_rejected() {
    Boss::new(Vec2::ZERO, Vec2::ONE, 30, vec![]);
}

#[test]
#[should_panic]
fn zero_duration_telegraphs_are_rejected() {
    let mut attack = warden().current_attack().clone();
    attack.telegraph_seconds = 0.0;
    Boss::new(Vec2::ZERO, Vec2::ONE, 30, vec![attack]);
}
