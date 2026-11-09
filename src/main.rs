use hollow_steps::boss::{Boss, BossState};
use hollow_steps::boss_special::{
    BossSpecials, SpecialAttack, BOSS_MAX_HEALTH, RING_RADIUS, SLAM_RADIUS,
};
use hollow_steps::coin::COIN_RADIUS;
use hollow_steps::editor::{EditorSession, Placement, RoomLayout, Selection};
use hollow_steps::game::{GameState, SPECIAL_BOSS_SIZE, VIEW_HEIGHT, VIEW_WIDTH};
use hollow_steps::health::MAX_HEALTH;
use hollow_steps::item::Item;
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomEdge;
use macroquad::prelude::*;
use macroquad::ui::{hash, root_ui, widgets};

fn window_conf() -> Conf {
    Conf {
        window_title: "Hollow Steps".to_owned(),
        window_width: VIEW_WIDTH as i32,
        window_height: VIEW_HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = GameState::new();
    let mut reload_result = None;
    let mut reload_scroll = 0;
    let mut editor_ui = EditorUi::default();

    loop {
        let delta_seconds = get_frame_time().min(1.0 / 15.0);
        if is_key_pressed(KeyCode::F2) {
            if let Err(error) = game.toggle_editor() {
                reload_result = Some(Err(error));
            }
            editor_ui = EditorUi::default();
        }
        if game.editor().is_some() {
            draw_editor(&mut game, &mut editor_ui);
            next_frame().await;
            continue;
        }
        if is_key_pressed(KeyCode::F5) {
            reload_result = Some(game.reload_rooms());
            reload_scroll = 0;
        }
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
        if let Some(result) = &reload_result {
            draw_reload_result(result, &mut reload_scroll);
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
            enemy.size().x * f32::from(enemy.health()) / f32::from(enemy.max_health()),
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
    draw_text("F5: reload room files", 24.0, 148.0, 18.0, LIGHTGRAY);
}

fn draw_reload_result(result: &Result<(), String>, scroll: &mut usize) {
    let Err(error) = result else {
        draw_rectangle(16.0, VIEW_HEIGHT - 48.0, VIEW_WIDTH - 32.0, 36.0, BLACK);
        draw_text("Rooms reloaded", 28.0, VIEW_HEIGHT - 24.0, 22.0, GREEN);
        return;
    };

    const VISIBLE_LINES: usize = 8;
    const FONT_SIZE: u16 = 18;
    let mut lines = Vec::new();
    for paragraph in error.lines() {
        let mut line = String::new();
        for character in paragraph.chars() {
            line.push(character);
            if measure_text(&line, None, FONT_SIZE, 1.0).width > VIEW_WIDTH - 72.0 {
                line.pop();
                lines.push(std::mem::take(&mut line));
                line.push(character);
            }
        }
        lines.push(line);
    }
    let max_scroll = lines.len().saturating_sub(VISIBLE_LINES);
    *scroll = (*scroll as f32 - mouse_wheel().1).clamp(0.0, max_scroll as f32) as usize;
    let top = VIEW_HEIGHT - 260.0;
    draw_rectangle(
        16.0,
        top,
        VIEW_WIDTH - 32.0,
        248.0,
        Color::from_rgba(28, 12, 18, 245),
    );
    draw_text(
        "Room reload failed - F5 to retry",
        28.0,
        top + 28.0,
        22.0,
        RED,
    );
    for (index, line) in lines.iter().skip(*scroll).take(VISIBLE_LINES).enumerate() {
        draw_text(
            line,
            28.0,
            top + 56.0 + index as f32 * 22.0,
            FONT_SIZE as f32,
            WHITE,
        );
    }
    let hint = if max_scroll > 0 {
        format!(
            "Mouse wheel: scroll errors ({}/{})",
            *scroll + 1,
            max_scroll + 1
        )
    } else {
        "Fix the files, then press F5 to retry".to_owned()
    };
    draw_text(&hint, 28.0, top + 232.0, 18.0, LIGHTGRAY);
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

#[derive(Default)]
struct EditorUi {
    definition: usize,
    room: usize,
    target: usize,
    edge: usize,
    placing: bool,
    selected: Option<Selection>,
    dragging: Option<(Selection, Vec2)>,
    destination: Option<usize>,
    properties: bool,
    scroll: f32,
    content_height: f32,
    operation_name: String,
    entity_choice: usize,
    behavior_choice: usize,
    property: PropertyUi,
}

impl EditorUi {
    fn clear_hidden_property_error(&mut self, enemy_count: usize) {
        if !self.properties
            || !matches!(self.selected, Some(Selection::Enemy(i)) if i < enemy_count)
        {
            self.property.error = None;
        }
    }
}

// All canvas transforms, hit testing, mouse translation, and drawing are front-end only.
struct EditorCanvas {
    origin: Vec2,
    scale: f32,
    bounds: Rect,
}
impl EditorCanvas {
    fn new(layout: &RoomLayout) -> Self {
        let bounds = layout.rect();
        let panel =
            hollow_steps::definition_editor::PanelLayout::new(screen_width(), screen_height());
        let scale = ((panel.panel.x - 40.0).max(1.0) / bounds.w)
            .min((screen_height() - 180.0).max(1.0) / bounds.h);
        Self {
            origin: vec2(20.0, 90.0),
            scale,
            bounds,
        }
    }
    fn screen(&self, point: Vec2) -> Vec2 {
        self.origin + (point - self.bounds.point()) * self.scale
    }
    fn world(&self, point: Vec2) -> Vec2 {
        (point - self.origin) / self.scale + self.bounds.point()
    }
    fn rect(&self, rect: Rect) -> Rect {
        let p = self.screen(rect.point());
        Rect::new(p.x, p.y, rect.w * self.scale, rect.h * self.scale)
    }
}

fn editor_markers(
    editor: &EditorSession,
    destination: Option<usize>,
    selected: Option<Selection>,
) -> Vec<(Selection, Rect, Color)> {
    let body =
        |p: Vec2, size: Vec2| Rect::new(p.x - size.x / 2.0, p.y - size.y / 2.0, size.x, size.y);
    if let Some(i) = destination {
        return editor
            .draft
            .layout
            .exits
            .get(i)
            .map(|exit| {
                vec![(
                    Selection::Destination(i),
                    body(exit.spawn.into(), vec2(32.0, 48.0)),
                    PURPLE,
                )]
            })
            .unwrap_or_default();
    }
    let mut markers = Vec::new();
    for (i, enemy) in editor.draft.enemies.iter().enumerate() {
        if let Ok(size) = editor.definitions.size(&enemy.definition) {
            markers.push((
                Selection::Enemy(i),
                body(
                    vec2(
                        enemy.patrol_left,
                        editor.draft.layout.floor_y - size.y / 2.0,
                    ),
                    size,
                ),
                RED,
            ));
        }
    }
    for (i, item) in editor.draft.layout.items.iter().enumerate() {
        markers.push((
            Selection::Item(i),
            body(item.position.into(), ITEM_SIZE),
            GOLD,
        ));
    }
    for (i, exit) in editor.draft.layout.exits.iter().enumerate() {
        markers.push((
            Selection::Exit(i),
            exit.exit().zone(editor.draft.layout.rect()),
            GREEN,
        ));
    }
    markers.push((
        Selection::Spawn,
        body(editor.draft.layout.spawn.into(), vec2(32.0, 48.0)),
        SKYBLUE,
    ));
    if let Some(Selection::Enemy(i) | Selection::PatrolLeft(i) | Selection::PatrolRight(i)) =
        selected
    {
        if let Some(enemy) = editor.draft.enemies.get(i) {
            if let Ok(size) = editor.definitions.size(&enemy.definition) {
                let y = editor.draft.layout.floor_y - size.y - 14.0;
                for (selection, x, handle_y) in [
                    (Selection::PatrolLeft(i), enemy.patrol_left, y),
                    (Selection::PatrolRight(i), enemy.patrol_right, y - 24.0),
                ] {
                    markers.push((selection, body(vec2(x, handle_y), vec2(14.0, 20.0)), ORANGE));
                }
            }
        }
    }
    markers
}

fn draw_editor(game: &mut GameState, state: &mut EditorUi) {
    clear_background(Color::from_rgba(16, 18, 31, 255));
    let mut save = false;
    let mut open = None;
    let editor = game.editor_mut().expect("editor open");
    if let Ok(definitions) = editor.content.placement_definitions() {
        editor.definitions = definitions;
    }
    let definitions = editor
        .content
        .entities
        .values()
        .map(|entity| format!("Enemy: {}", entity.name))
        .chain(
            Item::ALL
                .iter()
                .map(|item| format!("Item: {}", item.name())),
        )
        .collect::<Vec<_>>();
    let names = definitions.iter().map(String::as_str).collect::<Vec<_>>();
    let room_names = editor
        .rooms
        .iter()
        .map(|(id, _)| format!("{id:?}"))
        .collect::<Vec<_>>();
    let room_labels = room_names.iter().map(String::as_str).collect::<Vec<_>>();
    state.definition = state.definition.min(names.len().saturating_sub(1));
    state.room = state.room.min(room_labels.len().saturating_sub(1));
    state.target = state.target.min(room_labels.len().saturating_sub(1));
    let panel = hollow_steps::definition_editor::PanelLayout::new(screen_width(), screen_height());
    if panel.body.contains(Vec2::from(mouse_position())) {
        state.scroll =
            panel.scroll_bounds(state.content_height, state.scroll - mouse_wheel().1 * 32.0);
    } else {
        state.scroll = panel.scroll_bounds(state.content_height, state.scroll);
    }
    widgets::Window::new(hash!("room editor"), panel.body.point(), panel.body.size())
        .titlebar(false)
        .movable(false)
        .ui(&mut root_ui(), |ui| {
            // Set the native window scroll before laying out controls. Save lives in a separate window.
            let margin = ui.default_skin().margin;
            // The default window skin has equal top/bottom borders. Exclude them from the scroll viewport.
            let border = (ui.canvas().cursor().y - ui.scroll().y - panel.body.y - margin).max(0.0);
            let viewport_height = (panel.body.h - 2.0 * border).max(1.0);
            ui.scroll_here_ratio((margin - state.scroll) / viewport_height);
            (|| {
                panel_label(ui, "Room editor (F2)");
                for warning in editor.draft.warnings() {
                    panel_label(ui, warning);
                }
                if !editor.message.is_empty() {
                    panel_label(ui, &editor.message);
                }
                panel_label(
                    ui,
                    &format!("{:?}{}", editor.id, if editor.dirty { " *" } else { "" }),
                );
                if panel_button(
                    ui,
                    if state.properties {
                        "Room tools"
                    } else {
                        "Enemy properties"
                    },
                ) {
                    state.properties = !state.properties;
                    if state.properties {
                        state.placing = false;
                        state.destination = None;
                        state.dragging = None;
                    }
                }
                if state.properties {
                    draw_definition_panel(ui, editor, state);
                    return;
                }
                if panel_button(ui, "New room") {
                    editor.new_room();
                    *state = EditorUi::default();
                }
                editor_list(ui, hash!("rooms"), "Rooms", &room_labels, &mut state.room);
                if panel_button(ui, "Open room") {
                    open = editor.rooms.get(state.room).map(|(id, _)| *id);
                }
                ui.separator();
                editor_list(
                    ui,
                    hash!("definitions"),
                    "Entity",
                    &names,
                    &mut state.definition,
                );
                if panel_button(ui, if state.placing { "[Place]" } else { "Place" }) {
                    state.placing = true;
                    state.destination = None;
                    state.selected = None;
                }
                if panel_button(
                    ui,
                    if !state.placing {
                        "[Select / drag]"
                    } else {
                        "Select / drag"
                    },
                ) {
                    state.placing = false;
                }
                if panel_button(ui, "Delete selected") {
                    if let Some(selection) = state.selected {
                        match editor.draft.delete(selection) {
                            Ok(()) => {
                                state.selected = None;
                                state.destination = None;
                                editor.dirty = true;
                            }
                            Err(error) => editor.message = error,
                        }
                    }
                }
                ui.separator();
                ui.combo_box(
                    hash!("edge"),
                    "Edge",
                    &["Left", "Right", "Top", "Bottom"],
                    &mut state.edge,
                );
                editor_list(
                    ui,
                    hash!("target"),
                    "Exit target",
                    &room_labels,
                    &mut state.target,
                );
                if panel_button(ui, "Add exit") {
                    if let Some((id, layout)) = editor.rooms.get(state.target) {
                        state.selected = Some(editor.draft.add_exit(
                            [
                                RoomEdge::Left,
                                RoomEdge::Right,
                                RoomEdge::Top,
                                RoomEdge::Bottom,
                            ][state.edge],
                            *id,
                            layout.spawn.into(),
                        ));
                        editor.dirty = true;
                        state.placing = false;
                        state.destination = None;
                    }
                }
                if let Some(Selection::Exit(i) | Selection::Destination(i)) = state.selected {
                    if panel_button(ui, "Set exit target") {
                        if let Some((id, layout)) = editor.rooms.get(state.target) {
                            if let Err(error) =
                                editor.draft.set_exit_target(i, *id, layout.spawn.into())
                            {
                                editor.message = error;
                            } else {
                                editor.dirty = true;
                            }
                        }
                    }
                    if panel_button(ui, "Arrival spawn") {
                        state.destination = Some(i);
                        state.selected = Some(Selection::Destination(i));
                        state.placing = false;
                    }
                }
                if state.destination.is_some() && panel_button(ui, "Back to room") {
                    state.destination = None;
                    state.selected = None;
                }
                ui.separator();
                panel_label(ui, "Blue: room spawn");
                panel_label(ui, "Green: exit span");
                panel_label(ui, "Purple: arrival spawn");
            })();
            state.content_height = panel.body.h + ui.scroll_max().y.max(0.0);
            state.scroll = panel.scroll_bounds(state.content_height, state.scroll);
        });

    widgets::Window::new(
        hash!("editor save strip"),
        panel.save.point(),
        panel.save.size(),
    )
    .titlebar(false)
    .movable(false)
    .ui(&mut root_ui(), |ui| {
        if panel_button(ui, "Save") {
            save = true;
        }
    });

    let layout = state
        .destination
        .and_then(|i| editor.draft.layout.exits.get(i))
        .and_then(|exit| {
            editor
                .catalog()
                .into_iter()
                .find(|(id, _)| *id == exit.target_room)
                .map(|(_, layout)| layout)
        })
        .unwrap_or_else(|| editor.draft.layout.clone());
    let canvas = EditorCanvas::new(&layout);
    let area = canvas.rect(layout.rect());
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, LIGHTGRAY);
    if state.destination.is_some() {
        for exit in &layout.exits {
            let r = canvas.rect(exit.exit().zone(layout.rect()));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, GREEN);
        }
    }
    let floor = canvas.screen(vec2(layout.bounds.0, layout.floor_y));
    draw_line(floor.x, floor.y, area.right(), floor.y, 3.0, GRAY);
    draw_text(
        format!(
            "{:?} - {}",
            editor.id,
            if state.destination.is_some() {
                "destination room view"
            } else {
                "authored room"
            }
        ),
        20.0,
        30.0,
        24.0,
        WHITE,
    );
    draw_text(
        "F2: close (discard unsaved) | Drag: move | Right click / Del: delete",
        20.0,
        58.0,
        17.0,
        LIGHTGRAY,
    );
    let markers = editor_markers(editor, state.destination, state.selected);
    let mouse = Vec2::from(mouse_position());
    let world = canvas.world(mouse);
    let over_canvas =
        area.contains(mouse) && mouse.x < panel.panel.x && !root_ui().is_mouse_over(mouse);
    if is_mouse_button_pressed(MouseButton::Left) && over_canvas {
        if state.placing && state.destination.is_none() {
            let enemy_count = names.len() - Item::ALL.len();
            let placement = if state.definition < enemy_count {
                Placement::Enemy(
                    editor
                        .content
                        .entities
                        .values()
                        .nth(state.definition)
                        .expect("definition index")
                        .name
                        .clone(),
                )
            } else {
                Placement::Item(Item::ALL[state.definition - enemy_count])
            };
            match editor.draft.place(placement, world, &editor.definitions) {
                Ok(selection) => {
                    state.selected = Some(selection);
                    editor.dirty = true;
                    editor.message.clear();
                }
                Err(error) => editor.message = error,
            }
        } else {
            let hit = markers.iter().rev().find(|(_, rect, _)| {
                let r = canvas.rect(*rect);
                Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0).contains(mouse)
            });
            state.selected = hit.map(|(selection, _, _)| match *selection {
                Selection::PatrolLeft(i) | Selection::PatrolRight(i) => Selection::Enemy(i),
                other => other,
            });
            state.dragging = hit.map(|(selection, rect, _)| (*selection, world - rect.center()));
        }
    }
    if is_mouse_button_down(MouseButton::Left) && over_canvas {
        if let Some((selection, offset)) = state.dragging {
            match editor
                .draft
                .move_to(selection, world - offset, &editor.definitions)
            {
                Ok(()) => {
                    editor.dirty = true;
                    editor.message.clear();
                }
                Err(error) => editor.message = error,
            }
        }
    }
    if is_mouse_button_released(MouseButton::Left) {
        state.dragging = None;
    }
    if (is_mouse_button_pressed(MouseButton::Right) && over_canvas)
        || (is_key_pressed(KeyCode::Delete) && !state.properties)
    {
        let selection = if is_mouse_button_pressed(MouseButton::Right) {
            markers
                .iter()
                .rev()
                .find(|(_, rect, _)| rect.contains(world))
                .map(|(selection, _, _)| *selection)
        } else {
            state.selected
        };
        if let Some(selection) = selection {
            match editor.draft.delete(selection) {
                Ok(()) => {
                    state.selected = None;
                    state.dragging = None;
                    state.destination = None;
                    editor.dirty = true;
                }
                Err(error) => editor.message = error,
            }
        }
    }
    for (selection, rect, color) in editor_markers(editor, state.destination, state.selected) {
        let r = canvas.rect(rect);
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            Color::new(color.r, color.g, color.b, 0.45),
        );
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            if state.selected == Some(selection) {
                3.0
            } else {
                1.0
            },
            if state.selected == Some(selection) {
                WHITE
            } else {
                color
            },
        );
    }
    // Wrap status text in the canvas footer so long filesystem/validation errors stay readable.
    let mut line = String::new();
    let mut y = 475.0;
    for word in editor.message.split_whitespace() {
        if measure_text(format!("{line} {word}"), None, 17, 1.0).width > 675.0 {
            draw_text(&line, 20.0, y, 17.0, ORANGE);
            y += 19.0;
            line.clear();
        }
        line.push_str(word);
        line.push(' ');
    }
    draw_text(&line, 20.0, y, 17.0, ORANGE);
    state.clear_hidden_property_error(editor.draft.enemies.len());
    if save {
        state.dragging = None;
        if let Err(error) = game.save_editor() {
            game.editor_mut().expect("open").message = format!("Save failed: {error}");
        }
    }
    if let Some(id) = open {
        if let Err(error) = game.edit_room(id) {
            game.editor_mut().expect("open").message = error;
        }
        *state = EditorUi::default();
    }
}

