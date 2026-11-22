# Puzzle pieces: breakable walls, switches and barriers

A room can hold three kinds of piece, each a rectangle placed in the room file and in the editor:

- **Breakable wall**: solid. It goes away after the nail has hit it `hits` times (1 to 9), one hit per swing.
- **Switch**: not solid, a small rectangle with a number 1 to 8, unique in its room. A nail swing that
  reaches it turns it on, for the rest of the run. It cannot be turned off again.
- **Barrier**: solid. It goes away while the switch with its number is on. That switch must be in the
  same room.

Code: `src/puzzle.rs` (file form, checks, the live pieces, what a swing does), `src/game/world_puzzles.rs`
(what a run has broken and switched on), `src/scene_puzzles.rs` (drawing), `src/editor_puzzle.rs` (editor
edits). The shape is the one of terrain hazards, `docs/hazards.md`.

## Rules in play

- **Collision.** Each tick the player collides with the room's terrain plus the pieces still standing:
  walls not yet broken and barriers whose switch is off (`RoomPuzzles::collision`). A room without pieces
  borrows its terrain and builds nothing. Landing on a piece, bumping the head on one and being stopped
  by one while walking or dashing all work as for any solid. A player standing on a barrier that opens
  falls. Nothing is ever merged into `Room::terrain`, so `RoomLayout::from_room` still gives back what the
  file said.
- **Strikes.** Every swing, in every direction (sideways, up and down), is checked against the pieces
  while its box is active. A swing hits a wall at most once, however many ticks its box overlaps it
  (`Attack::id` tells swings apart). A box must share more than an edge with the piece: touching does not
  count. One swing can hit several walls and switches, each once. The nail is not stopped by solids, so a
  swing reaches a piece behind a thin wall.
- **No pogo.** A downward swing that hits a wall does not bounce the player. Walls and switches are not
  enemies and never count as a hit for pogo, health or mana.
- **Notice.** Turning a switch on shows "Switch N on" with the notice the locks use (`Locks::notice`). A
  broken wall has no notice. No sound, no particles, and no `GameEvent`: the only fitting one, `EnemyHit`,
  means an enemy was hit.
- **Look.** `scene_puzzles` draws a wall as a stone block with one crack line for each hit it has left, a
  switch as a plate with a lever (grey knob while off, a knob in the switch's own tint, leaning the other way,
  once on) and a barrier as iron bars tinted like their switch. A broken wall and an open barrier are not
  drawn. The look never changes collision.
- Enemies ignore the pieces: they are swept against the terrain only.

## What a run remembers

`GameState::puzzles` (`src/game/world_puzzles.rs`) remembers walls the nail broke and switches it turned on,
like `exit_lock::OpenedExits` remembers opened doors.

- Kept for the rest of the run: leaving the room and coming back finds the wall gone and the barrier open.
- A piece is remembered by its room and the numbers of its rectangle, never by its place in a list (the
  editor can reorder and delete pieces).
- The editor's Save rebuilds the saved room from its file and then breaks and switches on again what the
  run had. A piece the author moved, resized or deleted is forgotten, so it is whole and off again; saving
  another room leaves this room's pieces alone. A wall that was hit but not yet broken is whole again after a
  Save (the hits taken are not remembered).
- A new run (the run reset, for example after dying) makes every wall whole and every switch off.
- Nothing is written to disk. The room file only says where the pieces are.
- A **Test room** (F6) is a sandbox: its pieces start whole and off, nothing it does is remembered, and
  ending it (F2) puts back the saved room with what the run had done to it.

## Room file

`breakables`, `switches` and `barriers` are lists in the room's layout, each written only when it is not
empty, so older room files load unchanged. Unknown fields are refused. `rect` is `(x, y, width, height)`:

```ron
breakables: [
    (rect: (400.0, 396.0, 32.0, 64.0), hits: 3),
],
switches: [
    (rect: (700.0, 436.0, 24.0, 24.0), id: 2),
],
barriers: [
    (rect: (500.0, 364.0, 24.0, 96.0), switch: 2),
],
```

Checks (`RoomLayout::validate_geometry`, which the editor, Test room, Save and startup all run): finite
numbers, at least 8 px wide and tall, fully inside the room; a wall's hits 1 to 9; switch numbers 1 to 8
and not repeated in a room; every barrier's switch exists in the same room (the error names the barrier,
for example "barrier 0 needs switch 3, which this room does not have"); the room's spawn point must not be
inside a piece. When the catalog is checked (`RoomDraft::validate`), the arrival point of every other
room's exit that leads into the room must not be inside a piece either.
Pieces may overlap each other.

