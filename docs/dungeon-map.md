# Dungeon map: design note

A map of the rooms of a large dungeon, in play (press `M`) and in the editor (Room tools,
**Dungeon map**). This note covers the rules; the code is listed at the end.

## Where a room goes

Every room sits in one cell of a grid, written `(column, row)`, rows growing downward like the
game's y axis. A room file may name its cell:

    map_cell: Some((2, -1)),

`map_cell` is optional (older room files load unchanged), is left out of the file when unset, and
each number must be from -64 to 64 (checked with the rest of the layout when a room loads or
saves). A new room does not copy its template's cell.

`MapLayout::build` (pure, `src/dungeon_map.rs`) places the rooms:

1. A room with a cell keeps it. When rooms name one cell, the lowest room id keeps it and each
   other is reported as a **conflict** and moved to the nearest free cell.
2. The rooms placed so far are walked in room id order. Each exit puts its target, if it has no
   cell yet, next to the exit's room: a Right exit one cell right, Left one left, Bottom one down,
   Top one up. A door says nothing about direction, so its target takes the nearest free cell.
   Then the rooms that walk placed are walked the same way, until nothing new is placed. With no
   cell named anywhere, the room with the lowest id is put at (0, 0) to start from.
3. A room no walk reaches goes in a row below everything, in id order.

A target cell that is taken moves the room to the nearest free cell. "Nearest" counts cells (a
diagonal neighbor is two away); equal distances are tried up, left, right, then down, so the result
never depends on chance or on the order the rooms were listed in.

Each exit that leads to another room becomes a **link** from the room's cell to the target's cell.
It is flagged one-way when the target has no exit back, and carries the exit's key number if the
exit is locked.

## What the player has seen

`MapDiscovery` is the set of room ids visited this run. `GameState` records the room the player
enters (and the one the game starts in, and the hub after a run), and the room the player stands
in at the start of every tick, which also covers the editor moving the player. It is cleared when a
run starts and when one ends. It is keyed by room id, so an editor Save, which rebuilds every room,
and F5 keep it. A test room records nothing. With no hub in `run.ron` nothing ever clears it: it
starts with the start room and grows as the player walks.

## The map screen

Press `M` (the **Map** action: rebindable in Settings like Inventory, shipped key `M`, no
alternate; a `keybinds.ron` from before the action existed loads with `M`, and **Reset keybinds**
brings it back). The screen opens over the world and freezes it like the inventory. `Esc` or the
Map key closes it; a run ending or the editor opening closes it too. It does not open while the pause
menu, the inventory, the shop, the result screen, the editor or a test room is up, and the inventory
does not open over it.

`PlayMap` (pure, `dungeon_map/play.rs`) decides what is drawn: the rooms visited this run (filled,
with their name), the rooms one exit away from one of those (an empty box, no name), and the current
room (marked). A room that is neither is not drawn at all. Links start at a visited room, so an
exit is shown once the room it leaves has been seen: an arrow head when it is one-way, and the key
number of a locked exit.

`screen_layout` and `MapGrid::fit` (`dungeon_map/grid.rs`) turn cells into rectangles in the
960x540 virtual screen: the panel is inset from the view, the grid is scaled to fit the panel
area with a cap on the cell pitch (`MAX_PITCH`), centered, and the box is `BOX_FRACTION` of a cell.
One room gives one centered box of at most the cap; 40 rooms shrink to fit and never leave the
panel. The panel is placed with `camera::anchored`; `src/hud_map.rs` only paints the result, in
virtual coordinates (no window-size constants).

## In the editor

**Map cell** (Room tools, under Check dungeon) is the room's own cell. The **Auto** box leaves
`map_cell` out of the room file, so the exits place the room. Turning Auto off starts at the cell the
map shows now (so nothing jumps) and shows two number boxes, X and Y (-64 to 64): drag one or type in
it. The panel always says where the map puts the room ("Shown at (2, -1)"), and says so when another
room has the same cell. Any change is one undo step (a whole drag is one) and marks the room unsaved;
Save writes it with the rest of the room.

