//! The room's name in the editor session: the text of the name field, the rules it is held
//! to, and the labels other rooms are listed by. The rules themselves are in `room_name`.
//!
//! The draft only ever holds a good name. Text that breaks a rule stays in `name_input`
//! with its error until it is fixed, and blocks Save; the draft keeps the last good name.
use crate::editor_session::EditorSession;
use crate::room::RoomId;
use crate::room_name::{self, NamedRoom};

impl EditorSession {
    /// Every room a name must not clash with: the saved rooms, this session's draft and the
    /// parked drafts. A
    /// room never clashes with itself (`room_name::check_unique` skips the same id), so the
    /// saved copy of the room being edited and its draft can both be here.
    fn named_rooms(&self) -> Vec<NamedRoom<'_>> {
        let mut rooms: Vec<NamedRoom> = self
            .rooms
            .iter()
            .map(|(id, layout)| (*id, layout.name.as_deref()))
            .collect();
        rooms.push((self.id, self.draft.layout.name.as_deref()));
        rooms.extend(
            self.parked
                .iter()
                .map(|work| (work.id, work.draft.layout.name.as_deref())),
        );
        rooms
    }

    /// Checks `text` as the name of room `id` against every other room; returns it trimmed.
    pub fn check_room_name(&self, id: RoomId, text: &str) -> Result<String, String> {
        room_name::check(id, text, self.named_rooms())
    }

    /// "Room N" for the next free N: the name a new room starts with.
    pub fn suggested_room_name(&self) -> String {
        room_name::next_default_name(self.named_rooms())
    }

    /// What this room is called, as the draft has it.
    pub fn room_title(&self) -> String {
        room_name::room_title(self.id, self.draft.layout.name.as_deref())
    }

    /// What room `id` is called, as the session has it (the room being edited by its draft).
    pub fn title_of(&self, id: RoomId) -> String {
        let name = if id == self.id {
            self.draft.layout.name.as_deref()
        } else if let Some(work) = self.parked.iter().find(|work| work.id == id) {
            work.draft.layout.name.as_deref()
        } else {
            self.rooms
                .iter()
                .find(|(room, _)| *room == id)
                .and_then(|(_, layout)| layout.name.as_deref())
        };
        room_name::room_title(id, name)
    }

    /// The room list's labels, in the order of `rooms`; the room being edited shows its
    /// draft's name.
    pub fn room_labels(&self) -> Vec<String> {
        self.rooms
            .iter()
            .map(|(id, layout)| {
                let name = if *id == self.id {
                    self.draft.layout.name.as_deref()
                } else {
                    layout.name.as_deref()
                };
                room_name::room_title(*id, name)
            })
            .collect()
    }

    /// The text for the name field: what is being typed, or the room's name.
    pub fn room_name_text(&self) -> String {
        self.name_input.clone().unwrap_or_else(|| self.room_title())
    }

    /// The name field's text changed. A good name goes into the draft (trimmed) and marks
    /// the session unsaved; a bad one is refused with the reason and the draft is untouched.
    /// Either way the text is kept so the field does not jump while it is typed.
    pub fn set_room_name_text(&mut self, text: &str) -> Result<(), String> {
        let checked = self.check_room_name(self.id, text);
        if let Ok(name) = &checked {
            if *name != self.room_title() {
                self.draft.layout.name = Some(name.clone());
                self.dirty = true;
            }
        }
        self.name_input = (text != self.room_title()).then(|| text.to_owned());
        checked.map(|_| ())
    }

    /// Why the name field's text is refused, if it is. Save waits until this is `None`.
    pub fn name_error(&self) -> Option<String> {
        let text = self.name_input.as_deref()?;
        self.check_room_name(self.id, text).err()
    }

    /// The field lost focus: good text is replaced by the (trimmed) name; bad text stays,
    /// with its error, until it is fixed.
    pub fn finish_name_edit(&mut self) {
        if self.name_error().is_none() {
            self.name_input = None;
        }
    }

    /// Whether the room can be saved under its name: the field has no refused text and the
    /// name, or the display name of an unnamed room, is still good and free. A room file
    /// edited by hand can break either rule.
    pub fn check_name_for_save(&self) -> Result<(), String> {
        if let Some(error) = self.name_error() {
            return Err(error);
        }
        self.check_room_name(self.id, &self.room_title())
            .map(|_| ())
    }
}
