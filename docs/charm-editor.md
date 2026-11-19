## Charm editor

A **Charms** button next to **Player** in the room tools opens a panel that edits `charms.ron`
(saved to `assets/charms.ron`, which replaces the shipped `data/charms.ron` whole) and decides
which charms lie in which rooms. Opening it closes the Player panel and the other way round.

### The Charms panel

- One collapsing section per charm with every field of the file: `id` (read-only), `name`,
  `description` (the only text boxes), `mana_cost` and the effect amount (drag boxes with their
  units), the effect kind (dropdown), the swatch `color` (picker) and `owned_at_start`.
- Each field has a range (`charm_editor::FIELDS`, `MANA_COST`, `AMOUNTS`). A typed number outside
  it is refused with the reason under the field and the old value stays; a drag stops at the
  ends. A change of effect kind keeps the amount if it is in the new kind's range, else clamps it.
- Problems that involve more than one field are listed next to the field they blame and block
  Save: a duplicate id or name (names compare ignoring case), a `mana_cost` above the base max
  mana of the *saved* `vitals.ron`, a fully transparent color.
- **New charm** adds `charm_N` called "Charm N" (the lowest free N), with no popup. **Delete
  charm** removes it, and also unequips it and takes it off the owned list. **Reset charm** puts
  one charm back as saved (a never-saved charm is removed); **Reset all** puts the list back.
- **Save charms** writes `assets/charms.ron` through the staged writer. An invalid list writes
  nothing. The running game follows the edited list while you edit (an equipped charm changes the
  player's stats at once; the player is not reset) and uses the saved list after Save, like F5.
  If the saved costs no longer fit the base max mana together, charms are taken off from the
  last equipped and the editor message names them.

### Editor integration

- A dirty charm list is "Charms" in the Save / Discard / Cancel dialog. Save writes it (before the
  room, so a room can name a charm that was just made; after the room when the list deletes a
  charm a saved room places). Discard drops the edits and puts the saved list back in play.
- **Test room (F6)** plays the edited, unsaved list and writes nothing. A list with a problem
  refuses the test. A test starts owning the `owned_at_start` charms, so a new charm can be
  tried; the run's owned and equipped charms are put back when it ends.
- Charm edits have their own undo history (one step per drag, per typing stretch or per button).
  **Ctrl+Z / Ctrl+Y** go to it only while the last mouse press was inside the Charms panel.

### Charms in rooms

- A room file may list `charms: [(charm: "ward", position: (330.0, 452.0))]` in its `layout`.
  The field is optional, so existing files load unchanged. Each charm must exist in `charms.ron`
  and lie inside the room above the floor and outside the solids (16 x 16, centered).
- **Place** lists "Charm: <name>" after the items. Place, move, delete and undo work as for
  items. A placed charm is drawn as the charm's color swatch.
- In play, a charm is picked up with **Up** under the same rule as an item (grounded, within
  reach). It becomes owned and leaves every room for the run. A charm already owned is not
  placed again. The pickup raises `GameEvent::ItemPickedUp`.
- Save keeps the files consistent. A room cannot be saved naming a charm that is not saved yet
  ("save the charms first"). A charm that a saved room places cannot be deleted ("take it out of
  the room and save the room before deleting the charm").

### Known limits

- One effect per charm; owned charms are not saved to disk (a new run starts with the
  `owned_at_start` charms); charms can be equipped anywhere (no special places).
- Deleting a placed charm *and* placing a new charm in the same Save is refused: save the room
  without the new placement, then the charms, then place it again.
- Saving `vitals.ron` is checked against the charm list, so a Save cannot lock the game out. The
  Player panel Save, the F4 Save and Save all refuse a max mana below the cost of any charm, naming
  the dearest one and the lowest allowed max mana, and write nothing (not `player.ron` either). The
  Player panel and F4 compare it with the charms in `charms.ron` as saved, so a charm deleted in
  the Charms panel still counts until that list is saved; Save all compares it with the list it is
  about to write, so the same deletion does not block it. A max mana equal to the highest cost is
  allowed, and so is any raise. The Charms panel still checks a cost against the max mana saved in
  `vitals.ron` (its message says so), not the unsaved one that equipping uses.
- A hand edit can still break loading (a cost above the max mana in either file). The load error
  and the F5 error then end with one hint line: lower the charm's `mana_cost` in
  `assets/charms.ron`, or raise `max_mana` in `assets/vitals.ron` (or delete that file).

### Manual check: charms

1. Run the game and open the editor (F2), then **Charms**. Edit the Heart Charm's amount and its
   cost; Save charms; close the editor, open the inventory and equip it. The health bar and mana
   bar follow the saved numbers.
2. Type a cost above the base max mana: the field refuses it and says why. Make two names alike:
   the message shows and Save charms writes nothing. In the Player panel set Max mana to 10 and
   Save: refused, naming the Sharp Charm and 20, and no file changes; set 20: it saves.
3. Edit a charm, then press F2: the dialog lists "Charms". Cancel keeps it; Discard puts the
   saved numbers back; Save writes them.
4. Edit an equipped charm's amount and press **Test room (F6)**: the test plays the new number
   and nothing is written. Leave the test; the edit is still unsaved.
5. Drag a charm's cost, then press Ctrl+Z with the pointer last pressed in the panel: the whole
   drag is undone in one step; Ctrl+Y redoes it. Press the canvas and Ctrl+Z undoes the room.
6. Choose "Charm: Ward Charm" in the Entity list, Place it in a room, drag it, Save. Run the
   game, walk to it (it shows its prompt in reach), press Up: it is owned (inventory) and gone.
   Leave the room and come back: it stays gone.
