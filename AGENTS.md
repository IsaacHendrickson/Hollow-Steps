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

- `src/player.rs`, `player/run_reset`, `health`, `vitals`, `vitals/fill`, `combat`, `damage`: movement, dash, collision; health, mana; melee; one damage path
- `src/player/air_jump.rs`: Double Jump's air jump, one per flight, refilled on landing; `double_jump_speed` in player.ron (Player panel, F4)
- `src/tuning.rs`, `tuning_fields`, `player_editor`, `player_history`, `player_look_editor`, `player_body`: player.ron, vitals.ron values, drafts, undo, look
- `src/enemy.rs` (+ `enemy/`), `pose`, `body` (+ `body/`), `body_debug`, `visual_box`: enemies, poses, RON body templates, overlays, drawn box vs hitbox
- `src/behavior.rs` (+ `behavior/`), `action_runtime` (+ `action_runtime/`), `preset`, `action_parameters`, `behavior_values`: data-driven actions, presets
- `src/enemy_attack.rs` (+ `enemy_attack/`), `boss_definition`, `boss_editor`, `boss_special`, `boss_special/body_link`: attacks, projectiles, bosses, editing
- `src/assets.rs`, `events`, `timestep`, `camera`, `game/camera_bounds`, `search`: asset paths, `GameEvent` queue, fixed 120 Hz step, follow camera, list filter
- `src/sound.rs`, `particles`, `feel`, `background`, `postfx`: event sound cues, particle bursts, hit-stop and shake, parallax, vignette and glow
- `src/item.rs`, `coin`, `charms`, `room_charms`, `editor_charms`, `charm_editor`, `charm_history`: items, coins, charm rules, charms in rooms, Charms edits
- `src/profile.rs`, `run`, `run/screen`, `upgrades`, `upgrade_editor`, `snapshot_history`: profile.ron, run loop, upgrades; `docs/run-and-profile.md`
- `src/difficulty.rs`: difficulty tiers: rules and data/difficulty.ron, scale(tier); `docs/run-and-profile.md`
- `src/game/tier_link.rs`, `run/bonus`, `damage/tier`, `enemy/tier`, `boss_special/tier`: a run's tier on rooms, hits, coins; never written back
- `src/editor_min_tier.rs`, `editor_panel/min_tier_field`: a placement's min tier (enemy, boss): draft edits, the Room tools field; `docs/run-and-profile.md`
- `src/tuning_panel/difficulty.rs`: F4 Difficulty section (six numbers, read-out at the debug tier, Save); `hud_run`: the HUD tier; `docs/run-and-profile.md`
- `src/seed.rs`, `game/seed_link`, `run/seed_line`, `tuning_panel/run/seed`: run seed, streams, result Seed line, F4 Seed row; `docs/variety.md`
- `src/variety.rs`, `mirror`, `enemy/variety`, `editor_variety`, `editor_panel/variety_fields`: enemy picks, dungeon flip, editor rows; `docs/variety.md`
- `src/pause_menu.rs` (+ `pause_menu/`), `shop_menu` (+ `shop_menu/`), `keybinds` (+ `keybinds/`): pause menu, hub shop, rebindable controls; `docs/keybinds.md`
- `src/room.rs`, `door_exit`, `ability`, `exit_lock`, `exit_link`, `exit_arrival`: rooms, door exits, abilities, locked exits, exit links; `docs/door-exits.md`
- `src/reach.rs`, `reach/span`, `reach/editor`, `reach/text`: reach envelope per ability measured on the real `Player`, preview, text; `docs/reach.md`
- `src/gates.rs`, `dungeon_check`, `dungeon_map`: gate marks, Check dungeon, dungeon map (play, editor); `docs/gates.md`, `dungeon-check.md`, `dungeon-map.md`
- `src/terrain.rs`, `terrain_style` (+ `terrain_style/`), `terrain_styles`, `game/terrain_link`: solids and collision, procedural look as a draw list, presets
- `src/hazard.rs`, `hazard_look`, `editor_hazard`, `scene_hazards`, `editor_panel/hazard_panel`: terrain hazards; `docs/hazards.md`
- `src/puzzle.rs`, `editor_puzzle`, `scene_puzzles`, `game/world_puzzles`, `editor_panel/puzzle_panel`: breakable walls, switches, barriers; `docs/puzzles.md`
- `src/editor.rs`, `editor_files`, `editor_session`, `editor_history`, `editor_guard`, `editor_look`: room editor: edits, files, session, undo, unsaved guard
- `src/editor_terrain.rs`, `editor_tools`, `editor_view`: room editor terrain edits, tools, canvas zoom, pan and grid
- `src/room_name.rs`, `room_size`, `editor_name`, `editor_inbound`, `editor_selection`: room name and size, inbound arrivals, canvas selection
- `src/definition_editor.rs` (+ `definition_editor/`): pure entity and behavior edits, save planning, staged writes

Composition and front end:

- `src/game.rs`: gameplay state and world composition; add systems as modules or fields, testable without a window
- `src/game/pause_link.rs`, `game/keybind_link`, `game/vitals_link`, `game/player_link`, `game/run_link`, `game/shop_link`: `GameState` owns and drives these
- `src/game/world_load.rs`, `game/world_doors`, `game/world_items`, `game/world_charms`: the one world loader, door rule, items and charms in play and Save
- `src/game/test_room.rs`, `game/editor_save`, `game/map_link`: Test room (plays the editor draft), editor save (checks, staged write), rooms seen this run
- `src/charm_menu.rs` (+ `charm_menu/`), `charm_input`, `hud_charms`, `charm_panel` (+ `charm_panel/`), `game/charm_link`, `game/charm_editor_link`: Charms UI
- `src/hud.rs`, `hud_bars` (+ `hud_bars/`), `hud_bars_draw`, `hud_damage`, `hud_layout`, `hud_run`: HUD, health and mana bars, result screen, inventory
- `src/shop_input.rs`, `hud_shop`, `scene_shop`, `hud_map`: hub shop input and drawing, map screen
- `src/editor_panel.rs` (+ `editor_panel/`), `editor_ui` (+ `editor_ui/`): egui editor panel frame, one sub-module per panel; window state and canvas frame
- `src/editor_panel/room_name_field.rs`, `editor_panel/name_dialog`, `editor_panel/link_dialog`, `editor_panel/link_panel`: room name, New room, exit-link UI
- `src/editor_visuals.rs`, `terrain_canvas`, `terrain_draw`, `body_draw`, `scene`, `effects`, `postfx_pass`: Macroquad drawing: canvas, terrain, bodies, effects
- `src/player_panel.rs` (+ `player_panel/`), `tuning_panel`, `tuning_panel/run`, `upgrade_panel` (+ `upgrade_panel/`), `game/upgrade_editor_link`: panels, F4
- `src/pause_input.rs`, `startup`, `main`: pause menu input, start-up and load errors, window and frame loop; `main.rs` stays thin, no gameplay rules

The library target (`src/lib.rs`) exposes reusable gameplay modules. Integration
tests live in `tests/` and verify behavior through the public API.

When you add a module, add its file to the line of its area (keep the line at 160 characters or less); a new area gets a new line; put anything longer in docs/modules.md or the matching docs/*.md. Rename or remove a module in the same line. After the first path on a line, paths are relative to `src/` and leave off `.rs`.

Limits: a module line here is at most 160 characters, and this file is at most 24 KB (24576 bytes), counted as if every line ended with CRLF (bytes plus the number of lines), so it holds on every checkout. `tests/docs_size.rs` fails this file over that limit, and any module line here or line of `FEATURES.md` over 200 characters.

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
