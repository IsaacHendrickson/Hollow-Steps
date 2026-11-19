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

## Room look

The **Room look** section (under Room size) chooses how the room's solids and the frame around it
are drawn: a terrain preset, four color overrides with a Reset each, Moss, Draw boundary, and a
Seed offset with Reroll. The canvas shows the result live, using the same code as the game, and
it saves to `layout.terrain_style`. Each change is an undo step, except that a drag in the color
picker or on the Seed offset box is one step. Test room (F6) plays the unsaved look. The
presets and the style format are in `docs/terrain-look.md`; the rules are in
`src/editor_look.rs` and the panel in `src/editor_panel/look_panel.rs`.

## Test room

**Test room (F6)** (pinned above Save, or F6 with the editor open) plays the room as it is
in the editor, unsaved edits included: `GameState::start_test_room` builds a `Room` from
the draft with `RoomDraft::build` (using the session's preview definitions, and placing the
boss if there is one), puts the player at the room's spawn with full health and closes
the editor view. **F2** returns to the same session: draft, unsaved flag, message,
selection and undo history are as you left them.

While a test runs the built room stands in for the saved room with that id (or is added
temporarily for a New room), and the player, inventory, coin count and boss timers are
parked and restored, so playing never changes a saved room or the session. Nothing is
written to disk. Exits are inert (they would lead into saved rooms); F5 reload and Open
room are refused until F2. A draft that is not a valid room (for example a solid over the
spawn that bypassed the editor) does not start; the reason appears in the editor message.

## Unsaved changes

Closing the editor (F2), **Open room** and **New room** go through `UnsavedGuard`
(`src/editor_guard.rs`). A session with `dirty` set opens a modal **Unsaved changes**
window: **Save** saves (with the same invalid-text check as the pinned Save button) and
then continues, **Discard** continues without saving, **Cancel** (also Escape or a click
outside the window) keeps the session. If the save fails the window stays open and shows
the error. A clean session acts immediately. While the window is open the panel and
canvas take no input, and further F2 / room requests are ignored.
