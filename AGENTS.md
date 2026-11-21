# AGENTS.md

## Project purpose

Hollow Steps is a Rust 2D action-platformer prototype. It is inspired by the
exploration, movement, and atmosphere of Hollow Knight, but all code, names,
art, audio, and story must remain original. Keep the project small and
modular so the prototype can grow into rooms, enemies, combat, abilities,
save data, menus, and content tools without turning the main loop into a
single large system.

## Working efficiently (read first)

The project owner wants features added with as few tokens as possible.

- Reference notes (bodies, bosses, rooms, enemy data, editor) are in `docs/`;
  open one only when the task needs it. `README.md` is a short overview.
- Read this file, then open only the files relevant to the task. Do not read
  or re-print the whole repository.
- Edit files in place in the repository. Do not produce `.patch` files or
  paste full files into chat unless explicitly asked.
- When changing an existing file, make targeted edits. Never rewrite a whole
  file to change a few lines.
- One small feature or fix per task. If a request is large, do the smallest
  playable slice first and stop.
- Do not explain code that was not asked about. No tutorials, no restating
  the request, no summaries of unchanged files.
- Keep the final handoff short (see "Handoff format").
- Do not add placeholder systems, speculative abstractions, or unrelated
  refactors.

## Toolchain and commands

Use the stable Rust toolchain and Cargo.

Run these yourself before handing off a change. Do not ask the owner to run
them if you have a shell available:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

To try the game locally (owner runs this, since it opens a window):

```text
cargo run
```

If a command fails, read the error, fix the cause, and rerun until it passes.
Do not hand back code with known compile errors, test failures, or Clippy
warnings.

If a command cannot run because the environment lacks Rust or Cargo, say so
clearly in the handoff and list the exact commands the owner should run. Never
describe an unexecuted check as passing.

## Architecture

The repository has three layers: gameplay modules, the game state that
composes them, and the Macroquad front end.

Details of every module: docs/modules.md (open it only when the task needs it).

Gameplay modules (pure rules, no rendering):

