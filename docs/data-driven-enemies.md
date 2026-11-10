## Data-driven enemies

Enemy state machines use RON in `assets/behaviors/`; entity definitions in
`assets/entities/` select a behavior by its file stem. Room enemy-placement
files in `assets/rooms/` reference entity definition names and supply patrol
endpoints. Optional room `layout` data overrides geometry, items, spawn, and exits;
bosses use the optional placement above and coins remain in `src/room.rs`.
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
| `jump_to_player(dx: 0.0, dy: -200.0, speed: 300.0)` | Sample the player's center plus the offsets once when this action starts. Clamp the target so the whole entity fits between the walls, the arena top (y = 0), and the floor. Move in a straight line at the given speed; finish exactly at that target. Negative dy targets above the player. |
| `hover(seconds: 1.0)` | Hold both coordinates, including height, for the duration. |
| `slam_down(speed: 600.0)` | Move vertically down at the given speed; finish with the body's bottom exactly on the floor. |
| `area_blast(count: 3, radius: 48.0, fuse_time: 1.0, damage: 1)` | Sample `count` random circle centers anywhere inside the arena when the action starts. Show stationary warning circles, wait for the fuse, then detonate once and finish. |

Completed sequences remain idle until a transition fires. Use an
`action_finished` self-transition to repeat a sequence or continue pursuing a
moving player after reaching them. Empty action lists are allowed and count
as finished. Zero speeds are stationary; zero durations complete immediately
on the next positive simulation step. Speeds are in pixels/second, positions
and distances in pixels, and timers in seconds. Existing walking, waiting,
and telegraph actions stay on the floor. Jump and hover preserve height after
completion until another action moves the entity; slam lands on the floor.
Movement actions add no attack damage of their own; configured enemy contact
damage still applies when bodies touch. All enemy definitions can use every action.

Blast centers are sampled in `[bounds.left, bounds.right]` and `[0, floor_y]`;
circles may extend beyond the arena. Circles hit when they intersect the player's
rectangle, including the radius boundary. Overlapping detonations apply the
highest configured damage once per update, using normal player immunity.
Zero count emits no circles; zero damage is harmless. Zero fuse detonates on the
next positive simulation step. A transition interrupting a fuse cancels its
pending blasts. Knockback pauses the fuse and airborne movement without snapping
an airborne enemy onto the floor. Each instance owns a deterministic random stream.

For example, an enemy can leap above the player's sampled position, hover, land,
then cover the arena with delayed blasts using the existing action-list syntax:

```ron
[jump_to_player(dx: 0.0, dy: -200.0, speed: 300.0),
 hover(seconds: 0.75), slam_down(speed: 600.0),
 area_blast(count: 3, radius: 48.0, fuse_time: 1.0, damage: 2)]
```

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

`GameState::reload_rooms()` re-reads the built-in and saved custom room files and the enemy/behavior
definitions from the same asset directory used at startup. Changes apply only
after the entire load succeeds. Room-file errors are reported together; a
failed reload leaves the running world intact and allows another attempt.
A successful reload clears the error panel and replaces every room's enemy
list with fresh placements, resetting enemy health, positions, and AI timers
and removing existing summons. The current room, player movement/health,
inventory, collected coins/items, pause state, and boss progress are preserved.

