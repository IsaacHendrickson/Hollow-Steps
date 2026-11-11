use hollow_steps::editor::RoomLayout;
use hollow_steps::player::{Player, PlayerConfig, PlayerInput};
use hollow_steps::terrain::Terrain;
use macroquad::math::Vec2;

mod support;

const DT: f32 = 1.0 / 60.0;

fn layout(name: &str) -> RoomLayout {
    let path = support::fixture_assets().join("terrain").join(name);
    ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn terrain_room() -> (RoomLayout, Terrain) {
    let layout = layout("terrain_layout.ron");
    let terrain = Terrain::from_tuples(&layout.terrain);
    (layout, terrain)
}

fn player_at(layout: &RoomLayout, position: Vec2, terrain: &Terrain) -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_at(position, layout.world_bounds());
    // Settle onto whatever is below.
    for _ in 0..240 {
        player.update_with_terrain(PlayerInput::default(), DT, layout.world_bounds(), terrain);
    }
    player
}

fn step(player: &mut Player, input: PlayerInput, dt: f32, layout: &RoomLayout, t: &Terrain) {
    player.update_with_terrain(input, dt, layout.world_bounds(), t);
}

fn walk(axis: f32) -> PlayerInput {
    PlayerInput {
        move_axis: axis,
        ..PlayerInput::default()
    }
}

#[test]
fn falls_and_lands_on_a_platform() {
    let (layout, terrain) = terrain_room();
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_at(Vec2::new(360.0, 200.0), layout.world_bounds());
    for _ in 0..120 {
        step(&mut player, PlayerInput::default(), DT, &layout, &terrain);
    }
    assert!(player.is_grounded());
    assert!((player.position().y - (380.0 - 24.0)).abs() < 0.01);
    assert_eq!(player.velocity().y, 0.0);
}

#[test]
fn walking_under_a_platform_is_not_blocked() {
    let (layout, terrain) = terrain_room();
    let mut player = player_at(&layout, Vec2::new(250.0, 436.0), &terrain);
    for _ in 0..60 {
        step(&mut player, walk(1.0), DT, &layout, &terrain);
    }
    assert!(player.position().x > 400.0);
}

#[test]
fn walking_into_a_wall_stops_at_its_face() {
    let (layout, terrain) = terrain_room();
    let mut player = player_at(&layout, Vec2::new(500.0, 436.0), &terrain);
    for _ in 0..120 {
        step(&mut player, walk(1.0), DT, &layout, &terrain);
    }
    assert!((player.position().x - (600.0 - 16.0)).abs() < 0.01);
    assert_eq!(player.velocity().x, 0.0);
    assert!(player.is_grounded());
}

#[test]
fn head_bump_zeroes_upward_velocity() {
    let (layout, terrain) = terrain_room();
    let jump = PlayerInput {
        jump_pressed: true,
        jump_held: true,
        ..PlayerInput::default()
    };
    let highest_top = |terrain: &Terrain| {
        let mut player = player_at(&layout, Vec2::new(150.0, 436.0), &Terrain::default());
        let mut highest = f32::MAX;
        let mut bumped = false;
        step(&mut player, jump, DT, &layout, terrain);
        for _ in 0..90 {
            let rising = player.velocity().y < 0.0;
            step(
                &mut player,
                PlayerInput {
                    jump_held: true,
                    ..PlayerInput::default()
                },
                DT,
                &layout,
                terrain,
            );
            bumped |= rising && player.velocity().y >= 0.0 && player.velocity().y < 1.0;
            highest = highest.min(player.position().y - 24.0);
        }
        (highest, bumped)
    };
    let (free_top, _) = highest_top(&Terrain::default());
    let (bumped_top, bumped) = highest_top(&terrain);
    assert!(free_top < 360.0, "control jump must reach the block height");
    assert!(bumped_top >= 360.0 - 0.01, "head went through the block");
    assert!(bumped);
}

