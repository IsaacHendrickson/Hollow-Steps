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

The repository currently has three layers:

- `src/player.rs`: player input, movement rules, collision resolution, and
  player-focused tests. This layer owns gameplay facts, not rendering.
- `src/game.rs`: top-level gameplay state and world composition. Add new
  gameplay systems here as separate modules or fields, keeping each system's
  rules testable without a window.
- `src/main.rs`: Macroquad window setup, keyboard translation, drawing, and
  frame scheduling. It should stay thin; do not put movement, combat, room,
  or persistence rules here.

The library target (`src/lib.rs`) exposes reusable gameplay modules. Integration
tests live in `tests/` and verify behavior through the public API.

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

## Testing

Agents should be able to verify almost everything without a window.

- All gameplay rules (movement, jumping, dashing, collisions, damage, room
  transitions, save data) live in pure modules that take explicit input and
  `delta_seconds` and return or mutate plain state.
- Tests must never start Macroquad, open a window, or need a GPU, display, or
  audio device.
- Every new mechanic needs tests for: the normal case, the boundary or edge
  case (world edges, floor, zero or very large `delta_seconds`), and at least
  one case that must not happen (for example, jumping while airborne).
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
```

## First-slice invariants

The initial player implementation must preserve these rules:

- The player cannot move outside the horizontal world bounds.
- The player can jump only while grounded.
- A jump starts with upward velocity and gravity brings the player back down.
- Landing places the player exactly on the floor and clears downward velocity.
- Movement and jump behavior can be tested without starting Macroquad.