fn editor_list(
    ui: &mut macroquad::ui::Ui,
    id: u64,
    label: &str,
    names: &[&str],
    selected: &mut usize,
) {
    ui.label(None, label);
    widgets::Group::new(id, vec2(panel_content_width(), 70.0)).ui(ui, |ui| {
        for (index, name) in names.iter().enumerate() {
            let text = if index == *selected {
                format!("> {name}")
            } else {
                (*name).to_owned()
            };
            let width = (panel_content_width() - 24.0).max(1.0);
            if widgets::Button::new(short_text_width(&text, width))
                .size(vec2(width, 24.0))
                .ui(ui)
            {
                *selected = index;
            }
        }
    });
}

#[derive(Default)]
struct PropertyUi {
    path: Option<std::path::PathBuf>,
    fields: [String; 6],
    behavior: String,
    state: usize,
    loaded_state: Option<(String, usize)>,
    state_name: String,
    actions: String,
    transitions: String,
    new_state: String,
    error: Option<String>,
    structured: hollow_steps::definition_editor::StateInputs,
    add_action: usize,
    add_condition: usize,
    advanced: bool,
}

fn draw_definition_operations(
    ui: &mut macroquad::ui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    let paths = editor.content.entities.keys().cloned().collect::<Vec<_>>();
    let names = paths
        .iter()
        .map(|p| editor.content.entities[p].name.clone())
        .collect::<Vec<_>>();
    panel_combo(
        ui,
        hash!("entity operation choice"),
        "Definitions",
        &names,
        &mut state.entity_choice,
    );
    let behaviors = editor.content.behaviors.keys().cloned().collect::<Vec<_>>();
    panel_combo(
        ui,
        hash!("behavior operation choice"),
        "Behaviors",
        &behaviors,
        &mut state.behavior_choice,
    );
    panel_input(
        ui,
        hash!("definition operation name"),
        "New name",
        &mut state.operation_name,
    );
    let mut result = None;
    if panel_button(ui, "New entity") {
        let entity = hollow_steps::enemy::EnemyDefinition {
            name: state.operation_name.clone(),
            health: 1,
            size: (32.0, 40.0),
            contact_damage: 1,
            behavior: behaviors
                .get(state.behavior_choice)
                .cloned()
                .unwrap_or_default(),
        };
        result = Some(editor.content.add_entity(entity).map(|_| ()));
    }
    if let Some(path) = paths.get(state.entity_choice) {
        if panel_button(ui, "Duplicate entity") {
            result = Some(
                editor
                    .content
                    .duplicate_entity(path, &state.operation_name)
                    .map(|_| ()),
            );
        }
        if panel_button(ui, "Delete entity") {
            result = Some(
                if editor.draft.enemies.iter().any(|e| {
                    e.definition == editor.content.entities[path].name
                        || editor.content.entity_path(&e.definition).as_ref() == Some(path)
                }) {
                    Err("entity is referenced by the current room draft".into())
                } else {
                    editor.content.delete_entity(path)
                },
            );
        }
    }
    if panel_button(ui, "New behavior") {
        let behavior = hollow_steps::behavior::BehaviorDefinition {
            initial_state: "idle".into(),
            states: vec![hollow_steps::behavior::State {
                name: "idle".into(),
                actions: vec![],
                transitions: vec![],
            }],
        };
        result = Some(editor.content.add_behavior(&state.operation_name, behavior));
    }
    if let Some(name) = behaviors.get(state.behavior_choice) {
        if panel_button(ui, "Rename behavior") {
            result = Some(editor.content.rename_behavior(name, &state.operation_name));
        }
        if panel_button(ui, "Delete behavior") {
            result = Some(editor.content.delete_behavior(name));
        }
    }
    if let Some(result) = result {
        match result {
            Ok(()) => {
                editor.dirty = true;
                state.property = PropertyUi::default();
                editor.message = "Definition updated".into();
            }
            Err(e) => editor.message = e,
        }
    }
    ui.separator();
}

