# Hollow Steps

Hollow Steps is a small Rust 2D action-platformer prototype inspired by the
feel of exploration-focused games such as Hollow Knight. The first vertical
slice is intentionally narrow: one player can walk, jump, and travel between
three bounded rooms, including a boss arena.

## Run it

Install stable Rust. On Ubuntu/Debian, install Macroquad's X11, OpenGL, and
ALSA development dependencies first:

```sh
sudo apt-get update
sudo apt-get install -y pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
```

Then run:

```text
cargo run
```

Useful checks while developing:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

GitHub Actions runs these checks on `ubuntu-latest` and `windows-latest` for
every pull request and push to `main`. The Linux job installs the packages
above; Windows needs no extra system libraries.

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
- `F2`: open/close the room editor (gameplay pauses).
- `F5`: validate room files and their enemy, body template, behavior, and boss definitions, including
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

## Enemy body templates

Presentation is separate from collision and simulation. `assets/bodies/<name>.ron`
contains one strict body template, selected by the filename stem. Each template
has a `points` map (`"head": (rest: (x, y))`) and ordered `parts`. Positions and
sizes are fractions of the enemy's unchanged collision box, centered at `(0, 0)`,
with x right and y down. Enemy instances own unmirrored current positions,
velocities, and targets; drawing resolves positions and mirrors only the result.

Templates also accept motion data (omitted fields use the defaults in
`data/body_motion_defaults.ron`):

```ron
follow: (stiffness: 120.0, damping: 22.0),
follow_overrides: {"tail": (stiffness: 80.0, damping: 10.0)},
idle: [
    (point: "tail", amplitude: (0.015, 0.02), period_secs: 1.8, phase_offset_secs: 0.0),
],
```

The target defaults to rest plus the sum of idle offsets, each
`amplitude * sin(2*pi*(game_time + phase_offset_secs)/period_secs)`. Amplitudes
are local box fractions. `BodyInstance::set_target(name, position)` overrides
that target; `clear_target(name)` resumes rest + idle; `snap_to_targets()` places
all points at their targets and zeros velocities. Spawn snaps immediately,
including nonzero idle phases. No action poses are assigned.

Each step uses semi-implicit Euler: update velocity by
`(stiffness * (target - position) - damping * velocity) * dt`, then position by
the new velocity times `dt`. Only this spring delta is capped at 1/30 second;
idle time advances with the supplied game delta. Inventory, the editor, and
inactive rooms freeze motion. The 120/22 defaults settle without overshoot.
Lower damping gives tails and wings more overshoot. Stiffness must be finite and positive, damping
finite and nonnegative, and idle periods finite and positive. Amplitudes/phases
must be finite; idle and override names must exist in the point map.

Each part has `name`, `shape` (`Ellipse`, `Triangle`, `Rect`, or `Line`), `size:
(w, h)`, `rotation` in clockwise degrees, and `color_slot` (`primary`, `secondary`,
`accent`, or `eye`). Non-Line parts use `anchor: Some("head")` and `offset:
Some((x, y))`; Line parts use `from: Some("torso")` and `to: Some("hand_l")`
instead. Missing optional fields are omitted. Ellipses/rectangles are centered;
a triangle points right before rotation. Offsets use the box axes; rotation
turns the shape about its resolved center. A Line connects its two current
points, with thickness `min(w * box_width, h * box_height)`; its rotation turns
both endpoints about their midpoint (use zero to connect the points exactly).
Horizontal facing mirrors the complete result about the enemy center.

An enemy definition may include:

```ron
body: Some((
    template: "humanoid",
    primary: (0.55, 0.19, 0.30, 1.0),
    secondary: (0.27, 0.10, 0.23, 1.0),
    accent: (0.95, 0.57, 0.23, 1.0),
    eye: (1.0, 1.0, 0.75, 1.0),
    scale: 1.0,
)),
```

All four colors are normalized RGBA channels in `0..=1`. Scale defaults to one
and must be finite and positive; it scales points, offsets, and shapes about
the entity center. Omitting `body` or using `None` keeps the original maroon box.
The editor's **Body visual** section selects a template or Box and edits scale
and all four colors. New assignments copy `data/body_visual_defaults.ron`;
existing palettes are preserved when switching templates. The same section edits
shared template stiffness/damping, point overrides, and idle point/amplitude/
period/phase entries. Template geometry remains authored in RON. Loading and staged saves validate template names and
point references; unknown color slots are parse errors. F5 reloads templates
and enemy settings together, retaining the current game if validation fails.
Matching enemy placements retain surviving point positions, velocities, explicit
targets, and idle phase; new points snap to their targets and removed points disappear.
Reload matches name and patrol interval first, then moved placements by name and
occurrence, since placements have no persistent IDs.
Editor saves also reject body files changed since the editor opened.

