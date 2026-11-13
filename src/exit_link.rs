//! Linking an exit to another room: the flow the room editor runs when an exit is placed
//! or "Link..." is pressed. A is the room being edited, B the room the exit should lead to.
//!
//! 1. A chooser lists the saved rooms and unsaved drafts by name, and "New room...".
//!    Cancel leaves A as it was; a just-placed exit is taken out again.
//! 2. B opens in the editor. A's draft waits, untouched, among the session's parked drafts.
//!    An exit in B is picked or placed, and Connect finishes the link.
//! 3. A's exit leads to B and arrives next to the chosen exit; with Two-way, B's exit leads
//!    back to A and arrives next to A's exit. The editor returns to A.
//!
//! Nothing is written here. Every draft the flow touches stays in memory (the session's
//! `parked` list) until the normal Save, which writes them all or none. `ExitLink` is the
//! state; the `EditorSession` methods below are its transitions, and egui only draws them.
use macroquad::math::Vec2;

use crate::behavior::read_ron;
use crate::editor::{room_filename, RoomDraft};
use crate::editor_session::{EditorSession, RoomWork};
use crate::exit_arrival::{arrival_next_to, exit_at_click};
use crate::room::{RoomEdge, RoomEnemies, RoomId};
use crate::room_name::NamePrompt;

/// Where the flow is.
#[derive(Clone, Debug)]
pub enum Step {
    /// The room chooser is open.
    Choosing,
    /// "New room..." asks for the new room's name.
    Naming(NamePrompt),
    /// B is open and an exit in it is being picked or placed.
    Connecting(Box<Connecting>),
}

/// B, while it is open.
#[derive(Clone, Debug)]
pub struct Connecting {
    pub target: RoomId,
    /// B was created by this flow, so Cancel throws it away.
    pub created: bool,
    pub two_way: bool,
    /// The exit placed in B during this step and not connected yet. It always leads back
    /// to A, so connecting it forces Two-way on.
    pub placed: Option<usize>,
    /// B's draft as it was when B opened, for undo.
    opened: RoomDraft,
    /// B as it was in the session's parked list before the flow took it, to put back on
    /// Cancel; `None` when B came from disk or was created.
    previous: Option<RoomWork>,
}

/// The exit in A being linked.
#[derive(Clone, Debug)]
struct Origin {
    room: RoomId,
    exit: usize,
    /// The exit was placed to start this flow, so Cancel takes it out again.
    placed: bool,
    /// A's draft and unsaved flag before the flow added anything to them.
    before: RoomDraft,
    was_dirty: bool,
}

#[derive(Clone, Debug)]
pub struct ExitLink {
    origin: Origin,
    step: Step,
}

/// A room the chooser offers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkCandidate {
    pub id: RoomId,
    pub title: String,
    /// The room has unsaved changes or does not exist on disk yet.
    pub unsaved: bool,
    /// The room the exit is in; choosing it is refused.
    pub is_origin: bool,
}

/// What a finished link did, for the front end's undo history.
#[derive(Clone, Debug)]
pub struct Linked {
    pub from: RoomId,
    pub to: RoomId,
    pub two_way: bool,
    /// Where B's chosen exit led before it was pointed back at A, if it led elsewhere.
    pub replaced: Option<RoomId>,
    /// A's and B's drafts as they were before the flow changed them.
    pub from_before: RoomDraft,
    pub to_before: RoomDraft,
}

impl ExitLink {
    pub fn step(&self) -> &Step {
        &self.step
    }
    pub fn origin_room(&self) -> RoomId {
        self.origin.room
    }
    pub fn origin_exit(&self) -> usize {
        self.origin.exit
    }
    /// The flow is in B (as opposed to a chooser over A).
    pub fn is_connecting(&self) -> bool {
        matches!(self.step, Step::Connecting(_))
    }
}

impl EditorSession {
    pub fn link_active(&self) -> bool {
        self.link.is_some()
    }

    /// Starts linking exit `exit` of the room being edited: opens the chooser.
    pub fn begin_link(&mut self, exit: usize) -> Result<(), String> {
        self.start_link(exit, false, self.draft.clone(), self.dirty)
    }

