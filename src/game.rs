use std::path::{Path, PathBuf};

use macroquad::math::{Rect, Vec2};

#[cfg(test)]
use crate::boss_special::SpecialAttack;
use crate::coin::Coin;
use crate::damage::DamageResolver;
use crate::editor::{self, EditorSession};
use crate::enemy::EnemyDefinitions;
use crate::events::{EventQueue, GameEvent, LocatedEvent};
use crate::feel::{Feel, FeelConfig};
use crate::item::WorldItem;
use crate::player::{Player, PlayerInput, WorldBounds};
use crate::room::{load_rooms, Room, RoomId};
use crate::tuning::TuningConfig;
use crate::vitals::{VitalsConfig, VitalsState};

#[cfg(test)]
mod death_tests;
mod editor_save;
#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod feel_tests;
#[cfg(test)]
mod flier_tests;
#[cfg(test)]
mod hud_loss_tests;
#[cfg(test)]
mod located_event_tests;
#[cfg(test)]
mod strike_tests;
mod terrain_link;
mod test_room;
#[cfg(test)]
mod tests;
mod vitals_link;
#[cfg(test)]
mod vitals_tests;

pub use crate::room::FLOOR_Y;

pub const VIEW_WIDTH: f32 = 960.0;
pub const VIEW_HEIGHT: f32 = 540.0;

/// The top-level gameplay state for the current prototype.
///
/// Future systems can be added here as separate fields without making the
/// rendering loop responsible for gameplay rules.
#[derive(Debug)]
pub struct GameState {
    player: Player,
    rooms: Vec<Room>,
    editor: Option<EditorSession>,
    assets: PathBuf,
    terrain_styles: crate::terrain_styles::TerrainStyles,
    /// Index into `rooms` of the room being played. Rooms keep their positions, except that
    /// `save_editor` rebuilds the list and recomputes this.
    current: usize,
    coin_count: u32,
    inventory_open: bool,
    special_attack_timer: f32,
    last_special_hit: Option<u64>,
    /// The room being tried out from the editor, if any.
    test_run: Option<test_room::TestRun>,
    /// Player and enemy tunables in effect (the F4 window edits these).
    tuning: TuningConfig,
    /// The tunables as last loaded from or saved to `player.ron`.
    saved_tuning: TuningConfig,
    /// What has happened since `take_events` was last called, oldest first.
    events: EventQueue,
    /// Hit-stop and screen shake, fed by the events each tick raises.
    feel: Feel,
    /// Health trail, mana and healing; see `vitals_link`.
    vitals: VitalsState,
    /// The vitals tunables as last loaded from or saved to `vitals.ron`.
    saved_vitals: VitalsConfig,
}

impl GameState {
    /// Loads the game from the assets folder chosen by `assets::assets_root`. A failure
    /// names the folder that was tried, so the caller can show it instead of panicking.
    pub fn new() -> Result<Self, String> {
        let root = crate::assets::assets_root();
        Self::try_new(&root).map_err(|error| crate::assets::describe_load_failure(&root, &error))
    }

    pub fn try_new(assets: &Path) -> Result<Self, String> {
        let tuning = TuningConfig::load(assets)?;
        let feel = FeelConfig::load(assets)?;
        let vitals = VitalsConfig::load(assets)?;
        let mut rooms = load_rooms(assets)?.into_iter().collect::<Vec<_>>();
        let definitions = EnemyDefinitions::load(assets)?;
        rooms.extend(editor::load_extra_rooms(assets, &definitions)?);
        Self::validate_destinations(&rooms, &definitions)?;
        let world_bounds = rooms[RoomId::First.index()].world_bounds();
        let mut player = Player::new(Vec2::new(180.0, 0.0), tuning.player);
        player.place_at(rooms[RoomId::First.index()].spawn, world_bounds);

        Ok(Self {
            player,
            terrain_styles: terrain_link::load(assets, &rooms)?, // before `rooms` moves
            rooms,
            editor: None,
            assets: assets.to_owned(),
            current: RoomId::First.index(),
            coin_count: 0,
            inventory_open: false,
            special_attack_timer: 0.0,
            last_special_hit: None,
            test_run: None,
            tuning,
            saved_tuning: tuning,
            events: EventQueue::default(),
            feel: Feel::new(feel),
            vitals: vitals_link::fresh_vitals(vitals),
            saved_vitals: vitals,
        })
    }

    /// Hit-stop and screen-shake state.
    pub fn feel(&self) -> &Feel {
        &self.feel
    }

