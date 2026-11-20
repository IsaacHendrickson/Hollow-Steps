use std::path::{Path, PathBuf};

use macroquad::math::{Rect, Vec2};

#[cfg(test)]
use crate::boss_special::SpecialAttack;
use crate::charm_menu::CharmMenu;
use crate::charms::{CharmCatalog, CharmState};
use crate::coin::Coin;
use crate::damage::DamageResolver;
use crate::editor::{self, EditorSession};
use crate::enemy::EnemyDefinitions;
use crate::events::{EventQueue, GameEvent, LocatedEvent};
use crate::feel::{Feel, FeelConfig};
use crate::item::{TakenItems, WorldItem};
use crate::keybinds::Keybinds;
use crate::pause_menu::PauseMenu;
use crate::player::{Player, PlayerInput, WorldBounds};
use crate::player_editor::PlayerDraft;
use crate::room::{Room, RoomId};
use crate::run::RunConfig;
use crate::tuning::TuningConfig;
use crate::vitals::{VitalsConfig, VitalsState};

mod camera_bounds;
mod charm_editor_link;
mod charm_link;
#[cfg(test)]
mod charm_menu_tests;
#[cfg(test)]
mod charm_tests;
#[cfg(test)]
mod death_tests;
#[cfg(test)]
mod door_tests;
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
mod item_tests;
mod keybind_link;
#[cfg(test)]
mod keybind_tests;
#[cfg(test)]
mod located_event_tests;
mod pause_link;
#[cfg(test)]
mod pause_tests;
mod player_link;
#[cfg(test)]
mod player_stats_tests;
mod run_link;
#[cfg(test)]
mod run_pause_tests;
#[cfg(test)]
mod run_profile_tests;
#[cfg(test)]
mod run_tests;
#[cfg(test)]
mod strike_tests;
mod terrain_link;
mod test_room;
#[cfg(test)]
mod tests;
mod vitals_link;
#[cfg(test)]
mod vitals_tests;
mod world_charms;
mod world_doors;
mod world_items;
mod world_load;