fn draw_definition_panel(
    ui: &mut macroquad::ui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    use hollow_steps::definition_editor::{
        add_state, edit_state_lists, remove_state, rename_state, serialize,
    };
    draw_definition_operations(ui, editor, state);
    let Some(Selection::Enemy(index)) = state.selected else {
        state.property.error = None;
        panel_label(ui, "Select an enemy on canvas");
        return;
    };
    let Some(placement) = editor.draft.enemies.get(index) else {
        state.property.error = None;
        return;
    };
    let Some(path) = editor.content.entity_path(&placement.definition) else {
        state.property.error = None;
        return;
    };
    let p = &mut state.property;
    if p.path.as_ref() != Some(&path) {
        let entity = &editor.content.entities[&path];
        *p = PropertyUi {
            path: Some(path.clone()),
            fields: [
                entity.name.clone(),
                entity.health.to_string(),
                entity.size.0.to_string(),
                entity.size.1.to_string(),
                entity.contact_damage.to_string(),
                entity.behavior.clone(),
            ],
            ..PropertyUi::default()
        };
    }
    panel_label(ui, "Shared definition (all rooms)");
    for (i, label) in ["Name", "Health", "Width", "Height", "Contact damage"]
        .iter()
        .enumerate()
    {
        panel_input(ui, hash!("entity field", i), label, &mut p.fields[i]);
        if let Err(e) = hollow_steps::definition_editor::validate_entity_field(i, &p.fields[i]) {
            panel_label(ui, &e);
        }
    }
    let behaviors = editor.content.behaviors.keys().cloned().collect::<Vec<_>>();
    let mut behavior_index = behaviors
        .iter()
        .position(|n| *n == p.fields[5])
        .unwrap_or(0);
    panel_combo(
        ui,
        hash!("entity behavior"),
        "Behavior",
        &behaviors,
        &mut behavior_index,
    );
    if let Some(name) = behaviors.get(behavior_index) {
        p.fields[5] = name.clone();
    }
    let mut error = None;
    let parsed = hollow_steps::definition_editor::entity_from_fields(&p.fields);
    match parsed {
        Ok(entity) => {
            if editor.content.entities[&path] != entity {
                editor.content.entities.insert(path, entity);
                editor.dirty = true;
            }
        }
        Err(e) => error = Some(e),
    }
    if p.behavior != p.fields[5] {
        p.behavior = p.fields[5].clone();
        p.loaded_state = None;
        p.state = 0;
    }
    ui.separator();
    if let Some(behavior) = editor.content.behaviors.get_mut(&p.behavior) {
        let mut names = behavior
            .states
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>();
        p.state = p.state.min(names.len().saturating_sub(1));
        panel_combo(ui, hash!("behavior states"), "State", &names, &mut p.state);
        let key = (p.behavior.clone(), p.state);
        if p.loaded_state.as_ref() != Some(&key) {
            let selected = &behavior.states[p.state];
            p.state_name = selected.name.clone();
            p.structured = hollow_steps::definition_editor::StateInputs::from_state(selected);
            p.actions = serialize(&selected.actions).expect("serializable actions");
            p.transitions = serialize(&selected.transitions).expect("serializable transitions");
            p.loaded_state = Some(key);
        }
        panel_input(ui, hash!("state name"), "State name", &mut p.state_name);
        if panel_button(ui, "Rename state") {
            match rename_state(behavior, p.state, p.state_name.clone()) {
                Ok(()) => {
                    editor.dirty = true;
                    p.structured = hollow_steps::definition_editor::StateInputs::from_state(
                        &behavior.states[p.state],
                    );
                    names = behavior.states.iter().map(|s| s.name.clone()).collect();
                    p.transitions = serialize(&behavior.states[p.state].transitions)
                        .expect("serializable transitions");
                }
                Err(e) => error = Some(e),
            }
        }
        if panel_button(ui, "Make initial") {
            hollow_steps::definition_editor::set_initial_state(behavior, p.state)
                .expect("selected state");
            editor.dirty = true;
        }
        panel_label(ui, &format!("Initial: {}", behavior.initial_state));
        let before = behavior.states[p.state].clone();
        if panel_button(ui, "Advanced: raw RON") {
            p.advanced = !p.advanced;
            // Switching views always starts from the last valid model.
            p.actions = serialize(&before.actions).expect("actions");
            p.transitions = serialize(&before.transitions).expect("transitions");
            p.structured = hollow_steps::definition_editor::StateInputs::from_state(&before);
        }
        if p.advanced {
            panel_label(ui, "Ordered actions (RON list)");
            ui.editbox(
                hash!("actions"),
                vec2(panel_content_width(), 95.0),
                &mut p.actions,
            );
            panel_label(ui, "Transitions (RON list)");
            ui.editbox(
                hash!("transitions"),
                vec2(panel_content_width(), 110.0),
                &mut p.transitions,
            );
            if let Err(e) =
                edit_state_lists(&mut behavior.states[p.state], &p.actions, &p.transitions)
            {
                error = Some(e);
            }
        } else {
            draw_state_rows(ui, p, &names);
            if let Err(e) = p.structured.apply(&mut behavior.states[p.state], &names) {
                error = Some(e);
            }
        }
        editor.dirty |= before != behavior.states[p.state];
        panel_input(ui, hash!("new state"), "New state", &mut p.new_state);
        if panel_button(ui, "Add state") {
            match add_state(behavior, p.new_state.clone()) {
                Ok(()) => {
                    p.state = behavior.states.len() - 1;
                    p.loaded_state = None;
                    editor.dirty = true;
                }
                Err(e) => error = Some(e),
            }
        }
        if panel_button(ui, "Remove state") {
            match remove_state(behavior, p.state) {
                Ok(()) => {
                    p.state = 0;
                    p.loaded_state = None;
                    editor.dirty = true;
                }
                Err(e) => error = Some(e),
            }
        }
    } else {
        error = Some(format!("Missing behavior file {:?}", p.behavior));
    }
    p.error = error.clone();
    if let Some(e) = &error {
        editor.message = e.clone();
    }
    if let Some(e) = error {
        panel_label(ui, &e);
    }
}

