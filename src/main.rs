use hollow_steps::boss::{Boss, BossState};
use hollow_steps::boss_special::{
    BossSpecials, SpecialAttack, BOSS_MAX_HEALTH, RING_RADIUS, SLAM_RADIUS,
};
use hollow_steps::coin::COIN_RADIUS;
use hollow_steps::enemy::ENEMY_MAX_HEALTH;
use hollow_steps::game::{GameState, SPECIAL_BOSS_SIZE, VIEW_HEIGHT, VIEW_WIDTH};
use hollow_steps::health::MAX_HEALTH;
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::player::PlayerInput;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "Hollow Steps".to_owned(),
        window_width: VIEW_WIDTH as i32,
        window_height: VIEW_HEIGHT as i32,
        window_resizable: false,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = GameState::new();

    loop {
        let delta_seconds = get_frame_time().min(1.0 / 15.0);
        if is_key_pressed(KeyCode::I) || (game.inventory_open() && is_key_pressed(KeyCode::Escape))
        {
            game.toggle_inventory();
        }
        let input = read_player_input();
        game.update(input, delta_seconds);

        draw_scene(&game);
        if game.inventory_open() {
            draw_inventory(&game);
        }
        next_frame().await;
    }
}

fn read_player_input() -> PlayerInput {
    let mut move_axis = 0.0;
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        move_axis += 1.0;
    }
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        move_axis -= 1.0;
    }

    PlayerInput {
        move_axis,
        jump_pressed: is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Z),
        pickup_pressed: is_key_pressed(KeyCode::Up),
        jump_held: is_key_down(KeyCode::Space) || is_key_down(KeyCode::Z),
        attack_pressed: is_key_pressed(KeyCode::X),
        dash_pressed: is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift),
    }
}

fn draw_scene(game: &GameState) {
    let room = game.room();
    let bounds = game.world_bounds();
    let player = game.player();
    let player_position = player.position();
    let player_size = player.size();

    clear_background(Color::from_rgba(16, 18, 31, 255));
    draw_rectangle(
        bounds.left,
        bounds.floor_y,
        bounds.right - bounds.left,
        VIEW_HEIGHT - bounds.floor_y,
        Color::from_rgba(34, 37, 58, 255),
    );
    draw_line(
        bounds.left,
        bounds.floor_y,
        bounds.right,
        bounds.floor_y,
        4.0,
        Color::from_rgba(111, 122, 164, 255),
    );
    draw_rectangle_lines(
        room.bounds.x,
        room.bounds.y,
        room.bounds.w,
        room.bounds.h,
        2.0,
        Color::from_rgba(62, 70, 103, 255),
    );
    for exit in &room.exits {
        let zone = exit.zone(room.bounds);
        draw_rectangle(
            zone.x,
            zone.y,
            zone.w,
            zone.h,
            if room.exits_locked() {
                Color::from_rgba(220, 65, 70, 190)
            } else {
                Color::from_rgba(82, 172, 164, 100)
            },
        );
        if room.exits_locked() {
            draw_rectangle_lines(zone.x, zone.y, zone.w, zone.h, 3.0, RED);
        }
    }
    for item in game.items() {
        let nearby = item.can_pick_up(player);
        let top_left = item.position - ITEM_SIZE / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            ITEM_SIZE.x,
            ITEM_SIZE.y,
            if nearby { YELLOW } else { GOLD },
        );
        if nearby {
            draw_rectangle_lines(
                top_left.x - 3.0,
                top_left.y - 3.0,
                ITEM_SIZE.x + 6.0,
                ITEM_SIZE.y + 6.0,
                2.0,
                WHITE,
            );
            draw_text(
                format!("Up: pick up {}", item.item.name()),
                top_left.x - 80.0,
                top_left.y - 16.0,
                20.0,
                WHITE,
            );
        }
    }
    for coin in game.coins() {
        draw_circle(coin.position.x, coin.position.y, COIN_RADIUS, GOLD);
        draw_circle_lines(
            coin.position.x,
            coin.position.y,
            COIN_RADIUS - 2.0,
            1.0,
            YELLOW,
        );
    }
    for enemy in &game.room().enemies {
        let top_left = enemy.position() - enemy.size() / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            enemy.size().x,
            enemy.size().y,
            MAROON,
        );
        draw_rectangle(top_left.x, top_left.y - 8.0, enemy.size().x, 4.0, DARKGRAY);
        draw_rectangle(
            top_left.x,
            top_left.y - 8.0,
            enemy.size().x * f32::from(enemy.health()) / f32::from(ENEMY_MAX_HEALTH),
            4.0,
            RED,
        );
    }
    if let Some(boss) = &room.boss {
        draw_boss(boss);
    }
    if let Some(boss) = &room.boss_special {
        draw_boss_special(boss, bounds.floor_y);
    }
    if let Some(hitbox) = player.attack().hitbox(player_position, player_size) {
        draw_rectangle(
            hitbox.x,
            hitbox.y,
            hitbox.w,
            hitbox.h,
            Color::from_rgba(255, 231, 160, 150),
        );
        draw_rectangle_lines(hitbox.x, hitbox.y, hitbox.w, hitbox.h, 2.0, GOLD);
    }
    draw_rectangle(
        player_position.x - player_size.x / 2.0,
        player_position.y - player_size.y / 2.0,
        player_size.x,
        player_size.y,
        if player.health().flash_on() {
            WHITE
        } else {
            Color::from_rgba(110, 160, 230, 255)
        },
    );
    draw_text(
        "A/D: move   Space/Z: hold jump   X: attack   Up: pickup   Shift: dash   I: inventory",
        24.0,
        36.0,
        21.0,
        Color::from_rgba(220, 231, 255, 255),
    );
    draw_text(
        format!("Health: {}/{}", player.health().current(), MAX_HEALTH),
        24.0,
        92.0,
        22.0,
        WHITE,
    );
    draw_text(
        format!(
            "Coins: {}    Items: {}",
            game.coin_count(),
            player.collected_items().len()
        ),
        24.0,
        64.0,
        22.0,
        WHITE,
    );
    draw_text(
        format!(
            "Room: {:?} ({})",
            room.id,
            if room.exits_locked() {
                "exits locked until the boss falls"
            } else {
                "walk into a green exit"
            }
        ),
        24.0,
        120.0,
        22.0,
        WHITE,
    );
}

