//! How `GameState` owns the hub-and-run loop (`crate::run`, `crate::profile`,
//! `docs/run-and-profile.md`): the phase, the profile and the file it is saved to, starting a
//! run, ending one, and the result screen between a run and the hub.
//!
//! With no `hub_room` in `run.ron` none of this runs: death respawns in place and the profile is
//! never touched. With a hub, the game starts in it. Leaving it through an exit starts a run
//! (`reset_run_state`); death, the final boss falling or Abandon ends it, banks the run's coins
//! into the profile and saves it, and shows the result until `continue_after_result` returns the
//! player to the hub. Outside the result screen the player is in the Hub phase exactly when the
//! current room is the hub room; the editor can move the player, and the next tick puts the
//! phase right again (`reconcile_run`).
//!
//! A test room is a sandbox: no run starts or ends there, the profile is never touched, and a
//! boss defeated there clears nothing.
use std::path::PathBuf;

use macroquad::math::Vec2;

use super::seed_link::SeedState;
use super::tier_link::Layer;
use super::{vitals_link, world_load, GameState};
use crate::charms::CharmState;
use crate::events::GameEvent;
use crate::item::TakenItems;
use crate::player::WorldBounds;
use crate::profile::{self, Profile};
use crate::room::{Exit, Room, RoomId};
use crate::run::{CoinBonus, RunConfig, RunOutcome, RunPhase, RunResult};

/// The loop's settings, the profile and where the player is in the loop.
#[derive(Debug)]
pub(super) struct RunState {
    config: RunConfig,
    pub(super) profile: Profile,
    /// The file the profile is saved to; none until `use_profile_file`, so tests never touch a
    /// real config folder and nothing is saved before the file has been read.
    profile_path: Option<PathBuf>,
    phase: RunPhase,
    /// What went wrong last (a profile that did not save, a world that did not rebuild).
    message: Option<String>,
    /// The seeds runs start with and the one of the run in progress (`seed_link`).
    pub(super) seeds: SeedState,
}

impl RunState {
    /// The loop for `config` over `rooms`, and the index of the room the game begins in: the
    /// hub if there is one, otherwise `start_room`. Refuses a config naming a room that does not
    /// exist.
    pub(super) fn start(config: RunConfig, rooms: &[Room]) -> Result<(Self, usize), String> {
        let ids: Vec<RoomId> = rooms.iter().map(|room| room.id).collect();
        config.check_rooms(&ids)?;
        if let Some(hub) = rooms.iter().find(|room| Some(room.id) == config.hub_room) {
            config.check_shop(hub.bounds)?;
        }
        let first = config.hub_room.unwrap_or(config.start_room);
        let index = ids
            .iter()
            .position(|id| *id == first)
            .ok_or("run.ron: the first room does not exist")?;
        let state = Self {
            config,
            profile: Profile::default(),
            profile_path: None,
            phase: RunPhase::Hub,
            message: None,
            seeds: SeedState::default(),
        };
        Ok((state, index))
    }
}

impl GameState {
    /// The loop's settings (`run.ron`).
    pub fn run_config(&self) -> &RunConfig {
        &self.run.config
    }

    /// Where the player is in the loop; `None` while `run.ron` names no hub.
    pub fn run_phase(&self) -> Option<RunPhase> {
        self.run.config.has_hub().then_some(self.run.phase)
    }

    /// True while a run is going (not in the hub, not on the result screen, not in a test room).
    pub fn run_in_progress(&self) -> bool {
        self.run_rules_apply() && self.run.phase == RunPhase::Running
    }

    /// True in the hub phase.
    pub fn in_hub(&self) -> bool {
        self.run_rules_apply() && self.run.phase == RunPhase::Hub
    }

    /// The result of the run that just ended, while its screen is showing.
    pub fn run_result(&self) -> Option<RunResult> {
        match self.run_phase()? {
            RunPhase::Result(result) => Some(result),
            _ => None,
        }
    }

    /// What went wrong last in the loop (the profile did not save, the world did not rebuild).
    pub fn run_message(&self) -> Option<&str> {
        self.run.message.as_deref()
    }

    /// What the player keeps: coins banked, difficulty tier, dungeons cleared. The coins of the
    /// run in progress are `coin_count`.
    pub fn profile(&self) -> &Profile {
        &self.run.profile
    }

    /// The loop is on and this is not a test room: the rules below apply.
    fn run_rules_apply(&self) -> bool {
        self.run.config.has_hub() && self.test_run.is_none()
    }

    /// True while the result screen shows; the world is frozen then.
    pub(super) fn showing_result(&self) -> bool {
        self.run_phase()
            .is_some_and(|phase| matches!(phase, RunPhase::Result(_)))
    }

    /// Reads the profile saved at `path` and saves every later bank there. A missing file is a
    /// fresh profile. A file that cannot be used is moved to `profile.ron.bak`, a fresh profile
    /// is used and the reason comes back as the error, to show on screen.
    pub fn use_profile_file(&mut self, path: PathBuf) -> Result<(), String> {
        let (profile, problem) = profile::load(&path);
        self.run.profile = profile;
        self.run.profile_path = Some(path);
        // The upgrades the profile holds are in effect from the start.
        self.apply_stats();
        problem.map_or(Ok(()), Err)
    }

