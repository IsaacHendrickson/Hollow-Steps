//! Room names: the rules a name must meet, what a room shows when it has no name of its
//! own, the default for a new room, and the little state behind a "name this room" prompt.
//!
//! A name is stored in the room file as `name`. Rooms without one (the shipped rooms and
//! older files) are shown as `First`, `Second`, `Boss` or `Room N`, and those display names
//! count when checking that a name is unique, so a new name can never look like another room.
use crate::room::RoomId;

/// Most characters a name can have.
pub const MAX_NAME_LEN: usize = 32;

/// What a room is called when it has no name: `First`, `Second`, `Boss`, or `Room N` for
/// the room with file number `N - 1`.
pub fn default_display_name(id: RoomId) -> String {
    match id {
        RoomId::First => "First".into(),
        RoomId::Second => "Second".into(),
        RoomId::Boss => "Boss".into(),
        RoomId::Custom(index) => format!("Room {}", index + 1),
    }
}

/// The name a room is shown by: its own name, or its default display name.
pub fn room_title(id: RoomId, name: Option<&str>) -> String {
    name.map_or_else(|| default_display_name(id), str::to_owned)
}

fn allowed(c: char) -> bool {
    c.is_alphabetic() || c.is_ascii_digit() || matches!(c, ' ' | '-' | '_')
}

/// Checks the form of a name and returns it trimmed: 1 to `MAX_NAME_LEN` characters, only
/// letters, digits, spaces, `-` and `_`.
pub fn check_format(text: &str) -> Result<String, String> {
    let name = text.trim();
    if name.is_empty() {
        return Err("Room name cannot be empty".into());
    }
    if name.chars().count() > MAX_NAME_LEN {
        return Err(format!(
            "Room name can be at most {MAX_NAME_LEN} characters ({} typed)",
            name.chars().count()
        ));
    }
    if let Some(bad) = name.chars().find(|c| !allowed(*c)) {
        let shown = if bad.is_control() {
            "that character".to_owned()
        } else {
            format!("\"{bad}\"")
        };
        return Err(format!(
            "Room name cannot contain {shown}; use letters, digits, spaces, - and _"
        ));
    }
    Ok(name.to_owned())
}

/// A room as the checks see it: its id and its own name, if it has one.
pub type NamedRoom<'a> = (RoomId, Option<&'a str>);

/// Checks that no room other than `id` is shown by `name`, ignoring case. `rooms` is every
/// room and unsaved draft that could clash, with `id`'s own entry allowed to be among them.
pub fn check_unique<'a>(
    id: RoomId,
    name: &str,
    rooms: impl IntoIterator<Item = NamedRoom<'a>>,
) -> Result<(), String> {
    let wanted = name.to_lowercase();
    for (other, other_name) in rooms {
        let shown = room_title(other, other_name);
        if other != id && shown.to_lowercase() == wanted {
            return Err(format!("Another room is already called \"{shown}\""));
        }
    }
    Ok(())
}

/// The full check for a typed name: its form, then that it is unique. Returns the trimmed name.
pub fn check<'a>(
    id: RoomId,
    text: &str,
    rooms: impl IntoIterator<Item = NamedRoom<'a>>,
) -> Result<String, String> {
    let name = check_format(text)?;
    check_unique(id, &name, rooms)?;
    Ok(name)
}

/// `Room N` for the smallest `N` from 1 that no room is shown by, ignoring case.
pub fn next_default_name<'a>(rooms: impl IntoIterator<Item = NamedRoom<'a>>) -> String {
    let taken: Vec<String> = rooms
        .into_iter()
        .map(|(id, name)| room_title(id, name).to_lowercase())
        .collect();
    (1..)
        .map(|n| format!("Room {n}"))
        .find(|candidate| !taken.contains(&candidate.to_lowercase()))
        .expect("an unused number exists")
}

/// The text box of a "name this room" prompt: what is typed and why it was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamePrompt {
    pub text: String,
    pub error: Option<String>,
}

impl NamePrompt {
    /// A prompt that starts out with `suggestion` typed in.
    pub fn new(suggestion: String) -> Self {
        Self {
            text: suggestion,
            error: None,
        }
    }