pub use crate::room::FLOOR_Y;
pub use charm_editor_link::CharmsSaved;

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
    /// The pause menu; while it is open nothing in the world advances (see `pause_link`).
    pause_menu: PauseMenu,
    /// The keys that do each action, and the file they are saved to after every change (none
    /// until `use_keybinds_file`, so tests never touch a real config folder; see
    /// `keybind_link`).
    keybinds: Keybinds,
    keybinds_path: Option<PathBuf>,
    special_attack_timer: f32,
    last_special_hit: Option<u64>,
    /// The room being tried out from the editor, if any.
    test_run: Option<test_room::TestRun>,
    /// The player and enemy tunables and the vitals tunables: the values in effect (what the F4
    /// window and the editor's Player panel edit) and the ones last loaded from or saved to
    /// `player.ron` and `vitals.ron`.
    player_draft: PlayerDraft,
    /// What has happened since `take_events` was last called, oldest first.
    events: EventQueue,
    /// Hit-stop and screen shake, fed by the events each tick raises.
    feel: Feel,
    /// Health trail, mana and healing; see `vitals_link`. Its tunables are the draft's edited
    /// vitals with the equipped charms on top (`charm_link::apply_stats`).
    vitals: VitalsState,
    /// Every charm, and which of them the run owns and wears; see `charm_link`.
    charms: CharmCatalog,
    charm_state: CharmState,
    /// The cursor and the last refusal of the inventory's Charms section.
    charm_menu: CharmMenu,
    /// The items taken during the run; see `world_items`.
    taken_items: TakenItems,
    /// The hub-and-run loop: its settings, the profile and where the player is in the loop; see
    /// `run_link`. Inert while `run.ron` names no hub.
    run: run_link::RunState,
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
        let charms = CharmCatalog::load(assets, vitals.max_mana)?;
        let run_config = RunConfig::load(assets)?;
        let (rooms, terrain_styles) = world_load::load_world(assets, &charms)?;
        let (run, current) = run_link::RunState::start(run_config, &rooms)?;
        let mut player = Player::new(Vec2::new(180.0, 0.0), tuning.player);
        player.place_at(rooms[current].spawn, rooms[current].world_bounds());

        let mut game = Self {
            player,
            terrain_styles,
            rooms,
            editor: None,
            assets: assets.to_owned(),
            current,
            coin_count: 0,
            inventory_open: false,
            pause_menu: PauseMenu::default(),
            keybinds: Keybinds::default(),
            keybinds_path: None,
            special_attack_timer: 0.0,
            last_special_hit: None,
            test_run: None,
            player_draft: PlayerDraft::new(tuning).with_vitals(vitals),
            events: EventQueue::default(),
            feel: Feel::new(feel),
            vitals: vitals_link::fresh_vitals(vitals, tuning.player.max_health),
            charm_state: CharmState::starting(&charms),
            charm_menu: CharmMenu::default(),
            taken_items: TakenItems::default(),
            charms,
            run,
        };
        game.prune_owned_charm_placements();
        game.apply_stats();
        Ok(game)
    }

    /// Hit-stop and screen-shake state.
    pub fn feel(&self) -> &Feel {
        &self.feel
    }

    /// How far to shift the camera for screen shake. Zero while the game is paused.
    pub fn shake_offset(&self) -> Vec2 {
        if self.is_paused() || self.editor.is_some() {
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
        self.player_draft.edited()
    }

    /// True when the tunables in effect differ from `player.ron` (or the defaults).
    pub fn tuning_unsaved(&self) -> bool {
        self.player_draft.is_stats_dirty()
    }

    /// Puts new tunables into effect at once, mid-game. The player keeps its position,
    /// velocity and timers; every enemy takes the new knockback values. A new maximum health
    /// raises or cuts the player's health (see `Health::set_limits`). Invalid tunables are
    /// refused and nothing changes.
    pub fn set_tuning(&mut self, tuning: TuningConfig) -> Result<(), String> {
        self.player_draft.set_edited(tuning)?;
        self.apply_stats();
        Ok(())
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

    /// Opens or closes the inventory. Ignored while the pause menu is open or a run's result
    /// shows. An old charm refusal is not kept for the next time.
    pub fn toggle_inventory(&mut self) {
        if self.pause_menu.is_open() || self.showing_result() {
            return;
        }
        self.inventory_open = !self.inventory_open;
        self.charm_menu.clear_message();
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
            self.attach_player_draft();
            self.attach_charm_draft();
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
        self.attach_player_draft();
        self.attach_charm_draft();
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
    /// `feel.ron`, the vitals values in `vitals.ron` and the charms in `charms.ron` are re-read
    /// too; the player keeps its state (health, mana and a heal in progress included) and the
    /// charms it wears stay on, layered on the re-read numbers. Nothing changes if any file fails
    /// to load.
    pub fn reload_rooms(&mut self) -> Result<(), String> {
        if self.test_run.is_some() {
            return Err("leave the test room (F2) before reloading".into());
        }
        let tuning = TuningConfig::load(&self.assets)?;
        let feel = FeelConfig::load(&self.assets)?;
        let vitals = VitalsConfig::load(&self.assets)?;
        let charms = CharmCatalog::load(&self.assets, vitals.max_mana)?;
        let (rooms, terrain_styles) = world_load::load_world(&self.assets, &charms)?;
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
        self.player_draft.mark_saved();
        self.feel.set_config(feel);
        self.player_draft.reload_vitals(vitals);
        // Charms you hold that the file no longer lists are dropped; the rest stay on.
        self.charms = charms;
        self.charm_state.retain_known(&self.charms);
        // An open Charms panel with nothing unsaved shows the re-read list; unsaved edits stay.
        if !self.charms_unsaved() {
            self.attach_charm_draft();
        }
        self.prune_owned_charm_placements();
        self.apply_stats();
        Ok(())
    }

    /// Advances the game by one fixed tick. Nothing moves, and no hit-stop or shake time
    /// passes, while the inventory, the pause menu or the editor is open.
    ///
    /// Right after a hit, enemies and the boss stand still for the hit-stop time; the player
    /// and the camera carry on.
    pub fn update(&mut self, input: PlayerInput, delta_seconds: f32) {
        if self.is_paused() || self.editor.is_some() {
            return;
        }
        self.reconcile_run();
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
        let events_before = self.events.len();
        let index = self.current;
        let enemy_tuning = self.tuning().enemy;
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
            let sword_damage = self.player.config().melee.damage;
            let mut struck = false;
            for enemy in &mut room.enemies {
                if enemy.touches(hitbox.center(), hitbox.size()) {
                    let direction = self
                        .player
                        .attack()
                        .knockback_direction(self.player.position().x, enemy.position().x);
                    if enemy.hit_for(self.player.attack().id(), direction, sword_damage) {
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
                    boss.damage_boss(u32::from(sword_damage));
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
        let spawn = room.spawn;
        if self.settle_combat(events_before, spawn, bounds) {
            return;
        }
        // Up with X is an upward strike, not a pickup or a door.
        let interact = input.pickup_pressed && !(input.attack_pressed && input.up_held);
        let picked_up = interact && (self.pick_up_item() || self.pick_up_charm());

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

        // An item in reach takes the Up press first; otherwise it opens the door in front.
        let door = if interact && !picked_up {
            self.door_in_reach().copied()
        } else {
            None
        };
        if let Some(exit) = self
            .room()
            .exit_at(self.player.position(), self.player.size())
            .copied()
            .or(door)
            // A test room is a sandbox: its exits would lead into saved rooms.
            .filter(|_| self.test_run.is_none())
        {
            self.use_exit(exit);
        }
    }
}