Manual check: run the game, confirm chase (humanoid) and scittish (bird) have
visible bright eyes, watch the bird/beak mirror when it turns, and confirm an
enemy without a body still draws as a box. Edit a template point/part and an enemy palette/scale,
press F5, and confirm both refresh. In F2, edit an enemy's health and palette,
save, reopen, and confirm its body is retained. Hitboxes and health bars stay at
the original box size even if visual scale changes.
Watch breathing and opposite-phase wing motion; assign an animal body to see its
tail swish. Open inventory/F2 and confirm motion freezes, then resumes when closed.
Turn an enemy and confirm points flip immediately without sliding. Change an idle
amplitude or rest position and press F5: current points stay in place and ease to
the new targets. In Body visual, edit motion fields, save, reopen, and confirm
the values persist; an invalid period or spring value must block Save.

## Boss architecture

Boss definitions are strict RON files in `assets/bosses/`. Rooms reference the
definition's `name`, just like enemy placements:

```ron
(enemies: [], boss: Some((definition: "Ash Warden", position: (560.0, 436.0))))
```

Omitting `boss` leaves the room without a boss. Positions are body centers and
must fit within the room. A definition contains all encounter settings:

```ron
(
    name: "Ash Warden", health: 100, size: (48.0, 48.0), contact_damage: 0,
    selection: cycle, pause_seconds: 3.0,
    attacks: [
        (name: "Slam", actions: [
            glide_to_player(speed: 1000.0, above_floor: 180.0),
            telegraph(seconds: 0.5),
            glide_to_floor(speed: 1000.0),
            damage_target(radius: 56.0, damage: 1),
        ]),
    ],
)
```

`selection` is `cycle` (file order, wrapping) or `random` (uniform choices,
repeats allowed, with an instance-local seed for reproducible tests). The pause
is the minimum interval between attack starts, including attack time; an
unfinished sequence delays the next start. The first attack starts after that
interval. Selection and timers belong to each boss, so inactive rooms pause.

Attacks have unique nonempty names and nonempty ordered action lists using the
shared action library. `wait(seconds)` and `telegraph(seconds)` consume time;
instant actions run in order at a boundary, and unused time passes to the next
action. Boss action parameters (pixels, seconds, pixels/second) are:

| Action | Parameters and completion |
| --- | --- |
| `glide_to_player` | `speed`, `above_floor`: set a nonblocking glide to the captured player x and floor minus height. Height 0 preserves the current y. |
| `glide_to_floor` | `speed`: set a nonblocking glide to the captured x and floor minus half the boss height. |
| `damage_target` | `radius`, `damage`: immediately damage a player within the captured x radius, even if still gliding. |
| `summon` | `inset`: spawn two stationary patrollers this far from the arena's sides. |
| `spawn_rings` | `count`: place this many random arena rings immediately. |
| `explode_rings` | `radius`, `damage`: explode pending rings and damage a player inside any ring. |
| `jump_to` (alias of shared `jump_to_player`) | `dx`, `dy`, `speed`: glide to the captured player position plus offsets, clamped to the arena; complete on arrival. Use `dy: -200.0` for 200 pixels above the player. |
| `hover` | `seconds`: hold position for the duration. |
| `slam_down` | `speed`: descend to the floor; complete on landing. |
| `spawn_shockwave` | `radius`, `damage`: immediately damage a player within the horizontal radius of the boss. |
| `area_blast` | Shared `count`, `radius`, `fuse_time`, `damage`: create arena circles, wait for the fuse, then explode them. |

The ground-pound sequence (`jump_to` 200 above the player, `hover` 0.5 seconds,
`slam_down` at speed 100, `spawn_shockwave`) exists only in a test fixture.
Boss-only effects and nonblocking glides are rejected in enemy behaviors and
are not added to the enemy editor's action picker. The shared jump, hover, slam,
and area blast actions keep their existing enemy behavior and controls.
Existing enemy horizontal movement actions are rejected in
boss attacks. Loading rejects unknown/missing fields or action types, duplicate
boss/attack names, zero health, invalid size or placement, missing references,
negative/nonfinite values (signed finite jump offsets are allowed), zero boss
movement speeds, noninteger damage outside
0..255, and ring counts outside 1..256. Errors identify the file, boss, attack,
action index, and invalid parameter. F5 validates bosses before applying a
reload; existing encounter progress remains intact. Room saves preserve boss
placements; this task adds no boss editor controls.

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