    /// How far to shift the camera for screen shake. Zero while the game is paused.
    pub fn shake_offset(&self) -> Vec2 {
        if self.inventory_open || self.editor.is_some() {
            Vec2::ZERO
        } else {
            self.feel.shake_offset()
        }
    }

    /// Removes and returns the events raised since the last call, oldest first. Each thing
    /// that happens appears exactly once; nothing is raised while the game is paused.
    pub fn take_events(&mut self) -> Vec<GameEvent> {
        self.events.take()
    }

    /// Like `take_events`, with the world position each event happened at (see
    /// `LocatedEvent`). Use one or the other: either removes the events.
    pub fn take_located_events(&mut self) -> Vec<LocatedEvent> {
        self.events.take_located()
    }

    /// The player and enemy tunables in effect.
    pub fn tuning(&self) -> &TuningConfig {
        &self.tuning
    }

    /// True when the tunables in effect differ from `player.ron` (or the defaults).
    pub fn tuning_unsaved(&self) -> bool {
        self.tuning != self.saved_tuning
    }

    /// Puts new tunables into effect at once, mid-game. The player keeps its position,
    /// velocity and timers; every enemy takes the new knockback values. Invalid tunables
    /// are refused and nothing changes.
    pub fn set_tuning(&mut self, tuning: TuningConfig) -> Result<(), String> {
        tuning.validate()?;
        self.player.set_config(tuning.player);
        self.tuning = tuning;
        for enemy in self.rooms.iter_mut().flat_map(|room| &mut room.enemies) {
            enemy.set_tuning(tuning.enemy);
        }
        Ok(())
    }

    /// Writes the tunables in effect to `player.ron` in the assets folder and returns its path.
    pub fn save_tuning(&mut self) -> Result<PathBuf, String> {
        let path = crate::tuning::save(&self.assets, &self.tuning)?;
        self.saved_tuning = self.tuning;
        Ok(path)
    }

    pub fn player(&self) -> &Player {
        &self.player
    }

    pub fn items(&self) -> &[WorldItem] {
        &self.room().items
    }

    pub fn coins(&self) -> &[Coin] {
        &self.room().coins
    }

    pub fn coin_count(&self) -> u32 {
        self.coin_count
    }

    pub fn inventory_open(&self) -> bool {
        self.inventory_open
    }

    pub fn toggle_inventory(&mut self) {
        self.inventory_open = !self.inventory_open;
    }

    pub fn room(&self) -> &Room {
        &self.rooms[self.current]
    }

    fn validate_destinations(rooms: &[Room], definitions: &EnemyDefinitions) -> Result<(), String> {
        let catalog = rooms
            .iter()
            .map(|room| (room.id, editor::RoomLayout::from_room(room)))
            .collect::<Vec<_>>();
        for (id, layout) in &catalog {
            editor::RoomDraft {
                enemies: Vec::new(),
                boss: None,
                layout: layout.clone(),
            }
            .validate(definitions, &catalog)
            .map_err(|e| format!("room {id:?}: {e}"))?;
        }
        Ok(())
    }

    pub fn editor(&self) -> Option<&EditorSession> {
        self.editor.as_ref()
    }
    pub fn editor_mut(&mut self) -> Option<&mut EditorSession> {
        self.editor.as_mut()
    }

    pub fn toggle_editor(&mut self) -> Result<(), String> {
        if self.test_run.is_some() {
            // Closing a test room returns to the session it came from.
            return self.end_test_room();
        }
        if self.editor.take().is_none() {
            self.editor = Some(EditorSession::open(&self.assets, self.room(), &self.rooms)?);
        }
        Ok(())
    }

    pub fn edit_room(&mut self, id: RoomId) -> Result<(), String> {
        if self.test_run.is_some() {
            return Err("leave the test room (F2) before opening another room".into());
        }
        let index = self
            .rooms
            .iter()
            .position(|room| room.id == id)
            .ok_or("missing room")?;
        let room = &self.rooms[index];
        let editor = EditorSession::open(&self.assets, room, &self.rooms)?;
        self.current = index;
        self.player.place_at(room.spawn, room.world_bounds());
        self.editor = Some(editor);
        Ok(())
    }

    pub fn world_bounds(&self) -> WorldBounds {
        self.room().world_bounds()
    }