    /// Places a new exit on `edge` (leading nowhere useful yet) and opens the chooser for
    /// it; Cancel takes the exit out again. Returns the new exit's index.
    pub fn begin_link_new_exit(&mut self, edge: RoomEdge) -> Result<usize, String> {
        if self.link.is_some() {
            return Err("finish or cancel the link in progress first".into());
        }
        let before = self.draft.clone();
        let was_dirty = self.dirty;
        let spawn = self.draft.layout.spawn.into();
        let id = self.id;
        self.draft.add_exit(edge, id, spawn);
        let exit = self.draft.layout.exits.len() - 1;
        self.dirty = true;
        self.start_link(exit, true, before, was_dirty)?;
        Ok(exit)
    }

    fn start_link(
        &mut self,
        exit: usize,
        placed: bool,
        before: RoomDraft,
        was_dirty: bool,
    ) -> Result<(), String> {
        if self.link.is_some() {
            return Err("finish or cancel the link in progress first".into());
        }
        if exit >= self.draft.layout.exits.len() {
            return Err("select an exit to link first".into());
        }
        self.link = Some(ExitLink {
            origin: Origin {
                room: self.id,
                exit,
                placed,
                before,
                was_dirty,
            },
            step: Step::Choosing,
        });
        self.message.clear();
        Ok(())
    }

    /// The rooms the chooser offers, the saved ones first, then new unsaved rooms.
    pub fn link_candidates(&self) -> Vec<LinkCandidate> {
        let origin = self.link.as_ref().map(|link| link.origin.room);
        let candidate = |id: RoomId, unsaved: bool| LinkCandidate {
            title: self.title_of(id),
            unsaved,
            is_origin: Some(id) == origin,
            id,
        };
        let mut rooms: Vec<_> = self
            .rooms
            .iter()
            .map(|(id, _)| {
                let parked = self.parked.iter().any(|w| w.id == *id && w.dirty);
                candidate(*id, parked || (*id == self.id && self.dirty))
            })
            .collect();
        let known: Vec<RoomId> = self.rooms.iter().map(|(id, _)| *id).collect();
        for work in self.parked.iter().filter(|w| !known.contains(&w.id)) {
            rooms.push(candidate(work.id, true));
        }
        if self.new && !known.contains(&self.id) {
            rooms.push(candidate(self.id, true));
        }
        rooms
    }

    /// A room was chosen in the chooser: B opens. A itself is refused.
    pub fn link_choose(&mut self, target: RoomId) -> Result<(), String> {
        let link = self.link.as_ref().ok_or("no link in progress")?;
        if !matches!(link.step, Step::Choosing) {
            return Err("choose a room first".into());
        }
        if target == link.origin.room {
            let error = "An exit cannot lead to the room it is in; choose another room".to_owned();
            self.message = error.clone();
            return Err(error);
        }
        let (work, previous) = match self.parked.iter().position(|w| w.id == target) {
            Some(index) => {
                let work = self.parked.remove(index);
                (work.clone(), Some(work))
            }
            None => (self.load_saved_work(target)?, None),
        };
        self.open_target(work, false, previous);
        Ok(())
    }

    /// "New room..." was pressed in the chooser: asks for a name.
    pub fn link_new_room(&mut self) -> Result<(), String> {
        let suggestion = self.suggested_room_name();
        let link = self.link.as_mut().ok_or("no link in progress")?;
        if !matches!(link.step, Step::Choosing) {
            return Err("choose a room first".into());
        }
        link.step = Step::Naming(NamePrompt::new(suggestion));
        Ok(())
    }

    /// The name window's text changed or was confirmed: keeps the prompt's text and error.
    pub fn link_prompt(&mut self) -> Option<&mut NamePrompt> {
        match self.link.as_mut().map(|link| &mut link.step) {
            Some(Step::Naming(prompt)) => Some(prompt),
            _ => None,
        }
    }

    /// The name window confirmed `text`: the room is created (an unsaved draft with the
    /// next free id and this name, shaped like A) and opens as B. A refused name is
    /// returned and the window stays open.
    pub fn link_name_room(&mut self, text: &str) -> Result<(), String> {
        let link = self.link.as_ref().ok_or("no link in progress")?;
        if !matches!(link.step, Step::Naming(_)) {
            return Err("ask for a name first".into());
        }
        let id = self.free_room_id();
        let name = self.check_room_name(id, text)?;
        let mut draft = RoomDraft::new_room(&self.draft.layout);
        draft.layout.name = Some(name);
        let work = RoomWork {
            id,
            path: self.path.with_file_name(room_filename(id)),
            draft,
            new: true,
            dirty: true,
        };
        self.open_target(work, true, None);
        Ok(())
    }

