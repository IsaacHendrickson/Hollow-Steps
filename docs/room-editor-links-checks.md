## Undo and redo

`History` (`src/editor_history.rs`) keeps whole `RoomDraft` snapshots: up to 100 undo
steps (the oldest are dropped) and a redo stack. A snapshot is taken before each
discrete change: placing, deleting, **Add exit**, **Set exit target**, **Delete
selected**, each accepted **Room size** edit and each **Room look** change. Dragging a marker on
the canvas, dragging a Room size field, or dragging in a room color's picker or on the Seed
offset box, is one step from press to release. Any new edit clears redo,
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
  Choose Key 3 in **Locked by** on an exit, undo once: it is unlocked again.
  Change a room color (drag in the picker), release, undo once: the color is back in one step.
- Click a text box in the panel and press Ctrl+Z: the canvas does not change. Click the
  canvas and it does. After undo, make a new edit and confirm redo is gone.
- Hover **Undo** and **Redo** and read the tooltip. Open another room: Undo is greyed out.

### Linking exits

An exit stores the room it leads to and the absolute point the player arrives at in that
room (`target_room`, `spawn`). Linking fills both in from the room editor, for the exit in
room A you are editing and a room B it should lead to (`src/exit_link.rs`, `src/exit_arrival.rs`).

1. **Add exit and link...** places an exit on the chosen Edge and opens a room chooser at once;
   **Link...** on the selected exit opens the same chooser (the panel also says "Leads to
   <name>"). The chooser lists the saved rooms and unsaved drafts by name, then **New room...**
   (asks for a name with the room-name rules, then makes an unsaved room shaped like A). Cancel
   removes an exit that was just placed and changes nothing for an existing one. A itself is not
   a valid target; the chooser says so.
2. B opens in the editor with the banner "Linking A exit to B", a **Two-way** checkbox (on) and
   Cancel. Only exits can be selected, placed and dragged in B (the other tools, delete, undo,
   Save and Test room are off). Select an existing exit (outlined in yellow) or press
   **New exit here** and click to place one (grid snap, drag to adjust); a new exit leads back
   to A, so Two-way stays on for it. The picked exit shows a purple **Arrival** marker, where
   the player will arrive in B; drag it before **Connect** to choose the spot (refused where
   the player does not fit). **Connect** finishes.
3. A's exit leads to B and arrives at the marker (next to the chosen exit unless it was
   dragged). With Two-way, B's exit leads to A and arrives next to A's exit. The arrival is searched from the exit's inner face and is never
   inside a solid, outside the room or inside any exit. If B's exit led somewhere else, the
   message names the old target. The editor returns to A with the same selection and view and
   "Linked A <-> B".

Cancel at any step (or F2) returns to A with every draft as it was; a room the flow created is
discarded and its id and name are free again. From the name window, Cancel goes back to the
chooser. A is kept in memory, untouched, while B is open, and every room the flow touched stays an
unsaved draft until the normal **Save**, which checks all of them first and writes all or none.
The unsaved-changes window lists every unsaved room by name. The link is one undo step in A and
one in B (each room keeps its own history).

Known limit: moving or deleting a linked exit later does not update its partner; use **Link...**
on it again.

A dragged arrival belongs to the exit it was set for (picking another exit shows that exit's
own marker), is forgotten on Cancel, and gives way to the spot next to the exit if the exit is
dragged over it. It is only held until **Connect**, so dragging it does not mark B unsaved.

### Arrivals of other rooms' exits

Every exit in another room that leads into the room you are editing draws its arrival point
here as a purple, player-sized marker labelled "from <Room>" (`src/editor_inbound.rs`). Drag it
to move where that exit arrives, without opening the other room or the arrival-spawn view.

- The point must fit like any arrival: inside the room, not inside a solid, not inside an
  exit. A drag to a place that does not fit is refused with the reason and the marker stays.
  If the room later changes under it (a solid or an exit drawn over it) the marker turns red
  and its label says why; Save still refuses an arrival inside an exit or outside the room.
