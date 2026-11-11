//! Test room: plays the room being edited, straight from the unsaved draft, then returns to
//! the same editor session. Nothing is written to disk and no saved room is changed: the
//! built room stands in for the edited room while the run lasts, and the session, the
//! player and every counter the run can change are parked and put back afterwards.
use std::mem;

use super::GameState;
use crate::editor::EditorSession;
use crate::player::Player;
use crate::room::{place_boss, Room};
use crate::vitals::VitalsState;

/// What a run parks while it is going: the editor session and everything play can change.
#[derive(Debug)]
pub(super) struct TestRun {
    session: EditorSession,
    player: Player,
    vitals: VitalsState,
    coin_count: u32,
    inventory_open: bool,
    special_attack_timer: f32,
    last_special_hit: Option<u64>,
    /// Index into `rooms` of the room that was being played before the test started.
    current: usize,
    /// The saved room the test room stands in for; `None` for a room that is not saved yet.
    replaced: Option<Room>,
}

/// The draft as a playable room, boss included, using the session's preview definitions so
/// unsaved entity edits are what gets played.
fn build(session: &EditorSession) -> Result<Room, String> {
    let mut room = session.draft.build(session.id, &session.definitions)?;
    if session.draft.boss.is_some() {
        let bosses = session.content.boss_definitions()?;
        session.draft.validate_boss(&bosses)?;
        place_boss(&mut room, session.draft.boss.as_ref(), &bosses)?;
    }
    Ok(room)
}

impl GameState {
    /// True while a test room is being played; the editor session is parked meanwhile.
    pub fn testing_room(&self) -> bool {
        self.test_run.is_some()
    }

    /// Plays the open editor's draft with the player at its spawn and the editor closed.
    /// Exits do nothing during a test (they would lead into saved rooms). If the draft is
    /// not a valid room the error is also left in the editor message, and nothing changes.
    pub fn start_test_room(&mut self) -> Result<(), String> {
        if self.test_run.is_some() {
            return Err("already testing the room".into());
        }
        let session = self.editor.as_mut().ok_or("open the editor first")?;
        let room = match build(session) {
            Ok(room) => room,
            Err(error) => {
                session.message = format!("Cannot test the room: {error}");
                return Err(error);
            }
        };
        let session = self.editor.take().expect("editor open");
        let id = session.id;
        let (test_index, replaced) = match self.rooms.iter().position(|saved| saved.id == id) {
            Some(index) => (index, Some(mem::replace(&mut self.rooms[index], room))),
            None => {
                self.rooms.push(room);
                (self.rooms.len() - 1, None)
            }
        };
        let run = TestRun {
            session,
            player: self.player.clone(),
            vitals: self.vitals.clone(),
            coin_count: self.coin_count,
            inventory_open: self.inventory_open,
            special_attack_timer: self.special_attack_timer,
            last_special_hit: self.last_special_hit,
            current: self.current,
            replaced,
        };
        self.current = test_index;
        self.inventory_open = false;
        self.special_attack_timer = 0.0;
        self.last_special_hit = None;
        let (spawn, bounds) = (self.room().spawn, self.world_bounds());
        self.player.respawn(spawn, bounds);
        self.vitals = super::vitals_link::fresh_vitals(*self.vitals.config());
        self.test_run = Some(run);
        Ok(())
    }

    /// Ends the test and reopens the same editor session, draft and all.
    pub fn end_test_room(&mut self) -> Result<(), String> {
        let run = self.test_run.take().ok_or("no test room is running")?;
        let id = run.session.id;
        match run.replaced {
            Some(saved) => {
                let index = self
                    .rooms
                    .iter()
                    .position(|room| room.id == id)
                    .expect("the test room is in the room list");
                self.rooms[index] = saved;
            }
            None => self.rooms.retain(|room| room.id != id),
        }
        self.player = run.player;
        // Vitals values edited during the run stay; the run's health, mana and heal do not.
        let config = *self.vitals.config();
        self.vitals = run.vitals;
        self.vitals.set_config(config);
        self.coin_count = run.coin_count;
        self.inventory_open = run.inventory_open;
        self.special_attack_timer = run.special_attack_timer;
        self.last_special_hit = run.last_special_hit;
        self.current = run.current;
        self.editor = Some(run.session);
        Ok(())
    }
}