#[test]
fn dash_unaffected_without_terrain_and_huge_delta_does_not_tunnel() {
    let (layout, terrain) = terrain_room();
    // Huge fall delta onto a thin-ish platform.
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_at(Vec2::new(360.0, 100.0), layout.world_bounds());
    step(&mut player, PlayerInput::default(), 5.0, &layout, &terrain);
    assert!((player.position().y - (380.0 - 24.0)).abs() < 0.01);
    assert!(player.is_grounded());

    // Huge horizontal delta toward the wall.
    let mut player = player_at(&layout, Vec2::new(500.0, 436.0), &terrain);
    step(&mut player, walk(1.0), 5.0, &layout, &terrain);
    assert!((player.position().x - 584.0).abs() < 0.01);
}

#[test]
fn walking_off_a_ledge_still_allows_a_coyote_jump() {
    let (layout, terrain) = terrain_room();
    let mut player = player_at(&layout, Vec2::new(360.0, 100.0), &terrain);
    assert!((player.position().y - 356.0).abs() < 0.01);
    let mut guard = 0;
    while player.is_grounded() {
        step(&mut player, walk(1.0), DT, &layout, &terrain);
        guard += 1;
        assert!(guard < 120, "never walked off the platform");
    }
    let mut late = player.clone();
    let jump = PlayerInput {
        jump_pressed: true,
        ..PlayerInput::default()
    };
    step(&mut player, jump, DT, &layout, &terrain);
    assert!(player.velocity().y < 0.0, "coyote jump should fire");

    // Must not happen: jumping long after leaving the ledge.
    for _ in 0..30 {
        step(&mut late, PlayerInput::default(), DT, &layout, &terrain);
    }
    if !late.is_grounded() {
        step(&mut late, jump, DT, &layout, &terrain);
        assert!(late.velocity().y >= 0.0, "no jump after coyote expires");
    }
}

#[test]
fn jumping_works_from_a_platform() {
    let (layout, terrain) = terrain_room();
    let mut player = player_at(&layout, Vec2::new(360.0, 100.0), &terrain);
    let jump = PlayerInput {
        jump_pressed: true,
        ..PlayerInput::default()
    };
    step(&mut player, jump, DT, &layout, &terrain);
    assert!(player.velocity().y < 0.0);
    assert!(!player.is_grounded());
}

#[test]
fn old_room_files_load_without_terrain() {
    let legacy = layout("legacy_layout.ron");
    assert!(legacy.terrain.is_empty());
    legacy.validate_geometry().unwrap();
    // Saving an unchanged old room does not add a terrain field.
    let text = ron::to_string(&legacy).unwrap();
    assert!(!text.contains("terrain"));
}

#[test]
fn terrain_fixture_round_trips_and_validates() {
    let layout = layout("terrain_layout.ron");
    layout.validate_geometry().unwrap();
    let again: RoomLayout = ron::from_str(&ron::to_string(&layout).unwrap()).unwrap();
    assert_eq!(layout, again);
}

#[test]
fn invalid_terrain_is_rejected() {
    let base = layout("terrain_layout.ron");
    for bad in [
        (10.0, 10.0, 0.0, 10.0),
        (10.0, 10.0, 10.0, -5.0),
        (f32::NAN, 10.0, 10.0, 10.0),
        (950.0, 10.0, 50.0, 10.0),
        (10.0, 450.0, 10.0, 20.0),
        (-5.0, 10.0, 10.0, 10.0),
        (40.0, 410.0, 40.0, 40.0), // over the spawn point
    ] {
        let mut layout = base.clone();
        layout.terrain.push(bad);
        assert!(layout.validate_geometry().is_err(), "{bad:?} accepted");
    }
}