Default room geometry is defined in Rust in `src/room.rs`, with rectangular bounds, a floor,
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

## Room editor design

F2 pauses gameplay and opens the room editor; F2 again returns to play.
The editor uses `egui-macroquad` 0.17.3 (egui 0.31.1) for layout, input,
dropdowns, and draggable scrollbars. Its macroquad requirement is `^0.4.14`,
compatible with this project's `macroquad = "0.4"` and locked 0.4.16; Cargo
resolves one macroquad and one miniquad. This dependency replaces the built-in UI
because egui allocates control rectangles and clips nested scrolling correctly.
Default features are disabled to avoid enabling the integration's optional audio
backend. No additional Linux system libraries are required: the existing CI
package list is unchanged, and CI explicitly builds the locked binary on Linux
before testing it. The macOS clipboard dependencies in Cargo.lock are not built
on Linux (`cargo tree --target x86_64-unknown-linux-gnu` shows the active tree).

A single 360-pixel side panel (capped to the window width) contains collapsing
**Room**, **Entity**, **Behavior**, **States**, **Actions**, and **Transitions**
sections. **Save** is pinned outside the main scroll area. Wheel steps move
28 UI points, and solid scrollbars reserve their own space and support dragging.
Every searchable picker has a Filter text box, an **N of M** count, and an egui
dropdown with a scrollable result viewport sized for at least eight rows. Its width
follows the panel; near the window bottom egui places the popup above the picker.
An empty filter result shows **No matches**. Existing Unicode, prefix-first
filtering and source-index selection remain in `src/search.rs` with their tests.
Long captions truncate with a hover tooltip, other text wraps, and text fields
scroll horizontally. `searchable_list` in `src/editor_panel.rs` is the shared
place for future search/navigation improvements. Pointer capture includes open
popups and active UI drags, so panel gestures cannot edit the canvas; Delete in
a text field cannot delete the selected canvas object.
Mouse input and preview drawing stay in `main.rs`; panel widgets live in
`editor_panel.rs`. `editor.rs` owns window-free place, move, delete, validation,
and serialization rules. Edits use authored RON
placements rather than enemies that have moved or pickups already collected.

Rooms retain the legacy `enemies` field and gain optional `layout` data for
bounds, floor, spawn, items, and exits. Missing layout uses the original room
geometry and items, so existing content stays compatible. Enemy positions
remain floor-bound: placing snaps to the floor, moving shifts both patrol
endpoints without changing their separation. Items use free center coordinates.
Exits stay anchored to their original edge; dragging slides their span along it.
Spawn and exit destination markers can be dragged separately. Save validates
all geometry, entity references, and exit destinations before serializing, then
stages sibling temporary files before replacing the room and definition files.
Invalid edits leave all files unchanged; an I/O replacement failure attempts to
restore files already replaced and reports any rollback error.

New room creates an empty draft with the current room's geometry and a unique
`room_N.ron` filename. Save registers it as `Custom(N)` in the room list and game;
these files are loaded at startup. Exits can target any saved room. Room IDs are
stable numeric IDs, independent of list order. Coins remain outside this editor's scope.

Choose an entity in the **Entity** list, click **Place**, then click the room.
Choose **Select / drag** to move entities, the blue respawn marker, or a green
exit span. Right-click an enemy/item/boss or select it and press Delete to remove it.
Select an exit, choose a **Target**, and click **Set exit target**; use
**Arrival spawn** to drag its purple arrival marker in the target
room's coordinate space. **Back to room** restores the current-room view.
**Add exit** creates a span on the chosen **Edge** so new rooms can be connected.
**Save** applies the draft to gameplay and disk; errors remain visible in
the footer and preserve the draft, running room, and previous file.
Closing with F2, opening another room, or choosing New room discards unsaved
changes. A star next to the room ID marks unsaved changes. Saving resets that
room's editable enemies/items and places the player at its spawn, while keeping
inventory, coins, and unchanged boss progress. Editing a boss definition or placement
restarts affected encounters with the saved content. F5 retains its existing enemy-only reload
behavior for already loaded rooms; restart or use editor Save to apply layouts.