- `src/player.rs`: player input, movement (including dash), collision resolution
- `src/health.rs`: player health, damage, invincibility, `set_limits`
- `src/enemy.rs` (+ `enemy/`): enemy behavior, health, knockback, dying state, bodies
- `src/pose.rs`: semantic pose assets, playback, validation
- `src/body.rs` (+ `body/`): validated RON body templates, point springs, palettes, transforms
- `src/body_debug.rs`: pure body overlay primitives, nearest-entity selection
- `src/visual_box.rs`: drawn box of an enemy or boss apart from its hitbox (gameplay reads only the hitbox)
- `src/action_runtime.rs` (+ `action_runtime/`): shared action execution; read actions by reference, never clone
- `src/assets.rs`: assets folder resolution, load-failure text, `read_override` for `data/*.ron`
- `src/behavior.rs` (+ `behavior/`): data-driven action sequences, state transitions, RON validation
- `src/enemy_attack.rs` (+ `enemy_attack/`): shared attack geometry, timed enemy attacks, projectiles
- `src/preset.rs`: validated RON behavior preset catalog.
- `src/action_parameters.rs`: labeled scalar editor fields for the action registry
- `src/behavior_values.rs`: numeric behavior controls, transactional validated edits
- `src/boss_definition.rs`: validated RON boss definitions, attack selection settings
- `src/boss_editor.rs`: pure boss property buffers, ordered attack/action edits
- `src/boss_special.rs`: boss sequence cursors, selection, effects, coin drops
- `src/boss_special/body_link.rs`: `BossBody`: a boss driving its optional body; hitbox stays `size`
- `src/combat.rs`: melee swings (forward, up, down), per-direction hitboxes, pogo, `MeleeConfig`
- `src/damage.rs`: `DamageResolver`: all enemy and boss damage into the one hit on the player
- `src/events.rs`: `GameEvent`, `EventQueue`, `take_events` / `take_located_events`
- `src/sound.rs`: validated event-to-sound cues (`data/audio.ron`), sound-file lookup, play plan
- `src/particles.rs`: burst presets (`data/particles.ron`), seeded generator, capped live list
- `src/feel.rs`: hit-stop and screen shake from `GameEvent`s (`data/feel.ron`)
- `src/background.rs`: parallax background layers per room, validation, `parallax_offset`
- `src/postfx.rs`: vignette and glow settings (`data/postfx.ron`), shader source, pure formulas
- `src/player_body.rs`: the player's look (`data/player_body.ron`), pose choice, `BodyInstance`
- `src/item.rs`: world items and pickup (`within_reach`, shared with charms)
- `src/coin.rs`: collectible coins.
- `src/profile.rs`: the saved profile (coins, tier, cleared, upgrades): `profile.ron` in the config folder, staged save, bad file kept as `.bak`
- `src/run.rs`: run loop rules: `RunConfig` (`data/run.ron`, incl. `shop`), `RunOutcome`, `RunResult`, `RunPhase`, `Profile::bank`; `docs/run-and-profile.md`
- `src/run/screen.rs`: pure result screen text and layout, HUD and inventory coin captions (`run::result_lines`, `hud_counter`)
- `src/upgrades.rs`: permanent upgrades (`data/upgrades.ron`): `UpgradeCatalog`, `buy`, `permanent_effects`; levels live in the profile
- `src/upgrade_editor.rs`: pure `UpgradeDraft` editing `upgrades.ron`, field table, validate-first save; `docs/upgrade-editor.md`
- `src/snapshot_history.rs`: generic snapshot undo/redo for a panel's own list (the Upgrades panel's)
- `src/shop_menu.rs` (+ `shop_menu/`): hub upgrade shop as pure state and layout; `game/shop_link.rs` owns it; `shop_input`, `hud_shop`, `scene_shop` draw it
- `src/room.rs`: rooms (with `terrain_style` and lying `charms`), exits, room transitions
- `src/door_exit.rs`: door exits (a free rectangle, Up to enter): room-file form, checks, editor add/move/resize; `docs/door-exits.md`
- `src/ability.rs`, `src/exit_lock.rs`: abilities and keys as run state, locked exits, notice; `docs/keys-and-locks.md`
- `src/reach.rs`: reach envelope (jump height, longest gap, with and without Dash) measured on the real `Player`; `Interval`
- `src/gates.rs`: gate marks (entrance to exit needs an ability), play never reads them; editor `gate_panel`; `docs/gates.md`
- `src/dungeon_check.rs`: Check dungeon: unreachable rooms, locks, key counts, ability gates, traps, orphan gates; pure; `docs/dungeon-check.md`
- `src/terrain.rs`: solid rectangles, axis-separated substepped collision, validation
- `src/terrain_style.rs` (+ `terrain_style/`): procedural look of solids as a draw list, never collision
- `src/terrain_styles.rs`: terrain look presets (`data/terrain_styles.ron`), `RoomStyle`
- `src/game/terrain_link.rs`: loads terrain presets with the rooms; `GameState::terrain_styles`
- `src/camera.rs` (+ `src/game/camera_bounds.rs`): follow camera, room clamp, shake, letterbox, HUD anchors
- `src/timestep.rs`: fixed 120 Hz step accumulator (capped), edge-input latch
- `src/pause_menu.rs` (+ `pause_menu/`): pause menu as pure state, with its layout
- `src/keybinds.rs` (+ `keybinds/`): rebindable controls as pure state; `docs/keybinds.md`
- `src/tuning.rs`: player, melee, strike, knockback tunables (`data/player.ron`), staged save
- `src/tuning_fields.rs`: editable `player.ron` numbers, one range per field
- `src/player_editor.rs`: pure `PlayerDraft` editing `player.ron` and `vitals.ron`; max mana must cover saved charm costs (`CharmCost`)
- `src/player_history.rs`: the Player panel's own undo/redo
- `src/player_look_editor.rs`: pure `PlayerLookDraft` editing `player_body.ron`, preview
- `src/charms.rs`: charm rules: `Charm`, `CharmCatalog`, `CharmState`, `effective`
- `src/vitals.rs`: pure vitals (health, trailing bar, mana, hold-to-heal); `data/vitals.ron`
- `src/editor.rs`: pure room editing, validation, serialization, and file save boundary
- `src/editor_files.rs`: room file names (a bad `room_*.ron` name errors with the fix), staged room save, loading extra rooms
- `src/room_charms.rs`: charms lying in rooms: placements, pickup rule, charm checks
- `src/editor_charms.rs`: `placement_catalog` (with "Charm: <name>") and `charm_swatch`
- `src/charm_editor.rs`: pure `CharmDraft` editing `charms.ron`, validate-first save
- `src/charm_history.rs`: the Charms panel's own undo/redo
- `src/editor_session.rs`: `EditorSession`: open draft, definitions, unsaved flag, parked drafts
- `src/editor_look.rs`: pure terrain look edits on the room draft, `LookPreview`
- `src/editor_guard.rs`: unsaved-changes state machine (Save / Discard / Cancel)
- `src/room_name.rs`: room name rules, display names, `NamePrompt`
- `src/editor_name.rs`: the room name in `EditorSession` (set, check, suggest, labels)
- `src/exit_link.rs` (+ `src/exit_arrival.rs`): exit-link flow as a pure state machine
- `src/editor_inbound.rs` (+ `src/editor_selection.rs`): other rooms' arrivals here; canvas `Selection`
- `src/editor_history.rs`: snapshot undo/redo for the room draft, Ctrl+Z / Ctrl+Y keys
- `src/editor_terrain.rs`: pure terrain edits on the room draft (add, move, resize, delete), validated
- `src/editor_tools.rs`: editor tool enum (Select, Place, Terrain, Erase), key map, terrain drag state
- `src/editor_view.rs`: pure editor canvas math: zoom/pan, grid snapping, window regions
- `src/room_size.rs`: validated room bounds/floor edits for the editor
- `src/definition_editor.rs` (+ `definition_editor/`): pure entity/behavior edits, save planning, staged writes
- `src/search.rs`: pure editor list filtering and stable selection

