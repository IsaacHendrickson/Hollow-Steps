## Keybinds: design note

The player's keys can be changed in the pause menu: `Esc`, **Settings**. Each action
(move left, move right, jump, attack, dash, heal, up, down, inventory) has a **primary**
key, which is always set, and an optional **alternate** key. Press a row's key button (or
select it and press Enter), then press the new key. Left/Right (or A/D) switch between a
row's two keys, **Delete** empties an alternate key, **Esc** cancels, and **Reset keybinds**
puts everything back on the shipped keys. Changes apply at once and are saved at once.

Shipped keys: move `A`/`Left` and `D`/`Right`, jump `Space`/`Z`, attack `X`, dash
`Left Shift`/`Right Shift`, heal `C`, up (pick up, strike up) `Up`, down (strike down)
`Down`, inventory `I`.

### Rules

- No key is ever bound twice. Pressing a key that is already in use takes it from there and
  the two slots trade keys (the message line under the list says so). The one exception: an
  empty alternate cannot take another action's primary key, because that would leave the
  other action without one; the message line says to change that key first.
- Keys that cannot be bound, on purpose: `Esc` (the pause menu), `Enter` and numpad `Enter`
  (the menu's confirm), `Backspace` and `Delete` (empty a key in the menu), `F1` to `F25`
  (editor, debug and reload keys), the Super keys, and a few keys with no stable meaning
  (Menu, Pause, Print Screen, Scroll Lock, Num Lock). The list is `BINDABLE` in
  `src/keybinds/keys.rs`, which also gives every key its one name.
- The pause menu itself always answers to Up/Down/W/S, Left/Right/A/D, Enter and `Esc`,
  whatever the keybinds are, so a bad binding can never lock the menu.
- The editor (F2) and the other function keys keep their fixed keys.

### Decision: where and how the bindings are saved

They are **per-user settings, not game content**, so they are not in `assets/` (which holds
the repository's data, and may be read-only next to an installed executable). The file is
`keybinds.ron` in the user's config folder:

| System  | Folder                                              |
|---------|-----------------------------------------------------|
| Windows | `%APPDATA%\HollowSteps`                             |
| macOS   | `~/Library/Application Support/HollowSteps`         |
| other   | `$XDG_CONFIG_HOME/hollow-steps` or `~/.config/hollow-steps` |

`HOLLOW_STEPS_CONFIG` (a folder) overrides all of these. No new dependency: the folder is
worked out from environment variables in `keybinds::config_dir`.

The format is RON, one list of key names per action (primary first, then the alternate),
with the names the settings screen shows:

    (
        bindings: {
            "attack": ["X"],
            "jump": ["Space", "Z"],
        },
    )

- Every change writes the whole file through a temporary file moved into place
  (`tuning::write_staged`), so a failed write leaves the old file; the folder is created
  when it is missing. A failed save keeps the change for this run and the message line says
  so.
- An action missing from the file keeps its shipped keys. A file that does not parse or
  breaks a rule (unknown action or key, no key, more than two, a key used twice, including
  one that collides with an action the file leaves out) is **not used**: the game starts
  on the shipped keys and shows the reason on screen like a failed reload. The bad file is
  left alone on disk until the player changes a key in the menu, which replaces it.
  Editing the file by hand takes a restart (F5 does not re-read it).
- **Reset keybinds** deletes the file instead of writing the shipped keys into it, so
  the shipped keys apply on the next run too, whatever a later version ships.
- Tests never touch a real config folder: `GameState` saves only after
  `use_keybinds_file(path)`, which `main.rs` calls with `keybinds::default_path()`; the
  tests use temporary folders.

### Where the code is

`src/keybinds.rs` (actions, slots, the rules above) with `keybinds/keys.rs` (the key
table), `keybinds/store.rs` (file format and config folder), `keybinds/input.rs` (keys to
`PlayerInput`, and the HUD's controls hint) and `keybinds/notes.rs` (the message line's
texts); `src/pause_menu.rs` and `pause_menu/layout.rs` (the Settings screen as pure state
and layout); `src/game/keybind_link.rs` (`GameState` owns the bindings and saves them);
`src/pause_input.rs` and `src/hud.rs` (keys, mouse and drawing).