    /// Replaces the profile (the F4 window's debug controls) and saves it. A failed save keeps
    /// the new profile for this session and returns the reason.
    pub fn set_profile(&mut self, profile: Profile) -> Result<(), String> {
        self.run.profile = profile;
        self.apply_stats();
        self.save_profile()
    }

    pub(super) fn save_profile(&self) -> Result<(), String> {
        match &self.run.profile_path {
            Some(path) => profile::save(path, &self.run.profile),
            None => Ok(()),
        }
    }

    /// Gives up the run in progress: its coins are banked and the result screen shows. Does
    /// nothing outside a run, and nothing in a test room.
    pub fn abandon_run(&mut self) {
        if self.run_in_progress() {
            self.end_run(RunOutcome::Abandoned);
        }
    }

    /// The program is closing: a run in progress is banked as Abandoned first. A test room is
    /// ended first, so the run it parked is the one that is banked.
    pub fn abandon_run_on_exit(&mut self) {
        if self.testing_room() {
            let _ = self.end_test_room();
        }
        self.abandon_run();
    }

    /// Leaves the result screen: the run is over, so everything it touched is rebuilt and the
    /// player stands in the hub with full health and mana. Does nothing while no result shows.
    /// If the rooms cannot be rebuilt the screen stays and `run_message` says why.
    pub fn continue_after_result(&mut self) {
        let (RunPhase::Result(_), Some(hub)) = (self.run.phase, self.run.config.hub_room) else {
            return;
        };
        // Back in the hub nothing has a tier on it.
        if let Err(error) = self.reset_run_state(Layer::BASE) {
            self.run.message = Some(format!("Could not return to the hub: {error}"));
            return;
        }
        let Some(index) = self.room_index(hub) else {
            self.run.message = Some("Could not return to the hub: it is gone".to_owned());
            return;
        };
        self.current = index;
        let (spawn, bounds) = (self.room().spawn, self.world_bounds());
        self.player.respawn(spawn, bounds);
        self.refill_vitals();
        self.run.phase = RunPhase::Hub;
        // The hub has no seed; `last_run_seed` still has the one that just ended.
        self.run.seeds.current = None;
        self.run.message = None;
        self.discover_current_room();
        self.events.push(GameEvent::RoomEntered);
    }

    fn room_index(&self, id: RoomId) -> Option<usize> {
        self.rooms.iter().position(|room| room.id == id)
    }

    /// Banks the run's coins into the profile, saves it, and shows the result.
    fn end_run(&mut self, outcome: RunOutcome) {
        // The tier's bonus is paid on what the run collected, before the share that is kept.
        let layer = self.tiers.layer;
        let earned = layer.scale.coins(self.coin_count);
        self.tiers.bonus = CoinBonus::new(layer.profile_tier, &layer.scale);
        let result = self
            .run
            .profile
            .bank(earned, self.run.config.coin_keep_percent, outcome);
        // A failed save keeps the banked coins for this session.
        self.run.message = self.save_profile().err().map(|error| {
            format!("Profile not saved, the coins are kept for this session: {error}")
        });
        self.run.phase = RunPhase::Result(result);
        // The run is over, and so is what it showed on the map.
        self.map.discovery.clear();
        self.map.open = false;
        self.inventory_open = false;
        self.pause_menu.close();
    }

    /// Everything a run touched back to how a new run begins: every room rebuilt from its file
    /// (enemies, boss, coins, items, charms lying), owned charms back to the ones owned at the
    /// start, nothing taken, no coins, no items, full health and mana. Nothing changes if the
    /// rooms cannot be rebuilt. The rooms are rebuilt with `layer` over them and keep it until the
    /// next rebuild: a run's tier, or none when the run is over. The player's place is left to the
    /// caller.
    fn reset_run_state(&mut self, layer: Layer) -> Result<(), String> {
        let here = self.room().id;
        let (rooms, terrain_styles) = world_load::load_world(&self.assets, &self.charms, &layer)?;
        let current = rooms
            .iter()
            .position(|room| room.id == here)
            .ok_or("the room being played is gone")?;
        self.rooms = rooms;
        self.terrain_styles = terrain_styles;
        self.tiers.layer = layer;
        self.current = current;
        // A new run has seen nothing yet.
        self.map.discovery.clear();
        self.map.open = false;
        self.coin_count = 0;
        self.taken_items = TakenItems::default();
        // Every door is locked again, and no notice outlives the run.
        self.locks = super::world_locks::Locks::default();
        // Every wall is whole and every switch off again.
        self.puzzles = super::world_puzzles::Puzzles::default();
        self.charm_state = CharmState::starting(&self.charms);
        // Charms owned at the start are not lying in the rebuilt rooms.
        self.prune_owned_charm_placements();
        self.player.reset_for_run();
        self.special_attack_timer = 0.0;
        self.last_special_hit = None;
        self.inventory_open = false;
        self.charm_menu.clear_message();
        self.feel = super::Feel::new(*self.feel.config());
        // The equipped charms are gone, so the numbers in effect change before health is filled.
        self.apply_stats();
        self.refill_vitals();
        Ok(())
    }