- The exit belongs to the other room, so that room's draft changes: it is kept in memory
  (like the rooms a link flow touches) and marked unsaved: the room tools say "Unsaved:
  Second". This room gets no star from the drag, and the drag is not an undo step; the other
  room's arrival is not undoable.
- **Save** writes the other room together with this one, checking them as a set (all or none).
  The unsaved-changes window lists the other room by name.
- Only exits from *other* rooms are listed (an exit that leads back into its own room is
  edited with **Arrival spawn**), and the markers are hidden while exits are being linked.
  Selecting one shows "Arrival from <Room>" in the room tools; it cannot be erased or deleted.

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

### Manual check: room look

- In **Room look** pick each preset: the canvas redraws at once with that preset's colors,
  moss, speckles and frame. Pick the one already chosen: nothing changes.
- Click a color button, drag in the picker, let go: the canvas follows, **Reset** lights up,
  Ctrl+Z puts the color back in one step. Open a picker and close it without touching it: the
  room is not marked unsaved and there is nothing to undo.
- Change a color, then pick another preset: the color override is gone and the seed is kept.
- Toggle **Moss** and **Draw boundary**; drag **Seed offset** and press **Reroll**.
- Zoom and pan: the look stays glued to the room. Open an exit's arrival-spawn view: the other
  room is drawn with its own look.
- Save, press F5: the look is unchanged. A room at the default look has no `terrain_style` in
  its file.

### Manual check: linked exits

Run `cargo run`, press F2.

- Select the first room's exit, press **Link...**, pick Second: Second opens with the banner,
  the yellow outlines and a panel with only Two-way, New exit here, Connect and Cancel. The
  tool buttons, Delete, Ctrl+Z, Save and Test room do nothing. Press **Connect** with no exit
  selected: the footer says to select one.
- Click Second's left exit and Connect: the editor is back in the first room with the same
  selection and zoom and "Linked First <-> Second". Save, close the editor and walk through
  the first room's exit: you appear beside Second's exit, not in it. Walk back through it.
- **Add exit and link...**, Cancel: the exit is gone. Do it again and pick **New room...**, name
  it, press **New exit here**, click near its edge, Connect. The unsaved window (F2) lists both
  rooms by name. Discard, then repeat and Save: both rooms are written, and walking through
  the first room's new exit and back works.
- Open the chooser and pick the room you are in: it refuses with a message. Uncheck Two-way, Connect to
  an exit that led elsewhere: only the first room's exit changes. Connect with Two-way to one that led
  elsewhere: the message names the old target.
- Place an exit over a solid in the target room: refused with the reason, nothing added.
- In the link step, click an exit in Second: a purple "Arrival" marker appears beside it.
  Drag it somewhere else in Second (it refuses a solid, an exit or outside the room), then
  Connect. Save, close the editor and walk through the first room's exit: you appear where
  you left the marker. Pick a different exit before Connect: its marker is where it was.
- Cancel (button and F2) from the chooser, the name window and Second: nothing changed and a
  new room made by the flow is gone.

### Manual check: arrivals of other rooms' exits

