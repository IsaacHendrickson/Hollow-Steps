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

## Where the code is

`src/dungeon_map.rs` (types and `build`), `src/dungeon_map/place.rs` (the cell rules),
`src/dungeon_map/discovery.rs` (`MapDiscovery`), tests in `src/dungeon_map/tests.rs`;
`src/game/map_link.rs` (`GameState` owns the discovery and builds the map of the live rooms), tests
in `src/game/map_tests.rs`; `Room::map_cell` (`src/room.rs`) and `RoomLayout::map_cell`
(`src/editor.rs`).
