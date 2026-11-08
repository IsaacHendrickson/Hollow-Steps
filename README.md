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

The playable arena uses `src/boss_special.rs`: a 100-health boss cycles Slam,
Summon, and Rings roughly every three seconds. Move out of the red slam zone
and pending red rings; orange rings have already exploded. Summoned enemies
use normal enemy combat. Each melee swing deals one boss damage, and defeat
unlocks the exit and drops twenty collectible coins after a brief effect.
Inventory and inactive rooms pause the encounter. The game imports each enemy
and coin spawn once into the room's existing lists for combat and collection.

`src/boss.rs` is simulation-only. A boss owns health and an ordered list of
`BossAttack` definitions: each is a named rectangle relative to the boss with
telegraph, active, and cooldown durations. Bosses using this engine reuse its
state machine by supplying a different list. Telegraphs preview exactly the
area that becomes dangerous. The reusable Ash Warden example has 30 health,
deals one contact/attack damage, and alternates a low sweep (jump over it) with
a crown flare (stay low beside it).

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

Rooms load at startup from every `.ron` file directly inside `assets/rooms/`.
Run the game from the repository root so this relative asset path is available.
The three shipped files preserve the original rooms, pickups, enemy patrol,
coins, boss encounter, and travel coordinates. Add another file and connect an
exit to its unique `id` to add a room without changing Rust code. Filenames and
file order do not determine room identity. Gameplay starts in room `"first"` at
its `default_spawn`; each room uses its own default spawn after player death.

The format uses RON (Rusty Object Notation), with comments, named fields, and
explicit entity variants. For example, a complete room with a self-return exit:

```ron
(
    id: "workshop",
    origin: (0.0, 0.0),
    size: (400.0, 200.0),
    floor_y: 200.0,
    default_spawn: "start",
    spawns: [
        (name: "start", position: (140.0, 176.0)),
        (name: "arrival", position: (200.0, 100.0)),
    ],
    exits: [
        (edge: Right, offset: 104.0, length: 96.0,
         target_room: "workshop", target_spawn: "arrival"),
    ],
    entities: [
        Enemy(position: (220.0, 182.0), patrol: (180.0, 260.0),
              speed: 90.0, detection_radius: 180.0),
        Item(kind: AmberShard, position: (180.0, 190.0)),
        Coin(position: (70.0, 192.0)),
    ],
)
```

All fields are required; use `[]` for an empty list. Coordinates and sizes are
pixels, speeds are pixels per second, and positions are entity centers. The x
axis points right and y points down. `origin` is the room's top-left corner;
`size` is width and height; `floor_y` is the absolute y coordinate of the floor's
top edge. Entities and player bodies must fit inside the room above that floor.

Spawn names are unique within a room. Each exit selects a target room ID and a
spawn name in that target room. Arrival spawns may be airborne. Exit edges are
`Left`, `Right`, `Top`, or `Bottom`; `offset` is measured from the room's top for
side exits or left for top/bottom exits. Positive `length` must fit along that
edge; exit zones are 32 pixels deep. Spawn bodies must avoid exit zones to
prevent immediate return transitions. Overlapping an exit after movement
triggers one transition per update and clears movement while keeping inventory.

Placed entity variants are:

- `Enemy(position: (x, y), patrol: (left, right), speed: n, detection_radius: n)`:
  a normal enemy with three health. Patrol endpoints are absolute center x
  coordinates within the room, ordered left to right; the initial x must lie
  in that interval. Enemies are floor-bound, so y must equal `floor_y - 18`.
  Speed and detection radius must be finite and nonnegative.
- `Item(kind: AmberShard, position: (x, y))` or `Item(kind: Dash, ...)`:
  the existing inventory pickups, with a 16 by 20 pixel body.
- `Coin(position: (x, y))`: an 8 pixel radius coin.
- `ArenaBoss(position: (x, y))`: the existing 48 by 48 pixel arena boss using
  Slam, Summon, and Rings. At most one is allowed per room; its living state
  locks every exit in that room. The room must be at least 138 pixels wide to
  fit its summoned enemies. Encounter timers and spawn imports belong to each
  room independently. See `boss.ron` for the shipped encounter.

`src/room_data.rs` parses and validates the complete room graph before returning
usable rooms. It rejects malformed RON, missing/unknown fields and variants,
empty/duplicate IDs or spawn names, nonfinite or invalid geometry and entity
values, missing default spawns, missing target rooms, and exits with no matching
target spawn. Errors identify the file and, for validation failures, the room
and offending exit, spawn, or entity. Syntax errors include a line and column.
The binary reports the error and stops; it never silently substitutes a room.

`load_rooms(path)` loads a directory and `parse_rooms([(label, source), ...])`
loads in-memory sources. Both return `Result` and use no window, GPU, audio,
Macroquad context, or asset server. `GameState::load(path)` also checks the
starting room. Headless tests in `tests/room_data.rs` cover valid and bad files,
reference validation, geometry boundaries, and the exact shipped content;
existing gameplay tests continue exercising travel, pickup persistence,
respawn, and boss locks using these files.

`GameState` owns all loaded rooms and retains their mutable contents for the
session. Loading happens once at startup; transitions do not reload files or
respawn collected items, coins, or defeated enemies.

### Room dependencies

`serde` (with `derive`) maps RON fields and entity variants into small typed
input structs, avoiding a handwritten schema parser and rejecting unknown
fields. `ron` parses the human-editable Rust-like files and provides syntax
errors with source locations. These are the only added dependencies; validation
and gameplay construction remain explicit project code.

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
