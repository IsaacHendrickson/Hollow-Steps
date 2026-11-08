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
- `F5`: reload all room files and their enemy/behavior definitions, including
  while the inventory is open. Errors appear on screen; fix the files and
  press `F5` again to retry. Use the mouse wheel to scroll long errors.
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

Room geometry is defined in Rust in `src/room.rs`, with rectangular bounds, a floor,
and exits anchored to the left, right, top, or bottom edge. Exit spans are
offsets along that edge; each exit stores a destination room and player-center
spawn coordinate in that room. Overlapping an exit with the player's body
after movement triggers one transition per update. Spawns sit outside exit
zones so arriving does not immediately send the player back.

`GameState` owns the active room and retains each room's items for the session.
Transitions reset movement and recompute grounding at the destination while
preserving inventory. Room geometry and transitions remain window-free library
logic; the binary only renders the active room and its exits. Enemy placements
are loaded from the matching RON files in `assets/rooms/`.

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

## Data-driven enemies

Enemy state machines use RON in `assets/behaviors/`; entity definitions in
`assets/entities/` select a behavior by its file stem. Room enemy-placement
files in `assets/rooms/` reference entity definition names and supply patrol
endpoints. Room geometry, items, exits, and bosses remain in `src/room.rs`.
`serde` provides strict deserialization and `ron` parses the asset format;
neither adds rendering or an ECS to the simulation. Assets load when
constructing the rooms and on `F5`, with file paths included in validation errors.

A behavior file such as `assets/behaviors/chase.ron` contains named states:

```ron
(
    initial_state: "patrol",
    states: [
        (name: "patrol", actions: [patrol(speed: 90.0)], transitions: [
            (target: "chase", condition: player_distance_less_than(180.0)),
        ]),
        (name: "chase", actions: [move_toward_player(speed: 90.0)], transitions: [
            (target: "patrol", condition: player_distance_less_than(180.0), invert: true),
            (target: "chase", condition: action_finished),
        ]),
    ],
)
```

Actions execute in list order, carrying unused time to the next action:

| Action | Parameters and completion |
| --- | --- |
| `move_toward_player(speed: 90.0)` | Walk horizontally toward the player; finish at their x coordinate or the room wall. |
| `move_away_from_player(speed: 90.0)` | Walk horizontally away; finish at the room wall. At equal x, use the previous facing direction (initially right). |
| `patrol(speed: 90.0)` | Walk between the placement's endpoints, reflecting overshoot; run until interrupted by a transition. Return to the interval after chasing without teleporting. |
| `wait(seconds: 0.5)` | Remain still for the duration. |
| `telegraph(seconds: 0.5)` | Remain still and expose `Behavior::is_telegraphing()` for presentation during the duration. |

Completed sequences remain idle until a transition fires. Use an
`action_finished` self-transition to repeat a sequence or continue pursuing a
moving player after reaching them. Empty action lists are allowed and count
as finished. Zero speeds are stationary; zero durations complete immediately
on the next positive simulation step. Speeds are in pixels/second, positions
and distances in pixels, and timers in seconds. Entities stay on the floor.

| Condition | Meaning |
| --- | --- |
| `timer_elapsed(0.5)` | Time in the current state is at least the threshold. |
| `player_distance_less_than(180.0)` | Center-to-center distance on both axes is strictly below the threshold. |
| `player_distance_greater_than(180.0)` | Center-to-center distance is strictly above the threshold. |
| `health_below(2)` | Current health points are strictly below the threshold. |
| `action_finished` | All actions in the state have completed. |

Transitions are checked in file order before actions and again afterward if
none fired; the first matching transition wins. Optional `invert: true`
negates a condition, including its equality boundary. Entering any state,
including itself, resets its action cursor and both timers while preserving
facing. At most one transition fires per update, so instant cycles cannot
hang. A transition before actions runs the new state's actions in that step;
one after actions starts them next step. State timers are checked at update
boundaries; time beyond a transition is discarded. Negative, nonfinite, and
zero deltas do not advance behaviors. Knockback pauses their actions and timers;
dead enemies never update.

An entity file such as `assets/entities/chase.ron` selects its shared behavior
by filename without `.ron`; its `name` is what room files reference:

```ron
(name: "chase", health: 3, size: (36.0, 36.0), contact_damage: 1, behavior: "chase")
```

Health is a positive integer up to 255, size is a pair of positive dimensions,
and contact damage is an integer from 0 to 255 (0 disables contact damage).
Overlapping enemies apply the highest contact damage once, respecting the
player's normal immunity; damage saturates at zero health. Each entity instance
owns its health, timers, action cursor, and facing independently.

The three room placement files are `first.ron`, `second.ron`, and `boss.ron`:

```ron
(enemies: [(definition: "chase", patrol_left: 560.0, patrol_right: 800.0)])
```

Endpoints are absolute center x coordinates inside the room walls, accounting
for entity size. The enemy starts at the left endpoint, facing right. Equal
endpoints give a stationary patrol. Use `(enemies: [])` for an empty room.
The shipped first-room enemy retains its original speed, detection distance,
health, size, and damage. `patrol.ron` provides the same patrol without chasing.
Programmatic `Enemy::new` spawns (including existing boss summons) use the same
state machine with their supplied speed, radius, and endpoints.

Loading rejects malformed RON, unknown or missing fields, empty or duplicate
state/entity names, missing initial states or transition targets, missing
behavior or enemy references, invalid dimensions or patrol intervals, and
negative/nonfinite speeds, durations, or distances. Errors identify the file
and offending state, action, or reference; RON parse errors include line and
column. `GameState::try_new(asset_directory)` returns errors for callers;
`GameState::new()` loads the repository's `assets/` and fails with that context
if the data is invalid. No window is needed to load or test assets.

`GameState::reload_rooms()` re-reads all three room files and the enemy/behavior
definitions from the same asset directory used at startup. Changes apply only
after the entire load succeeds. Room-file errors are reported together; a
failed reload leaves the running world intact and allows another attempt.
A successful reload clears the error panel and replaces every room's enemy
list with fresh placements, resetting enemy health, positions, and AI timers
and removing existing summons. The current room, player movement/health,
inventory, collected coins/items, pause state, and boss progress are preserved.