    /// Re-read every room and its enemy definitions before applying any changes.
    /// Enemy gameplay restarts; matching body points, player, pickups, and unchanged bosses stay intact.
    /// Changed boss definitions restart from their authored placement.
    ///
    /// The player and enemy tunables in `player.ron`, the hit-stop and shake values in
    /// `feel.ron` and the vitals values in `vitals.ron` are re-read too; the player keeps its
    /// state (health, mana and a heal in progress included). Nothing changes if any file fails
    /// to load.
    pub fn reload_rooms(&mut self) -> Result<(), String> {
        if self.test_run.is_some() {
            return Err("leave the test room (F2) before reloading".into());
        }
        let tuning = TuningConfig::load(&self.assets)?;
        let feel = FeelConfig::load(&self.assets)?;
        let vitals = VitalsConfig::load(&self.assets)?;
        let mut rooms = load_rooms(&self.assets)?.into_iter().collect::<Vec<_>>();
        let definitions = EnemyDefinitions::load(&self.assets)?;
        rooms.extend(editor::load_extra_rooms(&self.assets, &definitions)?);
        Self::validate_destinations(&rooms, &definitions)?;
        let terrain_styles = terrain_link::load(&self.assets, &rooms)?;
        for mut reloaded in rooms {
            if let Some(current) = self.rooms.iter_mut().find(|room| room.id == reloaded.id) {
                // Prefer exact placements before matching moved placements by name/occurrence.
                let mut available = current.enemies.iter().map(Some).collect::<Vec<_>>();
                let mut matched = vec![false; reloaded.enemies.len()];
                for (index, enemy) in reloaded.enemies.iter_mut().enumerate() {
                    if let Some(previous) = available.iter_mut().find(|old| {
                        old.is_some_and(|old| old.placement_key() == enemy.placement_key())
                    }) {
                        enemy.preserve_body_motion_from(previous.take().unwrap());
                        matched[index] = true;
                    }
                }
                for (index, enemy) in reloaded.enemies.iter_mut().enumerate() {
                    if !matched[index] {
                        if let Some(previous) = available
                            .iter_mut()
                            .find(|old| old.is_some_and(|old| old.name() == enemy.name()))
                        {
                            enemy.preserve_body_motion_from(previous.take().unwrap());
                        }
                    }
                }
                current.enemies = std::mem::take(&mut reloaded.enemies);
                // The layers and the terrain look are only a picture, so a reload can swap them.
                current.background = std::mem::take(&mut reloaded.background);
                current.terrain_style = std::mem::take(&mut reloaded.terrain_style);
                let same_boss = match (&current.boss_special, &reloaded.boss_special) {
                    (Some(current), Some(reloaded)) => current.same_definition(reloaded),
                    (None, None) => true,
                    _ => false,
                };
                if !same_boss {
                    current.boss_special = reloaded.boss_special.take();
                }
            } else {
                self.rooms.push(reloaded);
            }
        }
        self.set_tuning(tuning)?;
        self.terrain_styles = terrain_styles;
        self.saved_tuning = tuning;
        self.feel.set_config(feel);
        self.vitals.set_config(vitals);
        self.saved_vitals = vitals;
        Ok(())
    }