#[test]
fn items_and_exits_cannot_be_inside_terrain() {
    use hollow_steps::editor::{ExitPlacement, ItemPlacement};
    use hollow_steps::item::Item;
    use hollow_steps::room::{RoomEdge, RoomId};
    let mut with_item = layout("legacy_layout.ron");
    with_item.terrain.push((300.0, 380.0, 120.0, 20.0));
    with_item.items.push(ItemPlacement {
        item: Item::AmberShard,
        position: (360.0, 390.0),
    });
    assert!(with_item.validate_geometry().is_err());

    let mut with_exit = layout("legacy_layout.ron");
    with_exit.terrain.push((900.0, 300.0, 40.0, 100.0));
    with_exit.exits.push(ExitPlacement {
        edge: RoomEdge::Right,
        offset: 300.0,
        length: 96.0,
        target_room: RoomId::Second,
        spawn: (160.0, 376.0),
    });
    assert!(with_exit.validate_geometry().is_err());
}

fn lunger(bounds: hollow_steps::player::WorldBounds, x: f32) -> hollow_steps::enemy::Enemy {
    use hollow_steps::behavior::BehaviorDefinition;
    use hollow_steps::enemy::{Enemy, EnemyDefinition};
    let behavior =
        BehaviorDefinition::from_ron(include_str!("fixtures/attacks/reload_lunge.ron")).unwrap();
    Enemy::from_definition(
        &EnemyDefinition {
            name: "lunger".into(),
            health: 3,
            size: (36.0, 36.0),
            contact_damage: 1,
            behavior: "lunge".into(),
            body: None,
            death_secs: 0.0,
            movement: Default::default(),
        },
        std::sync::Arc::new(behavior),
        x,
        x,
        bounds,
    )
    .unwrap()
}

#[test]
fn lunge_cannot_pass_through_a_wall() {
    let (layout, _) = terrain_room();
    let bounds = layout.world_bounds();
    // Wall x 600..620 spans the floor; lunger aims right at a player beyond it.
    let terrain = Terrain::from_tuples(&[(600.0, 300.0, 20.0, 160.0)]);
    let target = Vec2::new(800.0, 442.0);
    let mut free = lunger(bounds, 560.0);
    let mut blocked = lunger(bounds, 560.0);
    for _ in 0..90 {
        free.update_with_terrain(
            target,
            Vec2::new(32.0, 48.0),
            DT,
            bounds,
            &Terrain::default(),
        );
        blocked.update_with_terrain(target, Vec2::new(32.0, 48.0), DT, bounds, &terrain);
    }
    assert!(
        free.position().x + 18.0 > 600.0,
        "control lunge must reach the wall"
    );
    assert!(blocked.position().x + 18.0 <= 600.0 + 0.01);
}

#[test]
fn lunge_aimed_upward_cannot_pass_through_a_platform() {
    let (layout, _) = terrain_room();
    let bounds = layout.world_bounds();
    // Thin platform just above the lunger, player standing on it.
    let terrain = Terrain::from_tuples(&[(400.0, 380.0, 200.0, 8.0)]);
    let target = Vec2::new(500.0, 350.0);
    let mut enemy = lunger(bounds, 500.0);
    let mut free = lunger(bounds, 500.0);
    let mut lowest_top = f32::MAX;
    let mut free_top = f32::MAX;
    for _ in 0..90 {
        enemy.update_with_terrain(target, Vec2::new(32.0, 48.0), DT, bounds, &terrain);
        free.update_with_terrain(
            target,
            Vec2::new(32.0, 48.0),
            DT,
            bounds,
            &Terrain::default(),
        );
        lowest_top = lowest_top.min(enemy.position().y - 18.0);
        free_top = free_top.min(free.position().y - 18.0);
    }
    assert!(free_top < 388.0, "control lunge must reach the platform");
    assert!(
        lowest_top >= 388.0 - 0.01,
        "enemy passed through the platform"
    );
}

#[test]
fn enemy_inside_a_solid_is_not_moved_by_the_constraint() {
    let (layout, _) = terrain_room();
    let bounds = layout.world_bounds();
    let terrain = Terrain::from_tuples(&[(400.0, 400.0, 100.0, 60.0)]);
    let mut enemy = lunger(bounds, 450.0);
    let before = enemy.position();
    enemy.update_with_terrain(Vec2::new(450.0, 442.0), Vec2::ZERO, 0.0, bounds, &terrain);
    assert_eq!(enemy.position(), before);
}
