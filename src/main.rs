use hollow_steps::boss::{Boss, BossState};
use hollow_steps::boss_special::{BossSpecials, SpecialAttack};
use hollow_steps::coin::COIN_RADIUS;
use hollow_steps::editor::{EditorSession, RoomLayout, Selection};
use hollow_steps::game::{GameState, VIEW_HEIGHT, VIEW_WIDTH};
use hollow_steps::health::MAX_HEALTH;
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomEdge;
use hollow_steps::search::SearchList;
use macroquad::prelude::*;
mod body_draw;
mod editor_panel;
use editor_panel::{BossPropertyUi, PropertyUi};

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
    let mut body_debug = false;
    let body_debug_overlay = body_draw::BodyDebugOverlay::new();

    loop {
        let delta_seconds = get_frame_time().min(1.0 / 15.0);
        if is_key_pressed(KeyCode::F3) {
            body_debug = !body_debug;
        }
        if is_key_pressed(KeyCode::F2) {
            if let Err(error) = game.toggle_editor() {
                reload_result = Some(Err(error));
            }
            editor_ui = EditorUi::default();
        }
        if game.editor().is_some() {
            draw_editor(&mut game, &mut editor_ui, body_debug, &body_debug_overlay);
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
        // Drain the integration's event subscription while the editor is closed, so reopening
        // cannot replay earlier gameplay clicks or keys into its controls.
        egui_macroquad::ui(|_| {});
        egui_macroquad::draw();
        if body_debug {
            body_draw::draw_body_debug_overlay(
                &body_debug_overlay,
                &game.room().enemies,
                game.player().position(),
                |position| position,
                1.0,
            );
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
    for solid in room.terrain.solids() {
        draw_rectangle(
            solid.x,
            solid.y,
            solid.w,
            solid.h,
            Color::from_rgba(34, 37, 58, 255),
        );
        draw_rectangle_lines(
            solid.x,
            solid.y,
            solid.w,
            solid.h,
            3.0,
            Color::from_rgba(111, 122, 164, 255),
        );
    }
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
        for blast in enemy.behavior().blasts() {
            let color = if blast.exploded { RED } else { ORANGE };
            draw_circle_lines(blast.position.x, blast.position.y, blast.radius, 2.0, color);
        }
        let top_left = enemy.position() - enemy.size() / 2.0;
        body_draw::draw_enemy_body(enemy);
        body_draw::draw_enemy_attacks(enemy);
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
    draw_text(
        "F5: reload rooms, definitions and bodies",
        24.0,
        148.0,
        18.0,
        LIGHTGRAY,
    );
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
        if let Some((target_x, radius)) = boss.landing_preview() {
            draw_rectangle(target_x - radius, floor_y - 12.0, radius * 2.0, 12.0, RED);
        }
        for ring in boss.rings() {
            let color = if ring.exploded() { ORANGE } else { RED };
            draw_circle_lines(
                ring.position().x,
                ring.position().y,
                ring.radius(),
                3.0,
                color,
            );
        }
        let top_left = boss.position() - boss.size() / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            boss.size().x,
            boss.size().y,
            if boss.is_telegraphing() {
                ORANGE
            } else {
                PURPLE
            },
        );
    } else if boss.is_death_effect_active() {
        draw_circle(boss.position().x, boss.position().y, boss.size().x, WHITE);
    }
    draw_boss_health(boss.health(), boss.max_health());
    let cue = match boss.active_attack() {
        Some(SpecialAttack::Slam) => "Slam - move out of the red zone!",
        Some(SpecialAttack::Summon) => "Summoning enemies",
        Some(SpecialAttack::Rings) => "Rings - avoid the red circles!",
        None if boss.is_defeated() => "Defeated - collect the coins!",
        None => boss.active_attack_name().unwrap_or("Recovering"),
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
    definition: SearchList,
    room: SearchList,
    target: SearchList,
    new_entity: String,
    edge: usize,
    placing: bool,
    canvas_focused: bool,
    selected: Option<Selection>,
    dragging: Option<(Selection, Vec2)>,
    destination: Option<usize>,
    properties: bool,
    property: PropertyUi,
    boss_property: Option<BossPropertyUi>,
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
    fn new(layout: &RoomLayout, panel_left: f32) -> Self {
        let bounds = layout.rect();
        let scale = ((panel_left - 40.0).max(1.0) / bounds.w)
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
    if let Some(boss) = &editor.draft.boss {
        if let Some(definition) = editor
            .content
            .boss_path(&boss.definition)
            .and_then(|p| editor.content.bosses.get(&p))
        {
            markers.push((
                Selection::Boss,
                body(boss.position.into(), definition.size.into()),
                PURPLE,
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

fn draw_editor(
    game: &mut GameState,
    state: &mut EditorUi,
    body_debug: bool,
    body_debug_overlay: &body_draw::BodyDebugOverlay,
) {
    clear_background(Color::from_rgba(16, 18, 31, 255));
    let player_position = game.player().position();
    let editor = game.editor_mut().expect("editor open");
    if let Ok(definitions) = editor.content.placement_definitions() {
        editor.definitions = definitions;
    }
    let mut panel = None;
    egui_macroquad::ui(|ctx| {
        // The integration divides native wheel events by 90. Normalize once to 28 points per
        // wheel step; egui then routes/consumes the delta for the hovered scroll area.
        let wheel = mouse_wheel();
        let delta =
            egui_macroquad::egui::vec2(wheel.0.clamp(-3.0, 3.0), wheel.1.clamp(-3.0, 3.0)) * 28.0;
        ctx.input_mut(|input| {
            input.raw_scroll_delta = delta;
            input.smooth_scroll_delta = delta;
        });
        panel = Some(editor_panel::draw_panel(ctx, editor, state));
    });
    let panel = panel.expect("egui panel");
    let mut save = panel.save;
    let open = panel.open;

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
    let canvas = EditorCanvas::new(&layout, panel.left);
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
    let over_canvas = area.contains(mouse) && mouse.x < panel.left && !panel.blocks_pointer;
    if is_mouse_button_pressed(MouseButton::Left) {
        state.canvas_focused = over_canvas;
    }
    if is_mouse_button_pressed(MouseButton::Left) && over_canvas {
        if state.placing && state.destination.is_none() {
            if let Some((_, placement)) = state
                .definition
                .selected
                .and_then(|i| editor.content.placement_catalog().get(i).cloned())
            {
                match editor.place(placement, world) {
                    Ok(selection) => {
                        state.selected = Some(selection);
                        editor.dirty = true;
                        editor.message.clear();
                    }
                    Err(error) => editor.message = error,
                }
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
            if state.selected == Some(Selection::Boss) {
                if let Some(path) = editor
                    .draft
                    .boss
                    .as_ref()
                    .and_then(|b| editor.content.boss_path(&b.definition))
                {
                    if !state.boss_property.as_ref().is_some_and(|p| p.path == path) {
                        state.boss_property = Some(BossPropertyUi::new(editor, path));
                    }
                    state.properties = true;
                    state.property.error = None;
                }
            } else if state.properties {
                state.boss_property = None;
                if let Some(Selection::Enemy(index)) = state.selected {
                    let path = editor
                        .content
                        .entity_path(&editor.draft.enemies[index].definition);
                    state.property = PropertyUi::for_entity(editor, path);
                }
            }
        }
    }
    if is_mouse_button_down(MouseButton::Left) && over_canvas {
        if let Some((selection, offset)) = state.dragging {
            match editor.move_selection(selection, world - offset) {
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
        || (is_key_pressed(KeyCode::Delete)
            && state.canvas_focused
            && !panel.keyboard
            && !panel.blocks_pointer)
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
    let pending_error = if let Some(p) = &mut state.boss_property {
        p.sync(editor);
        p.error.clone()
    } else if state.properties {
        state.property.error.clone()
    } else {
        None
    };
    state.clear_hidden_property_error(editor.draft.enemies.len());
    if save && pending_error.is_some() {
        editor.message = format!(
            "Save failed: {}",
            pending_error.as_deref().unwrap_or_default()
        );
        save = false;
    }
    egui_macroquad::draw();
    if body_debug && state.destination.is_none() {
        // Preview authored placements and unsaved definitions without touching runtime bodies.
        let enemies = editor
            .draft
            .enemies
            .iter()
            .filter_map(|placement| {
                editor
                    .definitions
                    .spawn(
                        &placement.definition,
                        placement.patrol_left,
                        placement.patrol_right,
                        editor.draft.layout.world_bounds(),
                    )
                    .ok()
            })
            .collect::<Vec<_>>();
        body_draw::draw_body_debug_overlay(
            body_debug_overlay,
            &enemies,
            player_position,
            |position| canvas.screen(position),
            canvas.scale,
        );
    }
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

#[cfg(test)]
mod editor_ui_tests {
    use super::*;

    #[test]
    fn authored_boss_has_a_canvas_marker() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        let mut game = GameState::try_new(&assets).unwrap();
        game.edit_room(hollow_steps::room::RoomId::Boss).unwrap();
        let editor = game.editor().unwrap();
        let placement = editor.draft.boss.as_ref().unwrap();
        assert!(editor_markers(editor, None, None)
            .iter()
            .any(|(selection, rect, _)| {
                *selection == Selection::Boss
                    && rect.center() == Vec2::from(placement.position)
                    && rect.size() == vec2(48.0, 48.0)
            }));
    }

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