    /// Advances the game by one fixed tick. Nothing moves, and no hit-stop or shake time
    /// passes, while the inventory or the editor is open.
    ///
    /// Right after a hit, enemies and the boss stand still for the hit-stop time; the player
    /// and the camera carry on.
    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32) {
        if self.inventory_open || self.editor.is_some() {
            return;
        }
        let delta_seconds = if delta_seconds.is_finite() {
            delta_seconds.max(0.0)
        } else {
            0.0
        };
        let frozen = self.feel.hit_stop_active();
        self.feel.advance(delta_seconds);
        let first_new_event = self.events.len();
        self.step(input, delta_seconds, frozen);
        // Hit-stop begins with the next tick, after every event of this one is known.
        self.feel.apply(&self.events.as_slice()[first_new_event..]);
    }

    /// One tick of the world. `frozen` holds enemies and the boss still (hit-stop).
    fn step(&mut self, input: PlayerInput, delta_seconds: f32, frozen: bool) {
        let index = self.current;
        let enemy_tuning = self.tuning.enemy;
        let bounds = self.rooms[index].world_bounds();
        let player_input = self.input_for_player(input);
        self.player.update_with_terrain(
            player_input,
            delta_seconds,
            bounds,
            &self.rooms[index].terrain,
        );
        self.update_vitals(input, delta_seconds);
        let feet = self.player.position() + Vec2::new(0.0, self.player.size().y / 2.0);
        self.events.extend_at(self.player.take_events(), feet);
        let room = &mut self.rooms[index];
        let bounds = room.world_bounds();
        let mut special_damage = 0;
        let boss_update = if frozen {
            None
        } else {
            room.boss_special.as_mut()
        };
        if let Some(boss) = boss_update {
            special_damage = boss.update_with_player_size(
                delta_seconds,
                self.player.position(),
                self.player.size(),
                bounds,
            );
            boss.schedule(delta_seconds, self.player.position(), bounds);
            self.special_attack_timer = boss.attack_timer();
            room.enemies.extend(
                boss.new_enemies()
                    .iter()
                    .cloned()
                    .map(|enemy| enemy.spawned_with(enemy_tuning)),
            );
            room.coins.extend_from_slice(boss.new_coins());
        }
        let mut spawned_positions = Vec::new();
        // During hit-stop no enemy updates: no movement, attack timers, spawns or knockback.
        for enemy in room.enemies.iter_mut().filter(|_| !frozen) {
            enemy.set_tuning(enemy_tuning);
            enemy.update_with_terrain(
                self.player.position(),
                self.player.size(),
                delta_seconds,
                bounds,
                &room.terrain,
            );
            spawned_positions.extend(enemy.take_spawn_positions());
        }
        room.enemies
            .extend(spawned_positions.into_iter().map(|position| {
                crate::enemy::Enemy::new(position.x, position.x, 0.0, 0.0, bounds)
                    .spawned_with(enemy_tuning)
            }));
        if let Some(hitbox) = self.player.strike_hitbox() {
            let mut struck = false;
            for enemy in &mut room.enemies {
                if enemy.touches(hitbox.center(), hitbox.size()) {
                    let direction = self
                        .player
                        .attack()
                        .knockback_direction(self.player.position().x, enemy.position().x);
                    if enemy.hit(self.player.attack().id(), direction) {
                        struck = true;
                        vitals_link::regain(&mut self.player, &mut self.vitals);
                        self.events.push_at(GameEvent::EnemyHit, enemy.position());
                        if enemy.is_dead() {
                            self.vitals.gain_kill_mana();
                            self.events.push_at(GameEvent::EnemyDied, enemy.position());
                        }
                    }
                }
            }
            if let Some(boss) = &mut room.boss_special {
                let top_left = boss.position() - boss.size() / 2.0;
                let body = Rect::new(top_left.x, top_left.y, boss.size().x, boss.size().y);
                let swing_id = self.player.attack().id();
                if hitbox.overlaps(&body) && self.last_special_hit != Some(swing_id) {
                    // A boss that is already defeated is not a pogo target.
                    let was_defeated = boss.is_defeated();
                    struck |= !was_defeated;
                    boss.damage_boss(u32::from(crate::combat::SWORD_DAMAGE));
                    self.last_special_hit = Some(swing_id);
                    if !was_defeated {
                        vitals_link::regain(&mut self.player, &mut self.vitals);
                        self.events.push_at(GameEvent::EnemyHit, boss.position());
                        if boss.is_defeated() {
                            self.events
                                .push_at(GameEvent::BossDefeated, boss.position());
                        }
                    }
                }
                if boss.is_defeated() {
                    special_damage = 0;
                }
            }
            if struck {
                self.player.pogo();
            }
        }
        room.enemies.retain(|enemy| !enemy.is_removable());
        let hit = DamageResolver::gather(
            room,
            self.player.position(),
            self.player.size(),
            special_damage,
        )
        .resolve();
        if self.player.take_attack_damage(hit) {
            let loss = vitals_link::apply_damage(&mut self.vitals, hit);
            self.events
                .push_player_damaged(self.player.position(), loss);
        }
        if self.player.health().current() == 0 {
            self.player.respawn(room.spawn, bounds);
            self.vitals.respawn();
            return;
        }
        // Up with X is an upward strike, not a pickup.
        if input.pickup_pressed && !(input.attack_pressed && input.up_held) {
            let index = self.current;
            let room = &mut self.rooms[index];
            if let Some(index) = room
                .items
                .iter()
                .position(|item| item.can_pick_up(&self.player))
            {
                let item = room.items.remove(index);
                self.player.collect_item(item.item);
                self.events.push(GameEvent::ItemPickedUp);
            }
        }

        let index = self.current;
        let coins = &mut self.rooms[index].coins;
        coins.retain(|coin| {
            if coin.touches(&self.player) {
                self.coin_count = self.coin_count.saturating_add(1);
                self.events.push(GameEvent::CoinCollected);
                false
            } else {
                true
            }
        });

        if let Some(exit) = self
            .room()
            .exit_at(self.player.position(), self.player.size())
            .copied()
            // A test room is a sandbox: its exits would lead into saved rooms.
            .filter(|_| self.test_run.is_none())
        {
            if let Some(target) = self
                .rooms
                .iter()
                .position(|room| room.id == exit.target_room)
            {
                self.current = target;
                self.player.place_at(exit.spawn, self.world_bounds());
                self.events.push(GameEvent::RoomEntered);
            }
        }
    }
}