Select an enemy, then choose **Entity properties** in the right panel. Edit its
shared name, health, width/height, and contact damage. Choose a behavior in the
**Behavior** dropdown and click **Use selected behavior** to assign it. These
edits affect every instance using that definition. Renaming an
entity updates references in all saved rooms while retaining its entity filename.
Use **Room tools** to return to placement controls. Scroll the panel for controls
below the visible area.

Boss definitions appear as **Boss: name** in the same searchable entity list.
Place one boss per room, drag its purple body freely within the room, or delete it.
Selecting a boss on the canvas opens its shared properties: name, health, size,
contact damage, attack selection mode, and pause between attacks. **Add attack**
prompts for a unique name. Select an attack to rename it or edit its ordered
actions; use **Up**, **Down**, and **Delete** on attack and action rows.
The searchable **Action type** list includes every action supported by bosses.
**Add action** inserts editable data defaults; each parameter has a labeled,
validated field. Empty attacks/lists and invalid fields block Save, including
fields in collapsed sections or other attacks. Boss files use the same staged,
conflict-checked saves as entities; renames update all room references.
Boss-only insertion defaults live in `data/boss_action_defaults.ron`.

Manual boss editor check: open F2, choose the Boss room, confirm the purple boss
appears, then select and drag it. Edit properties, add/rename/reorder an attack,
search for and add an action, and change its parameters. Save, reopen the editor,
and confirm the edits remain. Try an invalid value and confirm Save reports the
error without changing files; delete the boss, save, and confirm it disappears.

To create an enemy from scratch, press F2, open **Room tools**, enter a unique
**New / duplicate entity name**, and click **New entity**. It opens Enemy
properties with its own blank `idle` behavior and the structured controls enabled;
it does not copy another enemy. Add actions, states, and transitions there, or
choose **New behavior from preset**, select a preset, name the behavior, leave
**Assign to selected entity** checked, and click **Create behavior**. To use an
existing behavior, select it and click **Use selected behavior**. **New blank
behavior** creates and assigns a fresh empty behavior to the selected entity;
**Duplicate entity** deliberately shares the source behavior until reassigned or
copied. Return to **Room tools**, select the new entity, click **Place**, click
the room canvas, then **Save**. Save also persists definitions without placements.

The behavior section edits the selected entity's shared behavior file and shows
**used by N entities**, counting entity definitions across all rooms, including
definitions with no placements. Before editing a behavior used by multiple
entities, choose **Save as new behavior** to copy it under a new name, optionally
assigning the copy to the selected entity. **Edit shared behavior** explicitly
enables changes for all users instead. Assignment affects every placement of that
entity definition.

**Key values** lists every numeric action and condition parameter across all
states, with state names, descriptive labels, units, and row numbers. Inverted
conditions describe the inverted comparison, including equality. Invalid input
keeps the last valid definition and blocks Save even with the section collapsed.
Speeds, durations, radii, and distance thresholds must be finite and nonnegative;
jump offsets may be any finite signed value. Blast counts must be integers in
0..65535; damage and health thresholds must be integers in 0..255.
Enable **Advanced: structured rows** for state, action,
and transition editing; **Advanced: raw RON** remains available inside that view.
For Skittish, **Key values** exposes `idle: switch to flee when player closer
than` (start fleeing), `flee: switch to idle when player farther than` (stop
fleeing), and `flee: speed away from player`. The two distance thresholds are
independent and use strict comparisons; choose a stop distance larger than the
start distance to avoid rapid switching. Distance is center-to-center
in pixels. Edit these fields and **Save** to apply them to gameplay and disk.
Choose a state, rename it, make it initial, add a named state, or remove it. Renaming updates
all transition targets. Removing drops transitions targeting that state and, if
needed, makes the first remaining state initial; the final state cannot be removed.
Actions have a type name, labeled numeric parameters, and Up/Down/Delete controls.
Choose a type and **Add action** to insert editable defaults. Transitions have
target-state and condition dropdowns, a condition value when required, an invert
checkbox, and the same ordering controls. Invalid numbers show errors beside their
fields and block saving using the same validation as Key values and the loader.

