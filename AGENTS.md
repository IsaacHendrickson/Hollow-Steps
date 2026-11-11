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

Gameplay modules (pure rules, no rendering):

- `src/player.rs`: player input, movement rules (including dash),
  collision resolution, and player-focused tests.
- `src/health.rs`: player health, damage, and invincibility.
- `src/enemy.rs`: enemy behavior, health, knockback, the dying state, and body instances. Its sub-modules: `catalog.rs` (`EnemyDefinitions`: the validated entity catalog and spawning), `definition.rs` (`EnemyDefinition`, including `death_secs`, its default and range) and `death.rs` (`DeathTimer`, the fade clock of a defeated enemy).
- `src/pose.rs`: semantic pose assets, presentation playback, and validation (enemy and boss pose names).
- `src/body.rs`: validated RON body templates, local point springs/idle targets, palettes, and pure transforms.
- `src/body_debug.rs`: pure body overlay primitives, display constants, and nearest-entity selection.
- `src/action_runtime.rs`: shared action execution state, motion, attack output, and actor adapters; `ActionRuntime::execute` is a dispatcher that sends each action to its own function in `src/action_runtime/`: `attacks.rs` (area_blast, shoot_projectile, slam_down, jump_to, lunge), `movement.rs` (wait, hover, walking, patrol, jump_to_player, legacy_slam_down, shared gravity and flight helpers), `blasts.rs` (legacy_area_blast, spawn_rings, explode_rings), `instant.rs` (glides, damage_target, spawn_shockwave, summon), `poses.rs` (pose steps around each action, and the zero-time `pose` action). Executors read the action by reference; never clone it per frame.
- `src/assets.rs`: assets folder resolution (`HOLLOW_STEPS_ASSETS`, next to the executable, then the source tree), load-failure text, and `read_override` (an optional whole-file override of an embedded `data/*.ron` default).
- `src/behavior.rs`: root of the data-driven action sequences, state transitions and RON validation; it only declares modules and re-exports the public API. `src/behavior/`: `action.rs` (the `actions!` table: `Action` enum, loading, editor catalog), `action_serde.rs` (deserialization helpers and older-form readers), `action_info.rs` (wind-up, feedback, attack preview, pose bindings), `condition.rs` (the `conditions!` table and evaluation), `definition.rs` (states, transitions, validation, loading), `runtime.rs` (`Behavior` and `update_with_player_size`), `validators.rs` (shared value checks).
- `src/enemy_attack.rs`: shared attack geometry, timed enemy attacks, and projectiles.
- `src/preset.rs`: validated RON behavior preset catalog.
- `src/action_parameters.rs`: labeled scalar editor fields for the shared action registry.
- `src/behavior_values.rs`: numeric behavior controls and transactional validated edits.
- `src/boss_definition.rs`: validated RON boss definitions (optional `body`, checked against body templates and poses) and attack selection settings.
- `src/boss_editor.rs`: pure boss property buffers and ordered attack/action edits.
- `src/boss_special.rs`: boss sequence cursors, selection, actor effect integration, and coin drops.
- `src/boss_special/body_link.rs`: how a boss drives its optional body (`BossBody`): hurt flash, warning tint, facing, death fade, stepping and the pose steps of its attack actions (as `Enemy` applies them); the hitbox stays the definition's `size`.
- `src/combat.rs`: melee swings (forward, up, down), per-direction hitboxes from `StrikeConfig`, knockback direction, once-per-swing pogo, duration, and cooldown.
- `src/damage.rs`: `DamageResolver` combines enemy contact/blast/attack and boss action/contact damage into the single hit applied to the player.
- `src/events.rs`: `GameEvent` (jump, landing, dash, swing, enemy hit/death, player damage, coin, item, room, boss) and the `EventQueue` drained by `GameState::take_events`, or by `take_located_events` with the world position each event happened at (`LocatedEvent`); a `PlayerDamaged` event also carries the `HealthLoss` (health lost, health left) in `LocatedEvent::loss`.
- `src/sound.rs`: validated event-to-sound cues (`data/audio.ron`, an `assets/audio.ron` replaces it whole), sound-name and volume checks, sound-file lookup in `assets/audio/` (wav, then ogg), the WAV/Ogg header check, and the per-frame play plan; sub-module `tests.rs`.
- `src/particles.rs`: validated burst presets (`data/particles.ron`, an `assets/particles.ron` replaces it whole), a seeded deterministic generator, the capped live-particle list, and the event-to-burst rules; the `hud_damage` preset lists no events and is spawned only by `spawn_hud_damage` (the HUD's emitter); sub-module `tests.rs`.
- `src/feel.rs`: hit-stop and screen shake driven by `GameEvent`s (sine shake, no randomness); tunables in `data/feel.ron` (an `assets/feel.ron` replaces it whole).
- `src/background.rs`: parallax background layers (procedural `Rect`/`Circle`/`Triangle` shapes with a depth factor, per room in `layout.background`), their validation, and the pure `parallax_offset`; sub-module `tests.rs`.
- `src/postfx.rs`: post-process settings (vignette and glow, `data/postfx.ron`, an `assets/postfx.ron` replaces it whole, `enabled` is the data toggle), the shader source, and the pure `vignette_factor` / `glow_weight` formulas the shader runs; sub-module `tests.rs`.
- `src/player_body.rs`: the player's look (`data/player_body.ron`, an `assets/player_body.ron` replaces it whole), pose choice from player state, and the `BodyInstance` that plays the `wanderer_*` poses; no template in the assets keeps the box.
- `src/item.rs`: world items and pickup.
- `src/coin.rs`: collectible coins.
- `src/room.rs`: rooms, exits, and room transitions.
- `src/terrain.rs`: solid rectangles, axis-separated substepped collision, and terrain validation.
- `src/camera.rs`: follow camera with look-ahead and room clamping, screen-shake view offset, letterbox fit, HUD anchors; tunables in `data/camera.ron`.
- `src/timestep.rs`: fixed 120 Hz step accumulator (capped) and edge-input latch.
- `src/tuning.rs`: player, strike (hitbox sizes, pogo speed) and enemy-knockback tunables from `data/player.ron` (an `assets/player.ron` replaces it whole), validation, staged save.
- `src/tuning_fields.rs`: the editable `player.ron` numbers (`Field`, `FieldSpec`): one range per field shared by validation and the F4 window; re-exported from `tuning`.
- `src/vitals.rs`: pure player vitals (health, trailing bar, mana, hold-to-heal) driven by the tick; no window, no `GameState`. Tunables in `data/vitals.ron` (an `assets/vitals.ron` replaces it whole); `VitalsField`/`FieldSpec` ranges shared by validation and the F4 window. Tests in `src/vitals/tests.rs`.
- `src/editor.rs`: pure room editing, validation, serialization, and file save boundary.
- `src/editor_guard.rs`: unsaved-changes state machine (Save / Discard / Cancel) and the approved-action runner.
- `src/editor_history.rs`: snapshot undo/redo for the room draft (limit 100; a drag is one step) and the Ctrl+Z / Ctrl+Y key mapping.
- `src/editor_terrain.rs`: pure terrain edits on the room draft (add, move, resize, delete, hit tests, handles), validated as a whole.
- `src/editor_tools.rs`: the editor tool enum (Select, Place, Terrain, Erase), its key map, and the terrain tool's selection and drag state.
- `src/editor_view.rs`: pure editor canvas math: zoom/pan transform, grid snapping, window regions, footer wrapping.
- `src/room_size.rs`: validated room bounds/floor edits for the editor.
- `src/definition_editor.rs`: pure entity/behavior edits, save planning, and staged file writes.
- `src/search.rs`: pure editor list filtering and stable selection.

Composition and front end:

- `src/body_draw.rs`: Macroquad adapter: `draw_body` (ordered body parts for enemies, bosses and the player), the shared `body_tint` (hurt flash, warning tint, fade), legacy enemy boxes, and debug overlays.
- `src/game/vitals_link.rs`: how `GameState` drives the vitals: input gating while a heal runs, healing and regain copied into `Health`, and the F4 / F5 accessors (`vitals`, `set_vitals_config`, `save_vitals`).
- `src/game/test_room.rs`: Test room: swaps the editor's draft in as the playable room, parks the editor session and player, and restores them.
- `src/game.rs`: top-level gameplay state and world composition. Add new
  gameplay systems here as separate modules or fields, keeping each system's
  rules testable without a window.
- `src/editor_panel.rs`: egui editor panel frame (`draw_panel`, the Room tools / Entity properties switch). One sub-module per panel: `room_tools.rs` (Room tools view: rooms, exits, entity list), `entity.rs` (Entity properties view, entity fields, `pending_error`), `body_visual.rs` (Body visual, Pose assets, template motion), `behavior.rs` (preset picker, behavior create / pick / rename / delete, shared-behavior guard), `states.rs` (States, action and transition rows), `key_values.rs` (Key values), `boss.rs` (boss properties), `canvas_tools.rs` (View and Room size sections), `death_field.rs` (the Death time drag box), `history_panel.rs` (Undo/Redo), `tool_panel.rs` (tool selector, Delete selected), `unsaved_dialog.rs` (Save / Discard / Cancel window). `property.rs` holds `PropertyUi` and the named `EntityFields`; `widgets.rs` the shared widgets and `probe`, the single test-only layout hook. Headless tests: `panel_tests/` (whole panel) and `test_support.rs` (section helpers).
- `src/editor_ui.rs`: the room editor's window state (`EditorUi`) and its canvas frame (`draw_editor`), moved out of `main.rs`; tests in `src/editor_ui/tests.rs`.
- `src/terrain_canvas.rs`: Macroquad drawing of solids, handles and the drawing preview, and mouse-to-`TerrainTool` glue for the editor canvas.
- `src/tuning_panel.rs`: F4 egui window with a drag box per `player.ron` field and per `vitals.ron` field (typed out-of-range values are refused with a message), unsaved note, Save (writes `vitals.ron` only when its values changed), and headless layout/input checks in `tuning_panel/tests.rs`.
- `src/scene.rs`: Macroquad drawing of the world (background layers first, then floor, terrain, exits, items, coins, enemies, boss, player), moved out of `main.rs`.
- `src/postfx_pass.rs`: Macroquad adapter for the post-process pass: builds the material, draws the world target through it into a second target and returns the target the HUD is drawn on; an unbuilt shader or a switched-off pass returns the input untouched.
- `src/effects.rs`: Macroquad front end for event-driven effects: feeds the frame's located events to the particles and the sound bank, steps the particles with the fixed tick and draws them in world space (`draw_particles` is shared with the HUD's emitter). Its sub-module `sound_bank.rs` loads and plays sound files; real playback needs the opt-in `audio` cargo feature (`docs/audio.md`).
- `src/hud_bars.rs`: pure health and mana bars: `HudConfig` (`data/hud.ron`, an `assets/hud.ron` replaces it whole), `BarValues` from the vitals, and `layout_bars`, which turns them into the rectangle of each layer (empty, trailing, pending heal, health, mana) inside a fixed-length bar. Tests in `src/hud_bars/tests.rs`.
- `src/hud_damage.rs`: pure damage feedback for the health bar: `BarShake` (sine shake scaled by the share of max health lost, clamped, fading linearly, no randomness), `lost_segment` / `burst_point`, and `HudFeedback`, which owns the shake and a separate seeded `ParticleSystem` in screen space, driven by `PlayerDamaged` events. It knows nothing of the camera. Shake numbers are in `data/hud.ron`, the burst is the `hud_damage` preset of `data/particles.ron`. Tests in `src/hud_damage/tests.rs`.
- `src/hud_layout.rs`: pure HUD rules: boss bar fill and caption, room labels (the old health-pip helpers remain but the HUD no longer draws them).
- `src/hud.rs`: Macroquad HUD (health and mana bars with the damage shake, counters, room label, boss bar, the health bar's damage particles), inventory and reload panels drawn in virtual-resolution space with `camera::anchored`.
- `src/startup.rs`: Macroquad startup: loads the game and shows load errors on the virtual-resolution screen (F5 retries); shared letterboxed frame presentation.
- `src/main.rs`: Macroquad window setup, keyboard translation, drawing, and
  frame scheduling. It should stay thin; do not put movement, combat, room,
  or persistence rules here.

The library target (`src/lib.rs`) exposes reusable gameplay modules. Integration
tests live in `tests/` and verify behavior through the public API.

When you add, rename, or remove a module, update this list in the same change.

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
  code or fixed fixtures under `tests/fixtures/`, including enemy and behavior
  definitions. The sole exception is one dedicated shipped-content test that
  checks loading and validation without asserting exact layouts or coordinates.
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
