use hollow_steps::coin::COIN_RADIUS;
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
        "Move: A/D or arrows    Jump: Space/Z    Pick up: Up    Dash: Shift    Inventory: I",
        24.0,
        36.0,
        24.0,
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
        format!("Room: {:?} (walk into the green exit)", room.id),
        24.0,
        120.0,
        22.0,
        WHITE,
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