## Editor

- **Add breakable wall**, **Add switch** and **Add barrier** (Room tools, below Add hazard) put a standard
  piece on the floor next to the spawn point (to its right, else to its left) and select it: a 32 x 64 wall
  of 3 hits, a 24 x 24 switch with the lowest number still free (a room has at most eight), a 24 x 96
  barrier opened by the room's first switch. **Add barrier** in a room without a switch is refused and says
  to add a switch first. Each button is one undo step. New pieces start at the same spot: drag them apart
  (a barrier is on top of a switch, a switch on top of a wall, and of two of a kind the one added last).
- Drag a piece by its box like a hazard: it moves by its center and stays inside the room. Delete, the
  Delete key and the Erase tool (or right-click) work as for other markers.
- Selecting a piece shows its section. **Piece width** and **Piece height** (the center line and the bottom
  edge stay put, so a piece grows upward from the floor), and **Hits** (a wall, 1 to 9), **Switch number**
  (a switch, 1 to 8), or **Opened by switch** (a barrier: a dropdown of the switches of this room). A drag
  of a field is one undo step; a choice is one step. A refused value (for example a switch number that
  another switch has, or one a barrier uses) is not applied and the reason is shown.
- A switch that a barrier still needs cannot be deleted or renumbered: the edit is refused and the message
  names the barrier. Delete the barrier first, or point it at another switch.
- Canvas markers: brown for a wall, with its hits ("3 hits"); teal for a switch with its number ("S2"); slate
  for a barrier with the number of the switch that opens it ("S2" too, so a pair matches by eye). None is
  magenta, the hazards' color. The legend lines for them show in the panel once the room has a piece.
- Every change marks the room unsaved (the unsaved-changes guard asks before closing or switching rooms),
  shows in Test room (F6) before it is saved, and is written by Save.

## Known limits

- A switch is only ever turned on: no toggling, no timed switches, and none that opens an exit.
- No moving or crumbling platforms; no wall that needs an ability (Dash and the like) to break.
- No pogo off a breakable wall (see above); no sound or particles; no `GameEvent`.
- The dungeon checker (`src/dungeon_check.rs`) does not model areas inside a room, so a barrier that hides
  the way to its own switch is not caught. Use Test room to prove a room can be finished. The reach preview
  and the gate marks ignore the pieces as well.
- Enemies ignore the pieces: they walk through a barrier or a wall.
- F5 (reload) does not rebuild a loaded room's terrain, hazards or pieces, only its enemies and look: F2 and
  Save, or a restart, to see a changed piece.
- No resize handles on the canvas (use the width and height fields).
- A piece changed by the author is forgotten by a run (see "What a run remembers"), by design.
- No piece is in the shipped rooms yet.

## Manual check

1. F2, **Add breakable wall**: a brown box labelled "3 hits" appears and its fields show. Drag it into
   the room, then Test room (F6): it is drawn as stone with three cracks. Walk into it: you stop. Face it and
   swing (X): a crack disappears; the third separate swing breaks it and you walk through. Hold the key down through one long
   swing: that is still one hit.
2. Swing down onto a wall from above in the air: you do not bounce.
3. F2, **Add switch**, **Add barrier**; drag the barrier into a corridor and the switch (teal "S1", slate
   "S1") somewhere the player can reach. Test room: the barrier blocks; strike the switch: "Switch 1 on"
   shows and the barrier is gone; walk through.
4. Leave the room by an exit and come back: the barrier is still open and a broken wall is still gone.
5. F2, Save, then play: still open and broken. Move the barrier a little, Save: it is closed again.
6. Die and start a new run: the walls are whole and the barrier is closed.
7. Test room with an unsaved barrier: it is there and blocks you without saving; F2 and the saved room has
   none.
8. Drag a piece across the room and release: Undo puts it back in one step. Resize with the height field
   and drag another field: each is one Undo.
9. Select the switch that the barrier uses and Delete: refused, the message names the barrier. Delete the
   barrier, then the switch.
10. Close the editor with an unsaved piece: the Save / Discard / Cancel guard asks.