    /// B is the current room and A goes to the parked list.
    fn open_target(&mut self, work: RoomWork, created: bool, previous: Option<RoomWork>) {
        let target = work.id;
        let opened = work.draft.clone();
        let a = self.swap_work(work);
        self.parked.push(a);
        if let Some(link) = &mut self.link {
            link.step = Step::Connecting(Box::new(Connecting {
                target,
                created,
                two_way: true,
                placed: None,
                opened,
                previous,
            }));
        }
        self.message.clear();
    }

    /// The room as it is on disk, for a room the session has no unsaved draft of.
    fn load_saved_work(&self, id: RoomId) -> Result<RoomWork, String> {
        let layout = self
            .rooms
            .iter()
            .find(|(room, _)| *room == id)
            .map(|(_, layout)| layout.clone())
            .ok_or("that room does not exist")?;
        let path = self.path.with_file_name(room_filename(id));
        let document: RoomEnemies = read_ron(&path)?;
        Ok(RoomWork {
            id,
            path,
            draft: RoomDraft {
                enemies: document.enemies,
                boss: document.boss,
                layout,
            },
            new: false,
            dirty: false,
        })
    }

    /// The Two-way checkbox. Ignored for a freshly placed exit, which is always two-way.
    pub fn link_set_two_way(&mut self, on: bool) {
        if let Some(Step::Connecting(connecting)) = self.link.as_mut().map(|link| &mut link.step) {
            connecting.two_way = on;
        }
    }

    /// The Two-way checkbox as the user left it, whether or not a placed exit forces it on.
    pub fn link_two_way_setting(&self) -> bool {
        match self.link.as_ref().map(|link| &link.step) {
            Some(Step::Connecting(c)) => c.two_way,
            _ => false,
        }
    }

    /// Whether the link will also point B's exit back at A.
    pub fn link_two_way(&self) -> bool {
        match self.link.as_ref().map(|link| &link.step) {
            Some(Step::Connecting(c)) => c.two_way || c.placed.is_some(),
            _ => false,
        }
    }

    /// "New exit here": places an exit in B at `position` (replacing one placed earlier in
    /// this step), on the nearest edge, leading back to A. Refused like any exit that does
    /// not fit (inside a solid, say), and then nothing changes. Returns the exit's index.
    pub fn link_place_exit(&mut self, position: Vec2) -> Result<usize, String> {
        let link = self.link.as_ref().ok_or("no link in progress")?;
        let Step::Connecting(connecting) = &link.step else {
            return Err("open the room to link to first".into());
        };
        let from = link.origin.room;
        let spawn = self
            .parked
            .iter()
            .find(|work| work.id == from)
            .map_or(Vec2::ZERO, |work| work.draft.layout.spawn.into());
        let mut next = self.draft.clone();
        if let Some(previous) = connecting.placed {
            next.layout.exits.remove(previous);
        }
        let (edge, offset, length) = exit_at_click(&next.layout, position);
        next.add_exit(edge, from, spawn);
        let index = next.layout.exits.len() - 1;
        next.layout.exits[index].offset = offset;
        next.layout.exits[index].length = length;
        next.validate_local(&self.definitions)?;
        self.draft = next;
        self.dirty = true;
        if let Some(Step::Connecting(c)) = self.link.as_mut().map(|link| &mut link.step) {
            c.placed = Some(index);
        }
        Ok(index)
    }