fn panel_content_width() -> f32 {
    (screen_width().min(360.0) - 32.0).max(1.0)
}
fn short_panel_text(text: &str) -> String {
    short_text_width(text, panel_content_width())
}
fn short_text_width(text: &str, width: f32) -> String {
    let mut value = text.to_owned();
    if measure_text(&value, None, 16, 1.0).width <= width {
        return value;
    }
    while !value.is_empty() && measure_text(format!("{value}.."), None, 16, 1.0).width > width {
        value.pop();
    }
    format!("{value}..")
}
fn panel_label(ui: &mut macroquad::ui::Ui, text: &str) {
    let columns = (panel_content_width() / 16.0).max(1.0) as usize;
    for line in hollow_steps::definition_editor::wrap_label(text, columns) {
        ui.label(None, &line);
    }
}
fn panel_input(ui: &mut macroquad::ui::Ui, id: u64, label: &str, value: &mut String) {
    panel_label(ui, label);
    widgets::InputText::new(id)
        .size(vec2(panel_content_width(), 23.0))
        .ui(ui, value);
}
fn panel_combo(
    ui: &mut macroquad::ui::Ui,
    id: u64,
    label: &str,
    names: &[String],
    index: &mut usize,
) {
    panel_label(ui, label);
    if names.is_empty() {
        return;
    }
    *index = (*index).min(names.len() - 1);
    let short = names
        .iter()
        .map(|name| short_panel_text(name))
        .collect::<Vec<_>>();
    let labels = short.iter().map(String::as_str).collect::<Vec<_>>();
    widgets::ComboBox::new(id, &labels).ratio(1.0).ui(ui, index);
    panel_label(ui, &names[*index]);
}

