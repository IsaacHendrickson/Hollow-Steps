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
| Upgrade levels bought in the hub shop | Health and mana (full) |
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
| `shop` | none | Where the upgrade shop stands in the hub, as `(x, y)`, the center of its marker. Needs a `hub_room` and must lie inside that room's bounds (its edges count as inside); both are checked when the game loads. With none, there is no shop. |

`hub_room` and `shop` may be written plainly (`Custom(0)`, `(480.0, 436.0)`) or with `Some`. The
game refuses to load when a room named here does not exist, and says which file to add; a shop
outside the hub room is refused with the room's span in the message. `F5` does not re-read
`run.ron`: restart the game after editing it.

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
        upgrades: {
            "swift_feet": 1,
            "vitality": 2,
        },
    )

- `upgrades` maps an upgrade id to the level bought. It is left out of the file while empty, so
  a file with no upgrades looks as it did before, and a version-1 file without it still loads. An
  id that `upgrades.ron` no longer lists stays in the file (it has no effect and is saved back),
  so removing an upgrade from the catalog and adding it again loses nothing.
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

## Permanent upgrades and the shop

Banked coins buy **permanent upgrades**, bought once in the hub and kept in the profile.

- **The catalog** is `data/upgrades.ron`, embedded; an `assets/upgrades.ron` replaces it whole
  (`assets::read_override`), unknown fields are refused and it is validated when the game loads.
  An upgrade has an `id` (letters, digits, `_` and `-`, the same rule as charms), a `name`, a
  `description`, an `effect` and `costs`. `effect` is one of the charm effects (`MaxHealth(n)`,
  `MoveSpeedPercent(p)`, `SwordDamage(n)`) and is the amount **per level**. `costs` lists the price
  of each level in coins: 1 to 10 entries, each above 0, so the number of entries is the maximum
  level. The shipped list is Vitality (+10 max health, 30/60/100), Swift Feet (+5% move speed,
  40/80/140) and Keen Edge (+3 sword damage, 50/100/180).
