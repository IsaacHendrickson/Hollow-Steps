# Hollow Steps

Hollow Steps is a small Rust 2D action-platformer prototype inspired by the
feel of exploration-focused games such as Hollow Knight. The first vertical
slice is intentionally narrow: one player can walk, jump, and land inside a
single bounded room.

## Run it

Install Rust, then run:

```text
cargo run
```

Useful checks while developing:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Current controls

- `A`/`D` or the left/right arrow keys: move
- `Space` or `Z`: jump
- `Up`: pick up a nearby item while standing on the floor. The Amber Shard
  highlights and shows a prompt when in reach; collected items leave the world
  and stay in the player's inventory for the current game session.

See [`AGENTS.md`](AGENTS.md) for the project conventions and the recommended
workflow for future changes.

## Gameplay ownership

`GameState` owns the current `Room` (bounds, floor spawn x, and enemies).
Enemies use center-to-center distance for detection, chase horizontally within
room walls, and return to their patrol interval when the player leaves range.
`Player` owns pure health/timer state; `GameState` resolves rectangular contact
and respawns at the room spawn with full health, cleared velocity, and a fresh
one-second grace period. Items and enemy positions persist through respawn.
All timers use simulation delta time; rendering only reads state.
