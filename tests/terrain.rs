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
            visual_size: None,
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

// ---- Ceilings: underside of a solid and the room's top bound ----

/// The head of the block in `terrain_layout.ron` is blocked at this y (its underside).
const BLOCK_UNDERSIDE: f32 = 360.0;

fn head(player: &Player) -> f32 {
    player.position().y - player.size().y / 2.0
}

/// After every step the player must overlap no solid and stay under the room's top.
fn assert_clear(player: &Player, terrain: &Terrain, layout: &RoomLayout, context: &str) {
    assert!(
        !terrain.overlaps_body(player.position(), player.size()),
        "{context}: player inside a solid at {:?}",
        player.position()
    );
    assert!(
        head(player) >= layout.world_bounds().top - 0.01,
        "{context}: head above the room top: {}",
        head(player)
    );
}

fn step_clear(player: &mut Player, input: PlayerInput, dt: f32, l: &RoomLayout, t: &Terrain) {
    step(player, input, dt, l, t);
    assert_clear(player, t, l, "step");
}

fn jump_input(held: bool) -> PlayerInput {
    PlayerInput {
        jump_pressed: true,
        jump_held: held,
        ..PlayerInput::default()
    }
}

fn held() -> PlayerInput {
    PlayerInput {
        jump_held: true,
        ..PlayerInput::default()
    }
}

/// Jumps with the key held until the head stops against `ceiling_y`; returns the player on
/// the step it stopped. Panics when it never does (the head went through, or fell short).
fn rise_until_blocked(
    mut player: Player,
    layout: &RoomLayout,
    terrain: &Terrain,
    ceiling_y: f32,
) -> Player {
    for frame in 0..90 {
        // The jump step itself can already hit a ceiling that is only a few pixels away.
        let rising = frame == 0 || player.velocity().y < 0.0;
        let input = if frame == 0 { jump_input(true) } else { held() };
        step_clear(&mut player, input, DT, layout, terrain);
        if rising && player.velocity().y == 0.0 && (head(&player) - ceiling_y).abs() < 0.01 {
            return player;
        }
        assert!(
            head(&player) >= ceiling_y - 0.01,
            "frame {frame}: head passed the ceiling at {ceiling_y}"
        );
    }
    panic!("the player never bumped the ceiling at {ceiling_y}");
}

fn falls_back_to_the_floor(player: &mut Player, layout: &RoomLayout, terrain: &Terrain) {
    let before = player.position().y;
    step_clear(player, held(), DT, layout, terrain);
    assert!(player.velocity().y > 0.0, "still rising after the bump");
    assert!(player.position().y > before, "stuck against the ceiling");
    for _ in 0..120 {
        step_clear(player, held(), DT, layout, terrain);
    }
    assert!(player.is_grounded());
}

#[test]
fn rising_into_a_platform_underside_stops_the_head_then_falls() {
    let (layout, terrain) = terrain_room();
    let player = player_at(&layout, Vec2::new(150.0, 436.0), &terrain);
    let mut bumped = rise_until_blocked(player, &layout, &terrain, BLOCK_UNDERSIDE);
    assert_eq!(bumped.velocity().y, 0.0);
    falls_back_to_the_floor(&mut bumped, &layout, &terrain);
}

#[test]
fn rising_into_the_room_top_stops_the_head_then_falls() {
    let layout = layout("ceiling_layout.ron");
    layout.validate_geometry().unwrap();
    let terrain = Terrain::from_tuples(&layout.terrain);
    // Standing on the platform puts the head two pixels under the room top (y=100).
    let player = player_at(&layout, Vec2::new(360.0, 100.0), &terrain);
    assert!((head(&player) - 102.0).abs() < 0.01);
    let mut bumped = rise_until_blocked(player, &layout, &terrain, 100.0);
    assert_eq!(bumped.velocity().y, 0.0);
    // Falls back onto the platform it left.
    step_clear(&mut bumped, held(), DT, &layout, &terrain);
    assert!(bumped.velocity().y > 0.0);
    for _ in 0..120 {
        step_clear(&mut bumped, held(), DT, &layout, &terrain);
    }
    assert!(bumped.is_grounded());
    assert!((bumped.position().y - 126.0).abs() < 0.01);
}