    /// Full health and full mana, with no trailing bar or heal left over.
    fn refill_vitals(&mut self) {
        self.vitals =
            vitals_link::fresh_vitals(*self.vitals.config(), self.player.config().max_health);
        self.vitals.fill_mana();
    }

    /// Starts a run: the state is reset and the phase is Running. The profile's tier is read
    /// here, once, so the run plays at it from start to end, and so is the run's seed: the fixed
    /// one if F4 set it, otherwise the next from the source. The seed goes into the layer the
    /// rooms are built with, so it decides the optional enemy placements and every rebuild in
    /// the run keeps its picks. A run that cannot start takes no seed.
    fn start_run(&mut self) -> Result<(), String> {
        let (seed, source) = self.run.seeds.next_seed();
        self.reset_run_state(self.profile_layer().with_seed(seed))?;
        self.run.seeds.begin(seed, source);
        self.run.phase = RunPhase::Running;
        self.run.message = None;
        Ok(())
    }

    /// Moves the player into room `target` at `spawn`.
    fn enter_room(&mut self, target: usize, spawn: Vec2) {
        self.current = target;
        self.player.place_at(spawn, self.world_bounds());
        self.discover_current_room();
        self.events.push(GameEvent::RoomEntered);
    }

    /// The player walked into `exit` (never in a test room). A locked exit opens only with its
    /// key (`pass_exit`). Leaving the hub starts a run, and entering the hub during a run
    /// abandons it. If a run cannot start (the rooms cannot be
    /// rebuilt) the player stays in the hub, back at its spawn so the next try is a walk away.
    pub(super) fn use_exit(&mut self, exit: Exit) {
        let Some(mut target) = self.room_index(exit.target_room) else {
            return;
        };
        // A locked exit asks for its key first; without it nothing else happens.
        if !self.pass_exit(exit) {
            return;
        }
        if self.run_rules_apply() {
            let hub = self.run.config.hub_room;
            if self.run.phase == RunPhase::Hub && hub == Some(self.room().id) {
                if let Err(error) = self.start_run() {
                    self.run.message = Some(format!("Could not start the run: {error}"));
                    let (spawn, bounds) = (self.room().spawn, self.world_bounds());
                    self.player.place_at(spawn, bounds);
                    return;
                }
                // The rooms were rebuilt, so the target's place in the list is looked up again.
                let Some(rebuilt) = self.room_index(exit.target_room) else {
                    return;
                };
                target = rebuilt;
            } else if self.run.phase == RunPhase::Running && hub == Some(exit.target_room) {
                self.enter_room(target, exit.spawn);
                self.end_run(RunOutcome::Abandoned);
                return;
            }
        }
        self.enter_room(target, exit.spawn);
    }

    /// Puts the phase right with where the player is, at the start of a tick. The editor can
    /// move the player to another room (Open room, Save), and that counts as walking there:
    /// standing outside the hub in the Hub phase starts a run, and standing in the hub during a
    /// run abandons it.
    pub(super) fn reconcile_run(&mut self) {
        let Some(hub) = self.run.config.hub_room.filter(|_| self.run_rules_apply()) else {
            return;
        };
        let in_hub = self.room().id == hub;
        match self.run.phase {
            RunPhase::Hub if !in_hub => {
                if let Err(error) = self.start_run() {
                    self.run.message = Some(format!("Could not start the run: {error}"));
                    if let Some(index) = self.room_index(hub) {
                        let spawn = self.rooms[index].spawn;
                        self.enter_room(index, spawn);
                    }
                }
            }
            RunPhase::Running if in_hub => self.end_run(RunOutcome::Abandoned),
            _ => {}
        }
    }

    /// The end of a tick's combat. In a run, the boss of the final boss room falling (an event of
    /// this tick, from `events_before` on) clears it and the player's health reaching zero ends
    /// it as a death. Anywhere else death respawns the player in the room at `spawn`, as it
    /// always did. Returns true when the tick is over.
    pub(super) fn settle_combat(
        &mut self,
        events_before: usize,
        spawn: Vec2,
        bounds: WorldBounds,
    ) -> bool {
        let dead = self.player.health().current() == 0;
        if self.run_in_progress() {
            let fell = self.events.as_slice()[events_before..].contains(&GameEvent::BossDefeated)
                && self.room().id == self.run.config.final_boss_room;
            if fell {
                self.end_run(RunOutcome::Cleared);
                return true;
            }
            if dead {
                self.end_run(RunOutcome::Died);
                return true;
            }
            return false;
        }
        if dead {
            self.player.respawn(spawn, bounds);
            self.vitals.respawn();
        }
        dead
    }
}
