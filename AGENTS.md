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
- `src/enemy.rs`: enemy behavior, health, knockback, death, and body instances.
- `src/pose.rs`: semantic pose assets, presentation playback, and validation.
- `src/body.rs`: validated RON body templates, local point springs/idle targets, palettes, and pure transforms.
- `src/body_debug.rs`: pure body overlay primitives, display constants, and nearest-entity selection.
- `src/behavior.rs`: data-driven action sequences, state transitions, and RON validation.
- `src/enemy_attack.rs`: shared attack geometry, timed enemy attacks, and projectiles.
- `src/preset.rs`: validated RON behavior preset catalog.
- `src/action_parameters.rs`: labeled scalar editor fields for the shared action registry.
- `src/behavior_values.rs`: numeric behavior controls and transactional validated edits.
- `src/boss.rs`: boss health, reusable timed attacks, state machine, and death.
- `src/boss_definition.rs`: validated RON boss definitions and attack selection settings.
- `src/boss_editor.rs`: pure boss property buffers and ordered attack/action edits.
- `src/boss_special.rs`: ordered shared boss actions, selection, summons, rings, and coin drops.
- `src/combat.rs`: melee swing hitboxes, duration, and cooldown.
- `src/item.rs`: world items and pickup.
- `src/coin.rs`: collectible coins.
- `src/room.rs`: rooms, exits, and room transitions.
- `src/editor.rs`: pure room editing, validation, serialization, and file save boundary.
- `src/definition_editor.rs`: pure entity/behavior edits, save planning, and staged file writes.
- `src/search.rs`: pure editor list filtering and stable selection.

Composition and front end:

- `src/body_draw.rs`: Macroquad adapter for ordered body parts, legacy enemy boxes, and debug overlays.
- `src/game.rs`: top-level gameplay state and world composition. Add new
  gameplay systems here as separate modules or fields, keeping each system's
  rules testable without a window.
- `src/editor_panel.rs`: egui editor panel widgets and headless layout/input checks.
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
  directly.
- Clamp or validate external values at system boundaries. Do not scatter
  magic numbers through the rendering loop.

## Tunable values

Every tunable value must be a named parameter in data files and editable in the editor.

Every number that affects how an entity, boss, attack, or action behaves
(distances, speeds, durations, damage, health, counts, radii, etc.) must be a
named parameter stored in data files and editable in the editor with a
descriptive label and validation. Do not hardcode these values in Rust,
apart from engine-level constants such as gravity. When adding an action,
condition, or entity type, add its parameters to the editor's catalogs in
the same change.

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
  `README.md` or a short design note before implementing it.
- Keep files focused and reasonably small so future agents only need to read
  the module they are changing.

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
