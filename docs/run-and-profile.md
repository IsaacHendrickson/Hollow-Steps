# Run and profile: design note

The game has a hub-and-run loop. The **hub** is a safe room. Leaving it through an exit starts a
**run**. A run ends on death, when the boss of the final boss room is defeated, or on Abandon.
A result screen follows, and Enter returns to the hub. The plan behind it is
`claude/design-roguelike-metroidvania.md` (project doc).

The loop is **off** unless `run.ron` names a hub. The embedded default has none, so the game
plays as it always did (death respawns in place) and every fixture and test is unchanged. The
shipped `assets/run.ron` turns it on.

## What is kept and what is lost

| Kept forever (the profile, `profile.ron`) | Reset when a run starts or ends |
|---|---|
| Coins banked | Owned and equipped charms (back to the `owned_at_start` ones) |
| Difficulty tier | Collected items (the Dash too) and the items taken from rooms |
| Dungeons cleared | This run's coin count |
| | Health and mana (full) |
| | Every room, rebuilt from its file: enemies, boss, coins, items, charms lying |

Nothing layered is ever written back to the data files.

## The settings: `data/run.ron`

Embedded default; an `assets/run.ron` replaces it whole (`assets::read_override`). Unknown
fields are refused, and every field is validated when the game loads.

| Field | Default | Meaning |
|---|---|---|
| `hub_room` | none | The safe room. With none, there is no hub, no run and no profile use. |
| `start_room` | `First` | Where the game begins when there is no hub; with a hub, the room its exit leads to. Must differ from `hub_room`. |
| `final_boss_room` | `Boss` | Defeating the boss of this room clears the dungeon. |
| `coin_keep_percent` | `100` | Share of a run's coins banked when it ends, 0 to 100 (finite). |

`hub_room` may be written `Custom(0)` or `Some(Custom(0))`. The game refuses to load when a
room named here does not exist, and says which file to add. `F5` does not re-read `run.ron`:
restart the game after editing it.

## The profile file

**Decision:** the profile is a per-user save, not game content, so it follows the keybinds
pattern (`docs/keybinds.md`): `profile.ron` in the user's config folder, next to `keybinds.ron`.
The folder comes from `keybinds::default_dir` (the same lookup, not a copy): `%APPDATA%\HollowSteps`
on Windows, `~/Library/Application Support/HollowSteps` on macOS, otherwise
`$XDG_CONFIG_HOME/hollow-steps` or `~/.config/hollow-steps`. `HOLLOW_STEPS_CONFIG` (a folder)
overrides all of them. No new dependency.

The format is RON with a `version` field:

    (
        version: 1,
        coins: 120,
        tier: 2,
        cleared: 2,
    )

- Unknown fields are refused (a typo is found, not ignored). A new field arrives with
  `#[serde(default)]` so version-1 files keep loading; the version changes only when the meaning
  of an existing field does.
- The version is read first: a file from a newer game says "unknown profile version N", not
  "unknown field".
- Every save writes the whole file through a temporary file moved into place
  (`tuning::write_staged`), so a failed write leaves the old file. The folder is created when it
  is missing.
- A missing file is a fresh profile (nothing is written until the first run ends).
- A file that cannot be used (it does not parse, has an unknown version or an unknown or missing
  field, is not text, or cannot be read) is **moved to `profile.ron.bak`**, replacing an older
  backup, never deleted or overwritten. The game starts on a fresh profile and shows the reason on
  screen like a failed reload. If the move itself fails, the message says so.
- **Saving never happens before loading:** `GameState` learns the path only from
  `use_profile_file(path)`, which loads first. Tests never touch a real config folder because they
  never call it with one; `main.rs` calls it with `profile::default_path()`.
- A failed save keeps the banked coins in memory for this session and shows a message; the next
  bank tries again.

## When coins are banked

At the **end** of a run, whatever the outcome: `floor(earned * coin_keep_percent / 100)` coins go
into the profile, all arithmetic saturating. Only a **clear** adds 1 to the tier and to the cleared
count; dying and abandoning change neither. Coins collected during a run are counted for the run
(`GameState::coin_count`), not for the profile, until then. Closing the game (the pause menu's Exit
or the window's close button) banks a running run as Abandoned first. There is no saving of a run
in progress.

## The hub invariant

**The player is in the Hub phase exactly when the current room is the hub room.**

- Hub to any other room by an exit: a run starts (state reset as in the table).
- Any room to the hub by an exit during a run: the run is Abandoned and banks as above.
- Death outside a test room: Died. The boss of `final_boss_room` falling outside a test room:
  Cleared. `abandon_run()`: Abandoned. Each shows the result screen (the world is frozen,
  Escape does nothing) until Enter or a click calls `continue_after_result()`, which puts the player
  in the hub with full health and mana and the run state reset.
- A run start and a continue rebuild every room from its file with the same loader that start-up
  and F5 use. If that fails (a room file was edited into an invalid state), nothing changes: the
  player cannot leave the hub, or stays on the result screen, and a message says why.
- The editor can move the player to another room (Open room, or Save of a room): the next tick
  after it closes restores the invariant the same way an exit would, so opening the first room from
  the hub starts a run, and opening the hub during a run abandons it. A **test room** is a sandbox:
  death there respawns as before, no run starts or ends, a boss defeated there clears nothing, the
  profile is never touched, and its exits do nothing.

## Known limits

Upgrades (the next task), a delay before the result screen, saving a run in progress, placing hub
furniture in the editor, editor controls for `run.ron` (it holds run rules, not player feel),
per-dungeon settings, and a hard stop of the program (power loss, a crash) not banking the run.

## Where the code is

`src/profile.rs` (the profile and its file), `src/run.rs` (`RunConfig`, `RunOutcome`,
`RunResult`, `RunPhase`, `Profile::bank`), `src/game/run_link.rs` (`GameState` owns the phase and
the profile), `src/game/world_load.rs` (the one world loader), `src/hud_run.rs` (result screen,
hub and run counters), `src/tuning_panel/run.rs` (F4 debug section).