The loader enums and editor type catalogs come from the same `actions!` and
`conditions!` declarations in `behavior.rs`. Add a supported variant there with
its typed parameters, descriptive labels, and validation to include it in the
dropdown and Key values automatically. Editor insertion defaults live in
`data/action_defaults.ron`; existing defaults are unchanged. Inserted values are
editable and saved in behavior files through the existing RON format. Add a
matching default entry in the same change. The compiler also requires gameplay
matches to handle the new variant. Gameplay tuning belongs in data parameters,
not Rust literals; see the **Tunable values** rule in `AGENTS.md`.

**Advanced: raw RON** toggles the original ordered-list text editors using the
existing syntax:

```ron
// Actions: insert, delete, reorder, or edit parameters in this list.
[patrol(speed: 60.0), wait(seconds: 0.5), telegraph(seconds: 0.25)]
// Transitions: first matching condition wins; invert is optional.
[(target: "idle", condition: action_finished),
 (target: "attack", condition: player_distance_less_than(120.0), invert: true)]
```

Other actions are `move_toward_player(speed: ...)` and
`move_away_from_player(speed: ...)`. Other conditions are `timer_elapsed(...)`,
`player_distance_greater_than(...)`, and `health_below(...)`.
The pinned **Save** button validates all definitions, room placements,
and exit destinations before any write. Syntax and validation errors appear in
the panel; correct the edit and retry. Saving keeps the existing RON schemas.
Definition parsing, state edits, and save planning live in `definition_editor.rs`
and are tested without a window, including save/reload and invalid-save checks.

### Behavior presets

In **Entity properties**, search **New behavior from preset** by name or description.
Choosing a preset asks for a unique behavior name (a portable filename stem without
`.ron`) and offers **Assign to selected entity** when an entity is selected.
**Create behavior** adds an independent draft; the pinned **Save** writes it to
`assets/behaviors/<name>.ron` together with any assignment. Cancel leaves the draft
unchanged. Presets never change when their instantiated behaviors are edited.

Each `.ron` file in `assets/presets/` contains exactly these fields:

```ron
(
    name: "Patrol",
    description: "Walk between the placement's patrol endpoints.",
    behavior: (
        initial_state: "patrol",
        states: [
            (name: "patrol", actions: [patrol(speed: 80.0)], transitions: []),
        ],
    ),
)
```

`behavior` is a full definition using the existing actions, conditions, state
links, and optional transition `invert` field described above. Saved behavior
files keep their existing format: only `initial_state` and `states`, with no
preset metadata or reference. No new runtime actions or conditions are introduced.
The catalog is validated when opening the editor: names and descriptions must
be nonblank, preset names must be unique (ignoring surrounding whitespace and
case), and the full behavior must pass normal validation. Errors identify the
source file and invalid metadata, state, or parameter; parse errors include line
and column. A missing presets directory gives an empty list for older projects.
Reopen the editor after changing preset files.

Shipped presets:

- **Patrol** walks between placement endpoints.
- **Chaser** starts chasing below 180 pixels and gives up above 300 pixels.
- **Charger** detects below 240 pixels, telegraphs for 0.6 seconds, moves toward
  the player at 420 pixels/second for up to 0.8 seconds, then rests for 1 second.
- **Skittish** flees below 140 pixels and stops beyond 220 pixels.
- **Stationary** has an idle state with no actions or transitions.

### Editor manual check

- Shrink to 640×360 (also inspect a panel-only 360×240 window), then enlarge.
  Scroll through every section and confirm no control is covered and Save stays visible.
- Use both the wheel and draggable scrollbars, including result lists longer than
  eight rows. Reach the final entry, including **Item: Dash**; filter to zero matches
  and confirm **No matches** and the count.
- Open every dropdown near the panel bottom; check above/below placement, last-row
  selection, dismissal, and long-name tooltips.
- Type a very long entity/state name. Check wrapping/truncation and field scrolling;
  enter an invalid number or raw RON, collapse its section, and confirm Save refuses it.
- Place an enemy right beside the panel edge. Click and drag panel controls and
  scrollbars; confirm no placement/movement/deletion leaks onto the canvas. Then
  select/drag/delete on the canvas, move patrol handles, edit exits/arrival spawn,
  create/open a room, edit properties/actions/transitions/raw RON, and save/reopen.
- Search presets, create and assign a named behavior, edit Key values, and save/reopen.
- Assign the jump/hover/slam/blast example above to an enemy, edit every parameter
  in Key values and structured rows, and save/reopen. Confirm the leap samples
  once, holds in the air, lands, and shows warning circles before blast damage.
  Assign one behavior to two entity definitions; confirm the usage count and copy
  warning appear before edits, then save a copy and check only the selected entity
  switches. Check **Edit shared behavior** and both advanced views as well.

