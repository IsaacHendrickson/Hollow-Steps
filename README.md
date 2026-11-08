# Hollow Steps

Hollow Steps is a small Rust 2D action-platformer prototype inspired by the
feel of exploration-focused games such as Hollow Knight. The first vertical
slice is intentionally narrow: one player can walk, jump, and travel between
three bounded rooms, including a boss arena.

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
- `Space` or `Z`: jump; hold to rise higher, release for a shorter jump.
  Jump input is buffered for 0.12 seconds before landing; coyote time allows
  a jump for 0.1 seconds after leaving support.
- `X`: swing in the last movement direction, including while airborne.
  The hitbox extends 38 pixels for 0.12 seconds with a 0.35-second cooldown.
  Each swing hits each enemy once; enemies take three hits to defeat and
  are knocked back on impact. Their health bars show remaining health.
- `Shift`: after collecting Dash, dash horizontally in the facing direction at
  800 units/second for 0.2 seconds. Dash starts have a 0.5 second cooldown;
  only one air dash is available until landing.
- `I`: open/close the inventory screen to see coins and collected items.
  Gameplay pauses while it is open; `Esc` also closes it.
- Touch a gold coin to collect it automatically, including while airborne.
  Each coin adds one to the balance and stays collected through room travel
  and respawn for the current game session.
- `Up`: pick up a nearby item while standing on the floor. The Amber Shard
  and Dash items highlight and show a prompt when in reach; collected items leave
  the world and stay in the player's inventory for the current game session.
- Walk into the right exit of the first room to enter the second room; its
  left exit leads back. The second room's right exit leads to the Ash Warden.
  Its arena exit stays red and locked until the boss is defeated.

## Boss architecture

`src/boss.rs` is simulation-only. A boss owns health and an ordered list of
`BossAttack` definitions: each is a named rectangle relative to the boss with
telegraph, active, and cooldown durations. New bosses reuse the same state
machine by supplying a different list; there is no boss-specific dispatch in
the game loop. Telegraphs preview exactly the area that becomes dangerous.
The Ash Warden has 30 health, deals one contact/attack damage, and alternates a
low sweep (jump over it) with a crown flare (stay low beside it).

The states are idle, telegraphing, attacking, and dead. Idle includes the
outgoing attack's cooldown plus a short pause before the next telegraph.
Timers consume elapsed simulation time across boundaries; full cycles can be
skipped for large steps. Invalid or negative time does not advance the boss.
Melee deals one damage per swing, and death immediately disables all boss
damage. Each room owns an optional boss; `Room::exits_locked()` derives its
answer from boss life instead of storing a second lock flag. `exit_at` enforces
the lock for every exit. Boss health/death persists through respawn and room
travel, and inactive rooms and the inventory pause freeze boss timers.

## Room design

Rooms are defined in Rust in `src/room.rs`, with rectangular bounds, a floor,
and exits anchored to the left, right, top, or bottom edge. Exit spans are
offsets along that edge; each exit stores a destination room and player-center
spawn coordinate in that room. Overlapping an exit with the player's body
after movement triggers one transition per update. Spawns sit outside exit
zones so arriving does not immediately send the player back.

`GameState` owns the active room and retains each room's items for the session.
Transitions reset movement and recompute grounding at the destination while
preserving inventory. Room geometry and transitions remain window-free library
logic; the binary only renders the active room and its exits. No room data files
or additional dependencies are used.

See [`AGENTS.md`](AGENTS.md) for the project conventions and the recommended
workflow for future changes.

## Gameplay ownership

`GameState` retains all rooms; each `Room` owns its bounds, player-center
respawn point, items, exits, and enemies. Only active-room enemies update.
Enemies use center-to-center distance for detection, chase horizontally within
room walls, and return to their patrol interval when the player leaves range.
`Player` owns pure health/timer state; `GameState` resolves rectangular contact
and respawns at the room spawn with full health, cleared velocity, and a fresh
one-second grace period. Items and enemy positions persist through respawn.
Defeated enemies stay removed when revisiting a room. Room transitions and
respawns cancel active swings and pending jump assistance.
All timers use simulation delta time; rendering only reads state.