#[test]
fn rising_into_the_top_of_a_room_with_no_solids_stops_the_head() {
    let layout = layout("low_room_layout.ron");
    layout.validate_geometry().unwrap();
    let terrain = Terrain::from_tuples(&layout.terrain);
    assert!(terrain.is_empty());
    let player = player_at(&layout, Vec2::new(300.0, 436.0), &terrain);
    let mut bumped = rise_until_blocked(player, &layout, &terrain, 400.0);
    assert_eq!(bumped.velocity().y, 0.0);
    falls_back_to_the_floor(&mut bumped, &layout, &terrain);
}

#[test]
fn moving_sideways_while_rising_never_enters_an_edge_or_corner() {
    let (layout, terrain) = terrain_room();
    // Start positions sweep across the block's left and right edges (x 100 and 200),
    // the platform (x 300..420) and the wall (x 600..620).
    for dt in [DT, 1.0 / 30.0, 1.0 / 120.0] {
        for axis in [-1.0, 1.0] {
            let mut start_x = 40.0;
            while start_x < 680.0 {
                let spot = Vec2::new(start_x, 436.0);
                if terrain.overlaps_body(spot, Vec2::new(32.0, 48.0)) {
                    start_x += 5.0;
                    continue;
                }
                let mut player = player_at(&layout, spot, &Terrain::default());
                step_clear(&mut player, jump_input(true), dt, &layout, &terrain);
                for frame in 0..(1.2 / dt) as u32 {
                    let input = PlayerInput {
                        move_axis: axis,
                        jump_held: frame < 40,
                        ..PlayerInput::default()
                    };
                    step_clear(&mut player, input, dt, &layout, &terrain);
                }
                start_x += 5.0;
            }
        }
    }
}

#[test]
fn a_ceiling_corner_does_not_trap_the_player() {
    let (layout, terrain) = terrain_room();
    // Rise just left of the block's left edge (x 100), then walk under it with the key held.
    let mut player = player_at(&layout, Vec2::new(80.0, 436.0), &terrain);
    step_clear(&mut player, jump_input(true), DT, &layout, &terrain);
    for _ in 0..30 {
        let input = PlayerInput {
            move_axis: 1.0,
            jump_held: true,
            ..PlayerInput::default()
        };
        step_clear(&mut player, input, DT, &layout, &terrain);
    }
    for _ in 0..120 {
        step_clear(&mut player, walk(1.0), DT, &layout, &terrain);
    }
    assert!(player.is_grounded());
    assert!(player.position().x > 200.0, "stuck at the corner");
}

#[test]
fn holding_jump_at_a_ceiling_does_not_stick_or_float() {
    let (layout, terrain) = terrain_room();
    let player = player_at(&layout, Vec2::new(150.0, 436.0), &terrain);
    let mut bumped = rise_until_blocked(player, &layout, &terrain, BLOCK_UNDERSIDE);
    let mut last = bumped.position().y;
    for frame in 0..8 {
        step_clear(&mut bumped, held(), DT, &layout, &terrain);
        assert!(
            bumped.position().y > last,
            "frame {frame}: no longer falling"
        );
        assert!(bumped.velocity().y > 0.0);
        last = bumped.position().y;
    }
    // A fresh press of the key while falling must not start a mid-air jump.
    step_clear(&mut bumped, jump_input(true), DT, &layout, &terrain);
    assert!(bumped.velocity().y > 0.0);
}

#[test]
fn coyote_time_and_jump_buffer_are_correct_after_a_head_bump() {
    let (layout, terrain) = terrain_room();
    let player = player_at(&layout, Vec2::new(150.0, 436.0), &terrain);
    // Hitting a ceiling grants no coyote time: a press right at the bump does nothing.
    let mut bumped = rise_until_blocked(player.clone(), &layout, &terrain, BLOCK_UNDERSIDE);
    step_clear(&mut bumped, jump_input(false), DT, &layout, &terrain);
    assert!(bumped.velocity().y > 0.0, "jumped in mid-air after a bump");
    // The press is long gone (0.12 s buffer) by the time the 52 px fall ends.
    for _ in 0..120 {
        step_clear(&mut bumped, PlayerInput::default(), DT, &layout, &terrain);
    }
    assert!(bumped.is_grounded());
    assert_eq!(bumped.velocity().y, 0.0, "stale buffer fired on landing");
    // The buffer still works for a press just before landing.
    let mut again = rise_until_blocked(player, &layout, &terrain, BLOCK_UNDERSIDE);
    while again.position().y + 24.0 < 460.0 - 8.0 {
        step_clear(&mut again, PlayerInput::default(), DT, &layout, &terrain);
    }
    step_clear(&mut again, jump_input(false), DT, &layout, &terrain);
    for _ in 0..12 {
        step_clear(&mut again, PlayerInput::default(), DT, &layout, &terrain);
        if again.velocity().y < 0.0 {
            return;
        }
    }
    panic!("buffered jump did not fire on landing");
}