Enemy attack actions share the generated action registry and pure attack runtime.
`jump_to` uses an authored air time and arc height; the target is sampled when
wind-up starts. For compatibility, `jump_to(speed: ..., dx: ..., dy: ...)` still
loads the legacy linear jump. New and legacy slam/blast forms are distinguished
by their parameter fields, so existing enemy and boss documents retain their rules.

Attack authoring notes:

- All five attack types have an explicit `telegraph_secs`; `0.0` opts out of
  their built-in wind-up. An immediately preceding `telegraph` also previews
  the next attack's outline.
- `AttackBox.size` is full width/height. A Circle uses width as its diameter;
  height is retained for switching back to Rect. Offsets and knockback vectors
  use world axes. Lunge stays active for `duration_secs`; landing/impact attacks
  use `AttackBox.active_secs` before their recovery starts.
- `jump_to.arc_height` is editable and defaults from `data/attack_motion_defaults.ron`.
  Jumps and projectiles respect the authored room ceiling, side walls, and floor.
- Knockback adds the authored vertical velocity impulse and horizontal movement
  for `knockback_secs`, which defaults from the same motion data file. Immunity
  rejects both damage and knockback together.
- Projectile `spread_degrees` is the entire fan angle, so three shots at 15 degrees
  have offsets -7.5, 0, and +7.5 degrees. Horizontal aim samples the player's side.
- Warning `feedback` contains editable alpha, line width, flash cycles, and RGB
  tint. Defaults live in `data/attack_feedback_defaults.ron`; alpha starts at 0.3.
  Every nested attack/feedback value has a labeled editor field and staged-save
  validation. F5 replaces enemy attack state and projectiles with the edited data.

Manual attack check: run `cargo run`; observe Charger/Hopper/Shooter warnings and
recovery, verify projectiles and slam hits, edit each action's nested fields in F2,
save, then press F5 and confirm the edited attacks run with the new values.


### Action poses

`assets/poses/<name>.ron` stores `(offsets: {"head": (x, y), ...})` in local
box fractions relative to rest. The seven semantic points are head, torso,
hand_l, hand_r, foot_l, foot_r and tail. A template's `pose_map` translates
these names to its own points; absent parts are skipped. Facing mirrors the
existing body transform once. Pose state never enters damage or collision APIs.

Attack actions have optional `telegraph_pose`, `active_pose`, `recovery_pose`,
plus `pose_blend_ratio` (0.8) and `strike_stiffness_multiplier` (3.0). All shipped
attack bindings are authored in behavior/preset data; generic editor actions
start with empty bindings so they can be used by bodyless enemies. Separate
`telegraph` actions use the following attack's binding and their own duration.
Phase changes set/clear existing point targets. Pose targets use the exact
critically damped spring solution with settling time derived from telegraph
duration; unposed points retain their template follow/idle rules. Zero-duration
strikes use the template's visual-only `instant_strike_secs` spring impulse.
This does not extend attack timing or delay the next action.

Each body's `hurt` fields select the damage pose, duration (0.15 seconds),
blend time, flash duration, color and strength. Hurt overrides action poses,
then resumes the current action target or idle. Warning outlines retain their
30% alpha, with a separate `warning_tint_strength` (0.25).

In the editor, phase and hurt poses have catalog dropdowns. **Pose assets**
creates and edits pose offsets; **Body visual → Pose following** edits semantic
mapping and spring tuning. Save validates the complete pose/body/action catalog
before staging any files; F5 reloads it atomically and preserves point positions.
F3's legend shows the nearest body's current pose using the existing HUD size.

Manual check: watch Charger windup/strike, Hopper crouch/leap/stomp and Shooter
windup/recoil facing both ways; damage each to check the white flash and hurt
pose. Toggle F3 at different zoom levels. Edit a pose offset and binding, save,
then press F5 and confirm the new motion with unchanged attack timing/hitboxes.

### Shared action execution

`action_runtime` owns per-action timers, motion and attack effects. Entity adapters
supply geometry, player input, deterministic randomness and an output sink.
Behavior transitions and boss encounter scheduling keep their own sequence cursors.
Legacy boss targeting, nonblocking glides and arena RNG sampling remain adapter
policies so extracting the executor does not change existing frame timelines.
