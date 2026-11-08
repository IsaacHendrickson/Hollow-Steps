use hollow_steps::enemy::ENEMY_MAX_HEALTH;
use hollow_steps::game::{GameState, VIEW_HEIGHT, VIEW_WIDTH};
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
        let input = read_player_input();
        game.update(input, delta_seconds);

        draw_scene(&game);
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
            Color::from_rgba(82, 172, 164, 100),
        );
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
        "Move: A/D or arrows   Jump: hold Space/Z   Attack: X   Pick up: Up",
        24.0,
        36.0,
        22.0,
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
        format!("Items collected: {}", player.collected_items().len()),
        24.0,
        64.0,
        22.0,
        WHITE,
    );
    draw_text(
        format!("Room: {:?} (walk into the green exit)", room.id),
        24.0,
        120.0,
        22.0,
        WHITE,
    );
}
