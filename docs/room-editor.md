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

Pick a tool in the **Tools** section (keys V, P, T, E with the canvas focused). Choose an
entity in the **Entity** list, pick **Place**, then click the room. **Select** moves
entities, the blue respawn marker, or a green exit span, and selects solids (see
"Terrain tools"). Right-click an enemy/item/boss/exit/solid, use **Erase**, or select
it and press Delete to remove it.
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

## Canvas view, grid and room size

The canvas transform is `CanvasView` in `src/editor_view.rs`. Until the user zooms or
pans it follows the room (fit to the space between the header and footer); after that
the view is kept, and switching to a destination room view refits. The wheel zooms
about the cursor only when the cursor is over the canvas, so over the panel it still
scrolls the panel. Middle-drag, or Space+left-drag while no text box has focus, pans;
the room cannot be panned out of the viewport. **Home** or **Reset view** refits.

The **Grid and snap** checkbox (off by default) draws a grid and snaps placement and
dragging to the **Snap size** (default 8 px), measured from the room's top-left corner.
Grid lines thin out when zoomed far out; snapping always uses the real size.

**Room size** edits x, y, width, height and floor_y. `room_size::resized` validates the
whole draft with the normal room rules, so a change that leaves the spawn, items,
exits, terrain, enemies or boss outside the room is refused with the usual message and
the fields snap back. Exit arrival points in *other* rooms are checked at Save, as before.

## Terrain tools

The **Tools** section replaces the old Place and Select / drag buttons. Keys (canvas
focused, no text box focused, no Ctrl or Alt): **V** Select, **P** Place, **T** Terrain,
**E** Erase. Place needs an entity or item chosen in the Entity list first.

- **Terrain**: drag a rectangle to make a solid (snapped when Grid and snap is on, kept
  inside the room). Nothing is added until you release; the green preview shows the size.
  A plain click adds nothing; a solid under 8 px wide or tall is refused.
- **Select**: click a solid to select it (white outline, size label, eight handles). Drag
  the body to move it; drag a handle to resize it, which stops at 8 px and at the room
  walls. Where a marker overlaps a solid a click selects the marker, and a handle of the
  selected solid wins over both.
- **Erase**: click an enemy, item, boss, exit or solid to delete it (markers first, the
  spawn and patrol handles are skipped). Right-click and Delete (with the canvas focused)
  do the same for the thing under the cursor or the selection; **Delete selected** too.

Rules live in `src/editor_terrain.rs` (add, move, resize, delete, hit tests) and
`src/editor_tools.rs` (tool enum, key map, drag state). Every edit is validated with
`RoomLayout::validate_geometry` on a copy, so a solid over the spawn, an item or an exit,
or outside the room, is refused with the usual message; a drag the rules refuse stays at
its last valid spot. Solids save to `layout.terrain` and undo/redo covers them (a drag
is one step). Terrain tools work in the room view, not the arrival-spawn view or the
Entity properties panel.

## Unsaved changes

Closing the editor (F2), **Open room** and **New room** go through `UnsavedGuard`
(`src/editor_guard.rs`). A session with `dirty` set opens a modal **Unsaved changes**
window: **Save** saves (with the same invalid-text check as the pinned Save button) and
then continues, **Discard** continues without saving, **Cancel** (also Escape or a click
outside the window) keeps the session. If the save fails the window stays open and shows
the error. A clean session acts immediately. While the window is open the panel and
canvas take no input, and further F2 / room requests are ignored.

## Undo and redo

`History` (`src/editor_history.rs`) keeps whole `RoomDraft` snapshots: up to 100 undo
steps (the oldest are dropped) and a redo stack. A snapshot is taken before each
discrete change: placing, deleting, **Add exit**, **Set exit target**, **Delete
selected**, and each accepted **Room size** edit. Dragging a marker on the canvas, or
dragging a Room size field, is one step from press to release. Any new edit clears redo,
and opening or creating a room starts with an empty history.

Undo is **Ctrl+Z**; redo is **Ctrl+Y** or **Ctrl+Shift+Z**. The keys work only while the
canvas has focus (you last clicked it) and no text box or dialog wants the keyboard. The
**Undo** and **Redo** buttons in the **History** section do the same and are greyed out
when there is nothing to do or a drag is in progress. Undo and redo mark the session as
having unsaved changes and drop the current selection.

Entity, behavior and boss *definition* edits (`EditorSession::content`) are not part of
history; the tooltip on the buttons says so. A step whose snapshot uses an entity or boss
that was renamed or deleted since is refused with a message instead of being applied, and
that step is dropped; the steps before it still work.

### Manual check: navigation, unsaved changes and undo

Run `cargo run`, press F2 and use a room with a few entities.

- Wheel over the canvas zooms about the cursor; wheel over the panel still scrolls the
  panel. Middle-drag and Space+drag pan. **Home** and **Reset view** refit. Zoom stops at
  both ends.
- Turn on **Grid and snap**, set Snap size to 16, place and drag an enemy: it lands on
  grid points. Turn it off and confirm free placement.
- Resize the window and wrap a long status message (try a bad Room size): the footer
  follows the window and the message is not clipped.
- In **Room size**, drag Width up (fine) and try to make it smaller than an entity's x
  (refused with a message, field snaps back).
- Make a change, then F2, **Open room** and **New room**: each asks Save / Discard /
  Cancel. Cancel keeps everything; Discard drops the edits; Save saves first. With no
  changes they act at once. Make Save fail (for example a read-only room file) and
  confirm the dialog stays open with the error.
- Place an enemy, drag it across the room and release: Ctrl+Z puts it back where it was
  in one step, another Ctrl+Z removes it, Ctrl+Y brings it back. Delete it with Del, undo.
  Add an exit, retarget it, undo twice. Drag Width, release, undo once.
- Click a text box in the panel and press Ctrl+Z: the canvas does not change. Click the
  canvas and it does. After undo, make a new edit and confirm redo is gone.
- Hover **Undo** and **Redo** and read the tooltip. Open another room: Undo is greyed out.

### Manual check: terrain tools

- Press T (click the canvas first), drag a rectangle: a green preview with its size, then
  a solid on release. Ctrl+Z removes it, Ctrl+Y brings it back.
- Turn on Grid and snap (16) and draw, move and resize: edges land on grid lines.
- Press V, click the solid: handles appear. Drag the body (one Ctrl+Z puts it back), drag
  a corner and an edge handle, drag a handle past the opposite edge (stops at 8 px) and
  past the room wall (stops at the wall).
- Drag a solid over the blue spawn marker, an item or the green exit: it stops and the
  footer explains why. Draw one across the spawn: refused, nothing added.
- Press E and hover a solid (red highlight), click it: it goes; Ctrl+Z restores it. Click
  an enemy or item with Erase: it is deleted instead of the solid behind it.
- Right-click a solid, and select one and press Delete. Click **Delete selected**.
- Save, press F5 or restart: the solids are loaded in the game and stop the player.
- Open the Entity properties panel (or the arrival-spawn view): drawing is off there.