    /// OK was pressed. `check` judges the text: a good name is returned (trimmed) and the
    /// prompt is done; a refusal is kept in `error` and the prompt stays open.
    pub fn confirm(
        &mut self,
        check: impl FnOnce(&str) -> Result<String, String>,
    ) -> Option<String> {
        match check(&self.text) {
            Ok(name) => {
                self.error = None;
                Some(name)
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }

    /// The error to show while typing: the text as it stands, judged by `check`.
    pub fn live_error(&self, check: impl FnOnce(&str) -> Result<String, String>) -> Option<String> {
        check(&self.text).err()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_and_unnamed_rooms_have_display_names() {
        assert_eq!(default_display_name(RoomId::First), "First");
        assert_eq!(default_display_name(RoomId::Second), "Second");
        assert_eq!(default_display_name(RoomId::Boss), "Boss");
        assert_eq!(default_display_name(RoomId::Custom(0)), "Room 1");
        assert_eq!(default_display_name(RoomId::Custom(6)), "Room 7");
        assert_eq!(room_title(RoomId::Custom(2), Some("Cellar")), "Cellar");
        assert_eq!(room_title(RoomId::Custom(2), None), "Room 3");
    }

    #[test]
    fn a_name_is_trimmed() {
        assert_eq!(check_format("  Crypt  ").unwrap(), "Crypt");
        assert_eq!(check_format("\tCrypt\n").unwrap(), "Crypt");
    }

    #[test]
    fn a_name_cannot_be_empty() {
        for text in ["", " ", "   \t "] {
            assert!(
                check_format(text).unwrap_err().contains("empty"),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_name_is_at_most_32_characters() {
        let longest = "a".repeat(MAX_NAME_LEN);
        assert_eq!(check_format(&longest).unwrap(), longest);
        let too_long = "a".repeat(MAX_NAME_LEN + 1);
        assert!(check_format(&too_long).unwrap_err().contains("32"));
        // The limit counts characters, not bytes, and applies after trimming.
        assert!(check_format(&"é".repeat(MAX_NAME_LEN)).is_ok());
        assert!(check_format(&format!("  {longest}  ")).is_ok());
    }

    #[test]
    fn a_name_allows_letters_digits_space_dash_and_underscore() {
        for text in [
            "Crypt",
            "Room 12",
            "boss-fight",
            "the_end",
            "A b-C_d 9",
            "Café",
        ] {
            assert_eq!(check_format(text).unwrap(), text, "{text:?}");
        }
    }

    #[test]
    fn other_characters_are_refused_and_named() {
        for bad in [
            ".", "/", "\\", ":", "*", "?", "\"", "<", "!", "é!", "a,b", "💀",
        ] {
            let error = check_format(&format!("Hall{bad}")).unwrap_err();
            assert!(error.contains("cannot contain"), "{bad:?}: {error}");
        }
        assert!(check_format("Hall.ron").unwrap_err().contains("\".\""));
        // Whitespace that is not a plain space is refused inside a name, and a control
        // character is described instead of printed.
        assert!(check_format("two\tparts").is_err());
        assert!(check_format("two\nparts")
            .unwrap_err()
            .contains("that character"));
    }

    #[test]
    fn names_must_be_unique_ignoring_case() {
        let rooms = [
            (RoomId::First, None),
            (RoomId::Custom(0), Some("Crypt")),
            (RoomId::Custom(1), None),
        ];
        let id = RoomId::Custom(5);
        assert!(check_unique(id, "Cellar", rooms).is_ok());
        for clash in ["Crypt", "crypt", "CRYPT", "First", "first", "ROOM 2"] {
            let error = check_unique(id, clash, rooms).unwrap_err();
            assert!(error.contains("already called"), "{clash}: {error}");
        }
    }

    #[test]
    fn a_room_can_keep_its_own_name() {
        let rooms = [(RoomId::Custom(0), Some("Crypt")), (RoomId::First, None)];
        assert!(check_unique(RoomId::Custom(0), "crypt", rooms).is_ok());
        assert!(check_unique(RoomId::First, "First", rooms).is_ok());
    }

    #[test]
    fn unnamed_rooms_count_by_their_display_names() {
        // Custom(2) is shown as "Room 3", so nothing else may take that name.
        let rooms = [(RoomId::Custom(2), None)];
        assert!(check_unique(RoomId::Custom(0), "Room 3", rooms).is_err());
        assert!(check_unique(RoomId::Custom(0), "Room 4", rooms).is_ok());
    }

    #[test]
    fn check_applies_format_before_uniqueness() {
        let rooms = [(RoomId::Custom(0), Some("Crypt"))];
        assert_eq!(check(RoomId::Boss, " Lair ", rooms).unwrap(), "Lair");
        assert!(check(RoomId::Boss, "Cr!pt", rooms)
            .unwrap_err()
            .contains("cannot contain"));
        assert!(check(RoomId::Boss, " crypt ", rooms)
            .unwrap_err()
            .contains("already called"));
    }

    #[test]
    fn the_default_name_is_the_next_free_room_number() {
        let none: [NamedRoom; 0] = [];
        assert_eq!(next_default_name(none), "Room 1");
        // An unnamed Custom(0) is already "Room 1".
        assert_eq!(next_default_name([(RoomId::Custom(0), None)]), "Room 2");
        let rooms = [
            (RoomId::Custom(0), Some("Crypt")),
            (RoomId::Custom(1), Some("room 1")),
            (RoomId::Custom(2), Some("ROOM 2")),
            (RoomId::Custom(3), Some("Room 4")),
        ];
        assert_eq!(next_default_name(rooms), "Room 3");
    }

    #[test]
    fn the_default_name_passes_every_rule() {
        let rooms = [(RoomId::Custom(0), None), (RoomId::Custom(1), None)];
        let name = next_default_name(rooms);
        assert_eq!(check(RoomId::Custom(2), &name, rooms).unwrap(), name);
    }

    #[test]
    fn a_prompt_returns_a_good_name_and_keeps_a_bad_one_open() {
        let rooms = [(RoomId::Custom(0), Some("Crypt"))];
        let judge = |text: &str| check(RoomId::Custom(1), text, rooms);
        let mut prompt = NamePrompt::new("Room 1".into());
        assert_eq!(prompt.confirm(judge), Some("Room 1".to_owned()));
        assert_eq!(prompt.error, None);

        prompt.text = "crypt".into();
        assert_eq!(prompt.confirm(judge), None);
        assert!(prompt.error.as_deref().unwrap().contains("already called"));
        assert_eq!(prompt.text, "crypt", "the text stays so it can be fixed");

        prompt.text = " Cellar ".into();
        assert_eq!(prompt.confirm(judge), Some("Cellar".to_owned()));
        assert_eq!(prompt.error, None);
    }

    #[test]
    fn a_prompt_reports_the_error_for_the_text_as_typed() {
        let judge = |text: &str| check_format(text);
        let mut prompt = NamePrompt::new(String::new());
        assert!(prompt.live_error(judge).unwrap().contains("empty"));
        prompt.text = "Hall".into();
        assert_eq!(prompt.live_error(judge), None);
    }
}