fn knockback(x: f32, y: f32) -> hollow_steps::enemy_attack::DamageHit {
    hollow_steps::enemy_attack::DamageHit {
        damage: 1,
        knockback: Vec2::new(x, y),
        knockback_secs: 0.16,
    }
}

#[test]
fn knockback_into_a_ceiling_stops_the_head() {
    let (layout, terrain) = terrain_room();
    for (x, y) in [(0.0, -700.0), (160.0, -700.0), (-160.0, -900.0)] {
        let mut player = player_at(&layout, Vec2::new(150.0, 436.0), &terrain);
        assert!(player.take_attack_damage(knockback(x, y)));
        let mut bumped = false;
        for _ in 0..40 {
            let rising = player.velocity().y < 0.0;
            step_clear(&mut player, PlayerInput::default(), DT, &layout, &terrain);
            bumped |= rising && player.velocity().y == 0.0;
            assert!(head(&player) >= BLOCK_UNDERSIDE - 0.01 || x != 0.0);
        }
        assert!(bumped, "knockback {x},{y} never reached the ceiling");
        for _ in 0..120 {
            step_clear(&mut player, PlayerInput::default(), DT, &layout, &terrain);
        }
        assert!(player.is_grounded());
    }
}

#[test]
fn very_large_deltas_cannot_pass_a_ceiling() {
    let (layout, terrain) = terrain_room();
    for dt in [0.1, 0.2, 0.5, 2.0, 5.0] {
        let mut player = player_at(&layout, Vec2::new(150.0, 436.0), &terrain);
        step_clear(&mut player, jump_input(true), dt, &layout, &terrain);
        step_clear(&mut player, held(), dt, &layout, &terrain);
        assert!(head(&player) >= BLOCK_UNDERSIDE - 0.01, "jump at dt {dt}");
    }
    // An enormous upward velocity (knockback) over a large frame, under the block and
    // under the room top.
    for dt in [DT, 0.25, 5.0] {
        let mut player = player_at(&layout, Vec2::new(150.0, 436.0), &terrain);
        player.take_attack_damage(knockback(0.0, -20_000.0));
        step_clear(&mut player, PlayerInput::default(), dt, &layout, &terrain);
        assert!(
            head(&player) >= BLOCK_UNDERSIDE - 0.01,
            "knockback at dt {dt}"
        );
        let mut player = player_at(&layout, Vec2::new(700.0, 436.0), &terrain);
        player.take_attack_damage(knockback(0.0, -20_000.0));
        step_clear(&mut player, PlayerInput::default(), dt, &layout, &terrain);
        assert!(head(&player) >= 0.0 - 0.01, "knockback at dt {dt}");
    }
    // The room top as well, from a platform near it.
    let layout = crate::layout("ceiling_layout.ron");
    let terrain = Terrain::from_tuples(&layout.terrain);
    for dt in [0.1, 0.25, 0.5] {
        let mut player = player_at(&layout, Vec2::new(360.0, 100.0), &terrain);
        step_clear(&mut player, jump_input(true), dt, &layout, &terrain);
        step_clear(&mut player, held(), dt, &layout, &terrain);
        assert!(head(&player) >= 100.0 - 0.01, "room top at dt {dt}");
    }
}

#[test]
fn a_head_stopped_at_the_room_top_still_reaches_a_top_exit() {
    use hollow_steps::room::{Exit, RoomEdge, RoomId};
    use macroquad::math::Rect;
    let layout = layout("ceiling_layout.ron");
    let terrain = Terrain::from_tuples(&layout.terrain);
    let exit = Exit {
        edge: RoomEdge::Top,
        offset: 300.0,
        length: 120.0,
        target_room: RoomId::Second,
        spawn: Vec2::new(100.0, 436.0),
    };
    let player = player_at(&layout, Vec2::new(360.0, 100.0), &terrain);
    let bumped = rise_until_blocked(player, &layout, &terrain, 100.0);
    let (p, size) = (bumped.position(), bumped.size());
    let body = Rect::new(p.x - size.x / 2.0, p.y - size.y / 2.0, size.x, size.y);
    assert!(body.overlaps(&exit.zone(layout.rect())), "exit unreachable");
}