    /// Connect, with exit `exit` of B chosen: A's exit leads to B and arrives next to it;
    /// with Two-way, B's exit leads to A and arrives next to A's exit. Refused (and
    /// nothing changes) when no free arrival point exists. On success the editor is back
    /// in A, and B waits among the parked drafts.
    pub fn link_connect(&mut self, exit: usize) -> Result<Linked, String> {
        let link = self.link.as_ref().ok_or("no link in progress")?;
        let Step::Connecting(connecting) = &link.step else {
            return Err("open the room to link to first".into());
        };
        let (from, a_exit) = (link.origin.room, link.origin.exit);
        let to = connecting.target;
        let a_work = self
            .parked
            .iter()
            .find(|work| work.id == from)
            .ok_or("the room being linked is missing")?;
        let mut a = a_work.draft.clone();
        let mut b = self.draft.clone();
        let mut exit = exit;
        // An exit placed earlier and not chosen is dropped.
        let mut forced = false;
        match connecting.placed {
            Some(placed) if placed == exit => forced = true,
            Some(placed) => {
                b.layout.exits.remove(placed);
                if exit > placed {
                    exit -= 1;
                }
            }
            None => {}
        }
        if exit >= b.layout.exits.len() {
            return Err(format!(
                "Select an exit in {} or place a new one first",
                self.title_of(to)
            ));
        }
        let two_way = connecting.two_way || forced;
        let into_b = arrival_next_to(&b.layout, exit)?;
        let into_a = if two_way {
            Some(arrival_next_to(&a.layout, a_exit)?)
        } else {
            None
        };
        let replaced = if two_way && !forced {
            Some(b.layout.exits[exit].target_room).filter(|old| *old != from)
        } else {
            None
        };
        a.layout.exits[a_exit].target_room = to;
        a.layout.exits[a_exit].spawn = into_b.into();
        if let Some(into_a) = into_a {
            b.layout.exits[exit].target_room = from;
            b.layout.exits[exit].spawn = into_a.into();
        }

        let from_before = link.origin.before.clone();
        let to_before = connecting.opened.clone();
        let (from_title, to_title) = (self.title_of(from), self.title_of(to));
        let old_title = replaced.map(|old| self.title_of(old));
        let arrow = if two_way { "<->" } else { "->" };

        // Everything above could be refused; from here on nothing can fail.
        self.draft = b;
        self.dirty = true;
        let index = self
            .parked
            .iter()
            .position(|work| work.id == from)
            .expect("checked above");
        let mut a_work = self.parked.remove(index);
        a_work.draft = a;
        a_work.dirty = true;
        let b_work = self.swap_work(a_work);
        self.parked.push(b_work);
        self.link = None;
        self.message = match old_title {
            Some(old) => format!(
                "Linked {from_title} {arrow} {to_title} ({to_title}'s exit no longer leads to {old})"
            ),
            None => format!("Linked {from_title} {arrow} {to_title}"),
        };
        Ok(Linked {
            from,
            to,
            two_way,
            replaced,
            from_before,
            to_before,
        })
    }

    /// Cancel, at whatever step the flow is in. From the name window it goes back to the
    /// chooser; anywhere else it ends the flow with every draft as it was before: B's
    /// changes are dropped (a room created by the flow is discarded) and A is back, with
    /// the exit that started the flow taken out again if the flow had placed it.
    pub fn link_cancel(&mut self) {
        let Some(link) = self.link.take() else {
            return;
        };
        match link.step {
            Step::Naming(_) => {
                self.link = Some(ExitLink {
                    step: Step::Choosing,
                    ..link
                });
                return;
            }
            Step::Connecting(connecting) => {
                if let Some(index) = self.parked.iter().position(|w| w.id == link.origin.room) {
                    let a = self.parked.remove(index);
                    // B, with whatever was placed in it, is dropped.
                    drop(self.swap_work(a));
                }
                if let Some(previous) = connecting.previous {
                    self.parked.push(previous);
                }
            }
            Step::Choosing => {}
        }
        if link.origin.placed {
            self.draft = link.origin.before;
            self.dirty = link.origin.was_dirty;
        }
        self.message = "Link cancelled".into();
    }

    /// "Linking A exit to B", while B is open.
    pub fn link_banner(&self) -> Option<String> {
        let link = self.link.as_ref()?;
        let Step::Connecting(connecting) = &link.step else {
            return None;
        };
        Some(format!(
            "Linking {} exit to {}",
            self.title_of(link.origin.room),
            self.title_of(connecting.target)
        ))
    }

    /// The exit placed in B and not connected yet, if any.
    pub fn link_placed_exit(&self) -> Option<usize> {
        match self.link.as_ref().map(|link| &link.step) {
            Some(Step::Connecting(c)) => c.placed,
            _ => None,
        }
    }

    /// Where the exit being linked leads now, by name ("Leads to ...").
    pub fn exit_leads_to(&self, exit: usize) -> Option<String> {
        let target = self.draft.layout.exits.get(exit)?.target_room;
        Some(self.title_of(target))
    }
}