Composition and front end:

- `src/body_draw.rs`: Macroquad adapter: `draw_body`, `body_tint`, legacy enemy boxes, debug overlays
- `src/game/pause_link.rs`: `GameState` owns the `PauseMenu`: `press_escape`, `is_paused`, `pause_*` accessors
- `src/game/keybind_link.rs`: `GameState` owns the `Keybinds`: file load/save, bind, clear, reset
- `src/charm_menu.rs` (+ `charm_menu/`): inventory Charms section as pure state, layout, text fitting
- `src/game/charm_link.rs`: `GameState` owns charms; `toggle_charm`, `apply_stats` (base plus charms)
- `src/game/vitals_link.rs`: `GameState` drives vitals: heal input gating, F4 / F5 accessors
- `src/game/player_link.rs`: `GameState` owns the one `PlayerDraft` F4 and the Player panel share
- `src/game/test_room.rs`: Test room: plays the editor draft as a room, restores editor and player after
- `src/game.rs`: gameplay state and world composition; add systems as modules or fields, testable without a window
- `src/editor_panel.rs` (+ `editor_panel/`): egui editor panel frame, one sub-module per panel
- `src/editor_ui.rs` (+ `editor_ui/`): editor window state (`EditorUi`) and canvas frame (`draw_editor`)
- `src/editor_panel/room_name_field.rs` (+ `name_dialog`, `link_dialog`, `link_panel`): room name, New room, exit-link UI
- `src/game/editor_save.rs`: `GameState::save_editor`: checks, staged write, all rooms or none
- `src/editor_visuals.rs`: editor canvas enemy, boss and player pictures
- `src/terrain_canvas.rs`: Macroquad drawing of solids' outlines and handles, terrain tool mouse glue
- `src/terrain_draw.rs`: Macroquad adapter drawing a terrain draw list (world or editor canvas)
- `src/charm_panel.rs` (+ `charm_panel/`): the editor's Charms panel
- `src/game/charm_editor_link.rs`: `GameState` holds the Charms panel's edits: sync, save, discard
- `src/upgrade_panel.rs` (+ `upgrade_panel/`): the editor's Upgrades panel; `src/game/upgrade_editor_link.rs` holds its edits: sync, save, discard
- `src/game/world_charms.rs`: charms lying in rooms, in play and in Save: pickup, validation
- `src/game/run_link.rs`: `GameState` owns the hub-and-run loop: phase, profile, start/end a run, result screen, `continue_after_result`
- `src/game/world_load.rs`: the one world loader (rooms, terrain looks) for start-up, F5 and a run's reset
- `src/game/world_doors.rs`: `door_in_reach`, the one rule for entering a door and for showing its prompt
- `src/game/world_items.rs`: items in play and Save: pickup, taken items kept for the run (`TakenItems`), restored on rebuild
- `src/player_panel.rs` (+ `player_panel/`): the editor's Player panel; `draw_field` is shared with F4
- `src/tuning_panel.rs`: F4 egui window: a drag box per `player.ron` and `vitals.ron` field
- `src/tuning_panel/run.rs`: F4 Run (debug) section: Coins and tier boxes and Reset profile, saved at once
- `src/scene.rs`: Macroquad drawing of the world (background to player)
- `src/postfx_pass.rs`: Macroquad post-process pass (material, world target, HUD target)
- `src/effects.rs`: Macroquad event-driven effects: particles and sounds (`docs/audio.md`)
- `src/hud_bars.rs` (+ `hud_bars/`): pure health and mana bar layout (`HudConfig`, `layout_bars`)
- `src/hud_bars_draw.rs`: Macroquad drawing of those rectangles (`draw_bars`); no rules
- `src/hud_damage.rs`: pure health-bar damage feedback: `BarShake`, `HudFeedback`
- `src/hud_layout.rs`: pure HUD rules: boss bar fill and caption, room label (health and mana bars are in `hud_bars`)
- `src/hud_run.rs`: Macroquad result screen and the Enter or click that leaves it; HUD and inventory coin counters
- `src/hud.rs`: Macroquad HUD, inventory, pause menu, reload panels (virtual-resolution space)
- `src/charm_input.rs`: Macroquad adapter for the charm list while the inventory is open
- `src/hud_charms.rs`: Macroquad drawing of the inventory's Charms section
- `src/pause_input.rs`: Macroquad adapter for the open pause menu (keys, mouse, key binding)
- `src/startup.rs`: Macroquad startup: loads the game, shows load errors (F5 retries)
- `src/main.rs`: Macroquad window, key translation, drawing, frame scheduling; keep thin, no gameplay rules