- **The shop** is a marker at `run.ron`'s `shop` position in the hub. Up within pickup reach
  opens it (an item or charm in reach takes the press first, and the hub's exit doors wait for it).
  It opens only in the hub: never in a run, on the result screen or in a test room. While it is
  open the world stands still; Up/Down (or W/S) move the cursor, Enter or a click buys the next
  level of the selected upgrade, resting the mouse on a row selects it, and Escape closes it. Opening the
  editor closes it too.
- **A row** shows the name, the level ("2/3"), the price of the next level or "Max", and what one
  level gives ("+10 max health per level", the text the inventory uses for charms). Under the list
  are the selected upgrade's description and the result of the last purchase: what was bought,
  or why not ("Not enough coins: the Vitality costs 60, you need 20 more", "The Vitality is
  already at its maximum level (3)"). A refusal changes nothing. The banked coins are at the top.
- **A purchase** is paid from the banked coins and saved with the profile at once, and the new
  numbers are in effect at once: the player is not moved and nothing is refilled (a raised maximum
  health adds the same amount of health, as it does for a charm). If the profile cannot be saved,
  the purchase stands for this session and the message says the file was not written.
- **Layers.** The numbers in effect are the base (`player.ron` and `vitals.ron`) plus the
  permanent upgrades plus the equipped charms, put together only by `GameState::apply_stats`
  (`charms::effective_with`). The base files are never written with bonuses, so F4 and the editor's
  Player panel still show and save the base. Upgrades cost no mana. The bars measure against the
  baseline, so bought health looks like a charm's: the original part of the bar gets shorter and
  the health added by upgrades and charms follows it at the end, behind the divider. (The mana bar
  works the same way; no upgrade changes mana yet.)
- **With no hub** (the embedded `run.ron`) the profile is not used and no upgrade is in effect:
  the game plays as before.
- **F5** re-reads `upgrades.ron` with the other data files (a broken file changes nothing and
  shows the error). **F4's Reset profile** takes the bought levels away along with the coins, tier
  and clears, and the F4 **Coins** box sets the banked coins (0 to 999999) so the shop can be tried
  without playing for them.

## When coins are banked

At the **end** of a run, whatever the outcome: `floor(earned * coin_keep_percent / 100)` coins go
into the profile, all arithmetic saturating. Only a **clear** adds 1 to the tier and to the cleared
count; dying and abandoning change neither. Coins collected during a run are counted for the run
(`GameState::coin_count`), not for the profile, until then. Closing the game (the pause menu's Exit
or the window's close button) banks a running run as Abandoned first. There is no saving of a run
in progress.

## The hub invariant

**Outside the result screen, the player is in the Hub phase exactly when the current room is the hub
room.** (On the result screen after Abandoned by an exit, the room is the hub and the phase is Result.)

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

## What the player sees

- **Pause menu.** Only during a run (not in the hub, not in a test room, not with no hub) the main
  screen lists Unpause, Settings, Abandon run, Exit. Abandon run opens a confirm screen (Abandon
  run / Keep playing, with Keep playing selected so a stray Enter keeps the run); Escape goes back
  one screen. `PauseOutcome::AbandonRun` is handled by `GameState` like Reset keybinds, so the front
  end only ever sees `Exit`.
- **Result screen.** Title (Dungeon cleared / You died / Run abandoned), coins earned, coins
  banked, total banked, and after a clear "Difficulty tier a -> b". A message about a profile that
  did not save shows under the lines. Enter or a left click leaves it. The text and layout are pure
  (`run/screen.rs`); `hud_run.rs` draws them and `main.rs` only passes on the keys and the click.
- **Counters.** With no hub the HUD counter is the coin count as always. In the hub it shows the
  banked coins with the tier; in a run, this run's coins. The inventory shows both.
- **Exit and the close button.** Both call `abandon_run_on_exit()` first, which ends a test room
  and banks a run in progress as Abandoned. The close button is intercepted with macroquad's
  `prevent_quit` / `is_quit_requested`, set only after start-up so the load-error screen can still
  be closed.

## Known limits

Effect kinds other than max health, move speed and sword damage (max mana, a coin bonus), refunds
or selling an upgrade back, an editor panel for the upgrades (the next task,
`feature/upgrade-editor`; until then edit `assets/upgrades.ron`), placing the shop in the editor
(it is a position in `run.ron`), a delay before the result screen, saving a run in progress,
editor controls for `run.ron` (it holds run rules, not player feel), per-dungeon settings, and a
hard stop of the program (power loss, a crash) not banking the run.

## Where the code is

`src/profile.rs` (the profile and its file), `src/run.rs` (`RunConfig`, `RunOutcome`,
`RunResult`, `RunPhase`, `Profile::bank`), `src/upgrades.rs` (the catalog, `buy`,
`permanent_effects`), `src/shop_menu.rs` (the shop's cursor, rows and layout),
`src/game/shop_link.rs` (`GameState` owns the shop), `src/shop_input.rs`, `src/hud_shop.rs` and
`src/scene_shop.rs` (the window, the keys and mouse, the marker), `src/game/run_link.rs`
(`GameState` owns the phase and the profile), `src/game/world_load.rs` (the one world loader), `src/player/run_reset.rs` and
`src/vitals/fill.rs` (the player and vitals reset), `src/run/screen.rs` (result screen text and layout, counter captions), `src/hud_run.rs` (drawing
them and the key or click that leaves the result screen), `src/tuning_panel/run.rs` (F4 debug section). Tests: `src/game/run_tests.rs` (the loop),
`src/game/run_profile_tests.rs` (profile, test rooms, editor, no hub), `src/upgrades/tests.rs` and
`src/game/upgrade_tests.rs` (the rules and the layered stats), `src/shop_menu/tests.rs` and
`src/game/shop_tests.rs` (the shop).