fn panel_button(ui: &mut macroquad::ui::Ui, text: impl AsRef<str>) -> bool {
    widgets::Button::new(short_panel_text(text.as_ref()))
        .size(vec2(panel_content_width(), 24.0))
        .ui(ui)
}
fn row_controls(ui: &mut macroquad::ui::Ui) -> Option<u8> {
    if panel_content_width() < 180.0 {
        for (op, label) in ["Up", "Down", "Delete"].iter().enumerate() {
            if panel_button(ui, label) {
                return Some(op as u8);
            }
        }
    } else {
        let width = panel_content_width() / 3.0;
        for (op, label) in ["Up", "Down", "Delete"].iter().enumerate() {
            if op > 0 {
                ui.same_line(op as f32 * width);
            }
            if widgets::Button::new(*label)
                .size(vec2(width - 4.0, 24.0))
                .ui(ui)
            {
                return Some(op as u8);
            }
        }
    }
    None
}
fn draw_state_rows(ui: &mut macroquad::ui::Ui, p: &mut PropertyUi, names: &[String]) {
    use hollow_steps::behavior::{Action, Condition};
    use hollow_steps::definition_editor::{move_row, remove_row};
    let action_names = Action::TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    let condition_names = Condition::TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    panel_label(ui, "Actions (in order)");
    let mut command = None;
    for (i, row) in p.structured.actions.iter_mut().enumerate() {
        ui.separator();
        panel_label(ui, &format!("{}. {}", i + 1, Action::TYPES[row.kind]));
        let template = Action::defaults(row.kind).expect("catalog action");
        for (j, (label, _)) in template.parameters().iter().enumerate() {
            panel_input(ui, hash!("action value", i, j), label, &mut row.values[j]);
            let mut check = template.clone();
            if let Err(error) = check.set_parameter(j, &row.values[j]) {
                panel_label(ui, &error);
            }
        }
        if let Some(op) = row_controls(ui) {
            command = Some((i, op));
        }
    }
    if let Some((i, op)) = command {
        if op == 2 {
            let _ = remove_row(&mut p.structured.actions, i);
        } else {
            let _ = move_row(&mut p.structured.actions, i, op == 0);
        }
    }
    panel_combo(
        ui,
        hash!("add action kind"),
        "Action type",
        &action_names,
        &mut p.add_action,
    );
    if panel_button(ui, "Add action") {
        p.structured.add_action(p.add_action).expect("catalog");
    }
    panel_label(ui, "Transitions (first match wins)");
    let mut command = None;
    for (i, row) in p.structured.transitions.iter_mut().enumerate() {
        ui.separator();
        panel_label(ui, &format!("Transition {}", i + 1));
        let mut target = names.iter().position(|n| *n == row.target).unwrap_or(0);
        panel_combo(
            ui,
            hash!("transition target", i),
            "Target",
            names,
            &mut target,
        );
        row.target = names[target].clone();
        let old_kind = row.kind;
        let mut kind = old_kind;
        panel_combo(
            ui,
            hash!("condition kind", i),
            "Condition",
            &condition_names,
            &mut kind,
        );
        if kind != old_kind {
            row.change_kind(kind).expect("catalog");
        }
        if let Some((label, _)) = Condition::defaults(kind).expect("catalog").parameter() {
            panel_input(ui, hash!("condition value", i), label, &mut row.value);
            if let Err(error) = row.build(names) {
                panel_label(ui, &error);
            }
        }
        ui.checkbox(hash!("invert", i), "Invert", &mut row.invert);
        if let Some(op) = row_controls(ui) {
            command = Some((i, op));
        }
    }
    if let Some((i, op)) = command {
        if op == 2 {
            let _ = remove_row(&mut p.structured.transitions, i);
        } else {
            let _ = move_row(&mut p.structured.transitions, i, op == 0);
        }
    }
    panel_combo(
        ui,
        hash!("add condition kind"),
        "New condition",
        &condition_names,
        &mut p.add_condition,
    );
    if panel_button(ui, "Add transition") {
        p.structured
            .add_transition(names[p.state].clone(), p.add_condition)
            .expect("catalog");
    }
}

#[cfg(test)]
mod editor_ui_tests {
    use super::*;

    #[test]
    fn property_error_only_survives_a_visible_selected_enemy() {
        for (properties, selected, retained) in [
            (true, Some(Selection::Enemy(0)), true),
            (false, Some(Selection::Enemy(0)), false),
            (true, None, false),
            (true, Some(Selection::Item(0)), false),
            (true, Some(Selection::Exit(0)), false),
            (true, Some(Selection::Enemy(1)), false),
        ] {
            let mut state = EditorUi {
                properties,
                selected,
                property: PropertyUi {
                    error: Some("invalid text".into()),
                    ..Default::default()
                },
                ..Default::default()
            };
            state.clear_hidden_property_error(1);
            assert_eq!(state.property.error.is_some(), retained);
        }
    }
}