The library target (`src/lib.rs`) exposes reusable gameplay modules. Integration
tests live in `tests/` and verify behavior through the public API.

When you add, rename, or remove a module, change ONE line here (160 characters at most) and put anything longer in docs/modules.md or the matching docs/*.md.

Limits, checked by `tests/docs_size.rs`: a module line here or a line of `FEATURES.md` over 200 characters fails the test, and so does this file over 24 KB (24576 bytes).

## Coordinate and simulation conventions

- Positions are the center of an entity's rectangle.
- The x axis points right and the y axis points down.
- Positive gravity increases downward velocity.
- `WorldBounds::floor_y` is the top edge of the floor.
- Player input is frame intent. `jump_pressed` is an edge event and must come
  from an API such as `is_key_pressed`, not a held-key query.
- Update methods receive `delta_seconds`; they must not read wall-clock time
  directly. `main.rs` calls `GameState::update` once per fixed tick (1/120 s,
  `timestep.rs`), never with the raw frame time.
- Clamp or validate external values at system boundaries. Do not scatter
  magic numbers through the rendering loop.
- Draw code never uses fixed pixel positions or window-size constants. Use the
  camera and the virtual resolution only. The world and HUD render to a 960x540
  render target (`VIEW_WIDTH`/`VIEW_HEIGHT`) that is letterboxed into the window;
  the world through the follow camera, the HUD anchored with `camera::anchored`.

## Roadmap

Work in this priority order; do not keep adding enemy-action parameters while
earlier items are missing:

1. Terrain collision (platforms, walls, ceilings)
2. Camera (follow camera, virtual resolution)
3. Fixed timestep
4. Editor tools (terrain painting, zoom/pan/grid, undo, play-from-here)
5. Player feel (acceleration, up/down strikes, pogo, hit-stop, shake)
6. Visuals (player body, HUD, particles, atmosphere)
7. Audio

## Tunable values

Tunable numbers live in data files, as named and validated parameters. Do not
hardcode values that affect how an entity, boss, attack, action or the player
behaves (distances, speeds, durations, damage, health, counts, radii, etc.) in
Rust, apart from engine-level constants such as gravity.

Editor controls are required for enemy, boss, attack and player-feel values.
Numeric values use sliders (`DragValue` with a clamped range and units), not
text boxes; text boxes are for names only. When adding an action, condition or
entity type, add its parameters to the editor's catalogs in the same change.

## Change workflow

1. Make one small gameplay change at a time.
2. Add or update a focused unit/integration test for the behavior.
3. Run formatting, tests, and Clippy, and fix any failures.
4. Only then connect the behavior to rendering or input.
5. Keep a change focused. A new mechanic should not also reorganize unrelated
   modules or add placeholder systems.

When adding a system, prefer this shape:

```text
src/<system>.rs       pure state and rules, with unit tests
src/game.rs            owns and updates the system
src/main.rs            translates input and renders the result
tests/<system>.rs      public behavior or cross-system tests
```

## Branch and sync workflow

Do this for every task, without being asked. Work is not finished until it is
pushed to GitHub.

1. Work on a feature branch named `feature/<short-name>`. Never commit
   directly to `main`. If the task continues an existing feature branch, use
   that branch.
2. Commit your changes to that branch.
3. Merge the latest `main` into the working branch.
4. If the merge conflicts, resolve it keeping both sets of changes. Do not
   blindly take one side (for example with `-X ours` or `-X theirs`), and do
   not drop code from either side. If the correct result is unclear, stop and
   explain the conflict in the handoff instead of guessing.
5. After merging, run `cargo fmt`, `cargo test`, and
   `cargo clippy --all-targets --all-features -- -D warnings`, and fix any
   failures.
6. Push the working branch to `origin`.
7. If the push fails (for example because of network or permission limits),
   say so in the handoff and include the exact branch name so the owner can
   finish it by hand. If a pull request cannot be created, give the
   `https://github.com/<owner>/<repo>/pull/new/<branch>` link instead.

## Keeping existing features intact

Features have disappeared before because a branch was built from an older
`main` or a merge dropped one side. Prevent this:

- Keep `FEATURES.md` in the repository root: one line per feature, naming the
  module and test file that cover it. Create it if it is missing. Update it in
  the same change that adds, changes, or removes a feature.
- Before finishing a task, read `FEATURES.md` and confirm every listed feature
  still exists in the code and its tests still run and pass.
- When resolving a merge conflict, never remove code or tests that belong to
  another feature. After merging, the test count must not drop unless a test
  was deliberately removed and the handoff says why.
- If `main` is missing something the task expects (for example, a feature
  that exists only on another branch), stop and say so in the handoff instead
  of rebuilding it.

## Testing

Agents should be able to verify almost everything without a window.

- All gameplay rules (movement, jumping, dashing, collisions, damage, room
  transitions, save data) live in pure modules that take explicit input and
  `delta_seconds` and return or mutate plain state.
- Tests must never start Macroquad, open a window, or need a GPU, display, or
  audio device.
- Tests must not compare path strings that contain separators. Build paths
  one component at a time with `Path::join`, compare `Path` values, components,
  or filenames, and keep assertions that errors identify the correct file.
- Every new mechanic needs tests for: the normal case, the boundary or edge
  case (world edges, floor, zero or very large `delta_seconds`), and at least
  one case that must not happen (for example, jumping while airborne).
- Tests must not depend on editable files in `assets/`. Use rooms built in
  code or fixed fixtures under `tests/fixtures/`, including enemy, behavior,
  body and pose definitions. The only place that reads `assets/` is
  `tests/shipped_content.rs`: one test per area that loads and validates what
  ships and asserts no numbers, names, layouts or coordinates (any new
  shipped-content check goes there). A test of what an enemy, room or body
  does uses a fixture, never the shipped file.
- Prefer deterministic tests: fixed `delta_seconds`, stepped frame by frame,
  asserting on positions and velocities. No randomness or wall-clock time.
- When fixing a bug, first add a test that reproduces it, then fix it.
- Do not weaken or delete an existing test to make a change pass. If a test
  is wrong, say why in the handoff.

What agents cannot verify: how movement feels, how the game looks, and whether
rendering is correct. For anything touching `src/main.rs` or visuals, list a
short manual check for the owner in the handoff (for example, "run the game,
press Space, confirm the player jumps and lands on the floor").

## Design guidelines

- Enemy behaviors must remain fully editable in the in-game editor. Expose
  every action, state, transition, assignment, and authored value through labeled
  controls, including detection/activation ranges, flee/chase start and stop
  distances, speeds, durations, and health thresholds. New behavior parameters
  must ship with editor controls, validation, save/reload support, and focused
  tests; do not hide tuning values in gameplay code or require raw RON edits.
- New enemies must start with their own blank behavior, never inherit the
  first catalog entry or silently share another enemy's behavior. Keep explicit
  assignment, preset creation, and shared-behavior copying available so authors
  can build enemies from scratch and edit them independently.
- Prefer small structs with explicit ownership and methods with one job.
- Keep constants/configuration near the system that uses them; promote shared
  values only when multiple systems genuinely need them.
- Avoid global mutable state and hidden singletons.
- Keep third-party dependencies limited and justify each new dependency in
  the change description.
- Separate simulation from presentation so tests do not need a graphics
  window.
- Use original names and assets. Do not copy Hollow Knight code or proprietary
  assets.
- If a requested feature needs a choice that changes the architecture (for
  example, a physics library, ECS, or save format), document the decision in
  a file in `docs/` or a short design note before implementing it.
- Keep files focused and reasonably small so future agents only need to read
  the module they are changing. Any file over ~600 lines must be split before
  adding to it.

## Handoff format

End each task with a brief message in this shape and nothing more:

```text
Changed: <files>
Checked: <commands run and whether each passed>
Not checked: <anything that could not be run, or needs a manual playtest>
Branch: <branch name> (pushed: yes/no, merged main: yes/no)
```

## First-slice invariants

The initial player implementation must preserve these rules:

- The player cannot move outside the horizontal world bounds.
- The player can jump only while grounded.
- A jump starts with upward velocity and gravity brings the player back down.
- Landing places the player exactly on the floor and clears downward velocity.
- Movement and jump behavior can be tested without starting Macroquad.