**Dungeon map** (the button under it) opens a window with every room the session knows: the saved
rooms, with the room being edited and every parked draft (also a new room with no file yet) in place of
their saved layouts. It uses the same `MapLayout::build` and `MapGrid::fit` as the screen in play, so a
room sits where the player's map will put it. The room being edited has a gold outline. A room that
shares a cell with another is red and says "same cell"; a room that no exit of any room leads to says
"no way in" (the first room usually does; it is a hint, not an error). Exits are lines between the
boxes, with an arrow head when one-way. Clicking another room opens it through the unsaved-changes
guard, exactly as **Open room** does (`PanelResponse::open`); clicking the room being edited or empty
space does nothing. The window closes when a room opens, and is hidden while an exit link is open.

**Check dungeon** reports two rooms that name the same cell, naming both ("First and Second both
have the map cell (2, 2); the map moves Second"), one problem for each room that was moved
(`docs/dungeon-check.md`).

## Known limits

- Rooms are drawn as equal boxes, not to scale and not as their real shape.
- The map screen does not scroll or zoom; a large dungeon is drawn smaller to fit.
- There is no minimap in the corner of the play screen.
- The player cannot place markers on the map.
- Items and bosses are not shown on the map.
- A room that is only reachable through a room that no walk reaches goes in the bottom row.
- The shipped hub has no exit leading into it, so it sits in the bottom row.
- A one-way arrow on the map of a visited room shows that the room beyond it has no exit back, even
  before the player has seen that room.
- A `keybinds.ron` that binds another action to `M` is refused by the usual rule that a key is used
  once; the file needs that action moved (or `map` named in it).
- The hint line under the pause menu does not mention `M`.

## Manual playtest checklist

The drawing in `hud_map.rs` and the egui window are not covered by tests that open a window. Check by
hand (`cargo run`):

1. In the editor (F2), link four rooms with exits on different sides (Right, Left, Top, Bottom), then
   open **Dungeon map**: each room sits where its exit says, a gold outline is on the room being
   edited, and a one-way exit has an arrow head.
2. Give one room a cell by hand (Auto off, drag X and Y): the window and the "Shown at" line follow;
   Undo puts it back in one step; Save, then close and reopen the window.
3. Give two rooms the same cell: both turn red, Check dungeon names both.
4. Click another room in the window: it opens (with unsaved changes, the Save / Discard / Cancel
   window comes first).
5. Start a run and press `M`: only the first room shows (and the empty boxes of rooms one exit away).
   Walk through an exit: the next room appears and the current room mark moves. `Esc` or `M` closes it.
6. Lock an exit with a key: the key number shows on that exit once its room is visited.
7. Die (or abandon) and start a new run: the map is empty again.
8. Settings (Esc, Settings): rebind **Map** to another key; the new key works and `M` does not.
   **Reset keybinds** brings `M` back.
9. F2, Save, F2 again: the map is still there with what was seen.
10. `M` does nothing in the editor, in a test room, over the pause menu, the shop and the result screen.

## Where the code is

`src/dungeon_map.rs` (types and `build`), `src/dungeon_map/place.rs` (the cell rules),
`src/dungeon_map/discovery.rs` (`MapDiscovery`), `src/dungeon_map/grid.rs` (screen layout),
`src/dungeon_map/play.rs` (`PlayMap`), tests in `src/dungeon_map/tests.rs`, `grid_tests.rs`,
`play_tests.rs`; `src/hud_map.rs` (drawing, bin only); `src/game/map_screen_tests.rs`;
`src/dungeon_map/editor.rs` (`RoomDraft::set_map_cell`, `EditorSession::dungeon_map`, `EditorMap` for the
window), `src/editor_panel/map_panel.rs` (the Map cell row and the button), `map_window.rs` (the
window), tests in `src/dungeon_map/editor_tests.rs`, `src/editor_panel/panel_tests/map.rs` and
`tests/editor_map.rs`;
`src/game/map_link.rs` (`GameState` owns the discovery and builds the map of the live rooms), tests
in `src/game/map_tests.rs`; `Room::map_cell` (`src/room.rs`) and `RoomLayout::map_cell`
(`src/editor.rs`).