fn draw_boss(boss: &Boss) {
    if let Some(area) = boss.attack_area() {
        let active = boss.state() == BossState::Attacking;
        draw_rectangle(
            area.x,
            area.y,
            area.w,
            area.h,
            if active {
                Color::from_rgba(240, 65, 70, 160)
            } else {
                Color::from_rgba(250, 200, 70, 60)
            },
        );
        draw_rectangle_lines(
            area.x,
            area.y,
            area.w,
            area.h,
            3.0,
            if active { RED } else { GOLD },
        );
    }
    let top_left = boss.position() - boss.size() / 2.0;
    let color = match boss.state() {
        BossState::Idle => PURPLE,
        BossState::Telegraphing => ORANGE,
        BossState::Attacking => MAROON,
        BossState::Dead => DARKGRAY,
    };
    draw_rectangle(top_left.x, top_left.y, boss.size().x, boss.size().y, color);
    draw_boss_health(u32::from(boss.health()), u32::from(boss.max_health()));
    let cue = match boss.state() {
        BossState::Idle => "Recovering",
        BossState::Telegraphing => boss.current_attack().name,
        BossState::Attacking => "Attack!",
        BossState::Dead => "Defeated - exits open",
    };
    draw_text(cue, 300.0, 206.0, 22.0, WHITE);
}

fn draw_boss_special(boss: &BossSpecials, floor_y: f32) {
    if !boss.is_defeated() {
        if boss.active_attack() == Some(SpecialAttack::Slam) {
            draw_rectangle(
                boss.position().x - SLAM_RADIUS,
                floor_y - 12.0,
                SLAM_RADIUS * 2.0,
                12.0,
                RED,
            );
        }
        for ring in boss.rings() {
            let color = if ring.exploded() { ORANGE } else { RED };
            draw_circle_lines(
                ring.position().x,
                ring.position().y,
                RING_RADIUS,
                3.0,
                color,
            );
        }
        let top_left = boss.position() - SPECIAL_BOSS_SIZE / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            SPECIAL_BOSS_SIZE.x,
            SPECIAL_BOSS_SIZE.y,
            if boss.is_telegraphing() {
                ORANGE
            } else {
                PURPLE
            },
        );
    } else if boss.is_death_effect_active() {
        draw_circle(
            boss.position().x,
            boss.position().y,
            SPECIAL_BOSS_SIZE.x,
            WHITE,
        );
    }
    draw_boss_health(boss.health(), BOSS_MAX_HEALTH);
    let cue = match boss.active_attack() {
        Some(SpecialAttack::Slam) => "Slam - move out of the red zone!",
        Some(SpecialAttack::Summon) => "Summoning enemies",
        Some(SpecialAttack::Rings) => "Rings - avoid the red circles!",
        None if boss.is_defeated() => "Defeated - collect the coins!",
        None => "Recovering",
    };
    draw_text(cue, 300.0, 206.0, 22.0, WHITE);
}

fn draw_boss_health(health: u32, max_health: u32) {
    draw_text(
        format!("Ash Warden: {health}/{max_health}"),
        300.0,
        156.0,
        26.0,
        WHITE,
    );
    draw_rectangle(300.0, 166.0, 360.0, 12.0, DARKGRAY);
    draw_rectangle(
        300.0,
        166.0,
        360.0 * health as f32 / max_health as f32,
        12.0,
        RED,
    );
}

fn draw_inventory(game: &GameState) {
    draw_rectangle(
        0.0,
        0.0,
        VIEW_WIDTH,
        VIEW_HEIGHT,
        Color::from_rgba(0, 0, 0, 180),
    );
    draw_rectangle(
        200.0,
        130.0,
        560.0,
        310.0,
        Color::from_rgba(28, 32, 50, 255),
    );
    draw_rectangle_lines(200.0, 130.0, 560.0, 310.0, 2.0, GRAY);
    draw_text("Inventory", 232.0, 180.0, 36.0, WHITE);
    draw_text(
        format!("Coins: {}", game.coin_count()),
        232.0,
        224.0,
        26.0,
        GOLD,
    );
    draw_text("Collected items", 232.0, 270.0, 24.0, WHITE);
    let items = game.player().collected_items();
    if items.is_empty() {
        draw_text("No items collected yet", 232.0, 306.0, 22.0, LIGHTGRAY);
    } else {
        for (index, item) in items.iter().enumerate() {
            draw_text(item.name(), 232.0, 306.0 + index as f32 * 28.0, 22.0, GOLD);
        }
    }
    draw_text(
        "Paused  |  I or Esc to return",
        232.0,
        408.0,
        22.0,
        LIGHTGRAY,
    );
}