Run `cargo run`, press F2 in the first room (Second's left exit leads to it).

- A purple marker "from Second" shows where Second's left exit puts the player in this room.
  Click it: the panel says "Arrival from Second" and Delete does nothing. Drag it to another
  free spot; the room tools say "Unsaved: Second", the room gets no star, and Ctrl+Z does not
  move it back.
- Drag it onto a solid, onto the green exit span or outside the room: the marker stays and
  the footer says why. Draw a solid over it: it turns red with the reason in its label.
- Press F2: the unsaved-changes window lists Second. Discard, reopen: the marker is back where
  it was. Move it again and Save: close the editor, walk from Second into the first room: you
  arrive at the new spot. Press **Link...** on any exit: the marker is hidden until you finish.

### Manual check: test room

- Draw a platform, move the blue spawn onto it, press **F6** (or click **Test room (F6)**):
  the game view appears with an amber "TEST ROOM" line at the bottom, the player starts on
  the spawn, and the platform stops the player.
- Walk into the green exit: nothing happens. Pick up an item, take a hit, then press **F2**:
  the editor is back with the same draft, the "*" unsaved mark, the same selection, and
  Ctrl+Z still undoes your last edit.
- Close the editor (Discard) and play: the saved room has none of the test's changes
  (item still there, health full, no platform) and no file changed.
- New room, add a platform, F6, F2: the unsaved room is still a draft.
- Put a solid over the spawn by editing the RON by hand, open the room, F6: it does not
  start and the footer says why.
- Press F5 and Open room during a test: refused until you press F2. Resize the window and
  repeat: the view letterboxes as in normal play.

### Manual check: Double Jump, gates and the Reach preview

Run `cargo run`, press F2 in the first room. The reach numbers below are for the shipped
`player.ron` when this was written (jump 163 px high, double jump 266 px high); yours follow the file.

- In the Entity list pick "Item: Double Jump" (beside "Item: Dash"), press **P** and click the floor
  near the spawn: a pickup appears. It also shows in the inventory (**I**) once collected.
- Turn **Reach** on (View section). Three outlines appear around the spawn: light blue "without any
  ability ...", pink "with Dash ..." and a taller yellow-green "with Double Jump N px high / N px far".
  The Room tools panel says "the outlines are a jump with no ability, and with Dash and Double Jump".
- Draw a platform (**T**) whose top is above the light blue outline and below the yellow-green one.
  Press **F6**: without the pickup you cannot jump onto it. Press Up at the pickup, then jump and press
  Space again in the air: you reach it. The second press does nothing on the ground, and holding
  Space does not fire it; landing refills it.
- Back in the editor (**F2**), select an exit, tick **Needs Double Jump** under "Room spawn" in
  "Reaching this exit", save, and press **Check dungeon**: "No problems found" while the pickup lies in
  a room the player can reach before it. Delete the pickup and press it again: it reports "... needs
  Double Jump, which cannot be picked up before it". Undo (Ctrl+Z) brings the pickup back and the
  check passes again. Tick **Needs Dash** too: the report names only the ability that no pickup
  before the gate provides.
- Select an arrival marker: the outlines move to it. Turn Reach off: they vanish.
- Open **F4**, find **Double jump speed** (Jump section), raise it and Save: under "Saved to" a line
  such as "jump 389 px, dash jump 509 px, double jump 266 px high / 512 px far; run Check dungeon"
  appears, with a larger double jump height than before. Save again unchanged: no line. The Player
  panel does the same. With Reach on, a changed field redraws the yellow-green outline (slowly in a
  debug build, about 0.2 s).
- In the Charms panel make two charms with the effect "Move speed" at 200, Reach on: the panel lists
  in red each ability that nothing needs, naming both numbers (with the shipped numbers, "no gap can
  need Dash ... 1490 px ... 509 px" and the same for Double Jump), and the outlines go. Delete the
  charms and they come back.

## Manual check: hitbox and visual size

- Open a room with an enemy, select it, open Entity properties. Tick Separate visual size:
  the picture starts at the hitbox size. Drag Visual width and height: the picture and its
  thin light outline resize live while the colored hitbox outline stays put, and a taller
  picture grows upward from the same floor line.
- Change Hitbox width and height: the hitbox outline resizes and the picture does not (while
  the visual size is ticked). Untick it: the picture goes back to the hitbox size.
- Zoom and pan the canvas: the picture scales with the hitbox. Save, press F2, play: the
  enemy is drawn at the saved visual size, its health bar is over the picture, and it hurts
  you only inside the hitbox. Press F3 in play: body points sit in the picture and the
  hitbox rectangle is the hitbox.
- Select the boss, set a bigger Visual size: the purple box grows upward beyond the hitbox
  outline; in play the boss box is drawn at that size and slams still land on the hitbox.
