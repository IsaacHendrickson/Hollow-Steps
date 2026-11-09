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
bosses and coins remain in `src/room.rs`.
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
stable numeric IDs, independent of list order. Coins and bosses remain existing
gameplay content and are outside this editor's scope.

Choose an entity in the **Entity** list, click **Place**, then click the room.
Choose **Select / drag** to move entities, the blue respawn marker, or a green
exit span. Right-click an enemy/item or select it and press Delete to remove it.
Select an exit, choose a **Target**, and click **Set exit target**; use
**Arrival spawn** to drag its purple arrival marker in the target
room's coordinate space. **Back to room** restores the current-room view.
**Add exit** creates a span on the chosen **Edge** so new rooms can be connected.
**Save** applies the draft to gameplay and disk; errors remain visible in
the footer and preserve the draft, running room, and previous file.
Closing with F2, opening another room, or choosing New room discards unsaved
changes. A star next to the room ID marks unsaved changes. Saving resets that
room's editable enemies/items and places the player at its spawn, while keeping
inventory, coins, and boss progress. F5 retains its existing enemy-only reload
behavior for already loaded rooms; restart or use editor Save to apply layouts.


Select an enemy, then choose **Enemy properties** in the right panel. Edit its
shared name, health, width/height, contact damage, and behavior file stem (without
`.ron`). These edits affect every instance using that definition. Renaming an
entity updates references in all saved rooms while retaining its entity filename.
Use **Room tools** to return to placement controls. Scroll the panel for controls
below the visible area.

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

In **Enemy properties**, search **New behavior from preset** by name or description.
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
