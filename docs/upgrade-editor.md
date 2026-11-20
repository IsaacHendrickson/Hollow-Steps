## Upgrade editor

An **Upgrades** button next to **Charms** in the room tools opens a panel that edits
`upgrades.ron` (saved to `assets/upgrades.ron`, which replaces the shipped `data/upgrades.ron`
whole). Opening it closes the Player and Charms panels, and the other way round. The rules of the
shop and the profile are in `docs/run-and-profile.md`.

### The Upgrades panel

- One collapsing section per upgrade with every field of the file: `id` (read-only), `name` and
  `description` (the only text boxes), the effect (a kind dropdown and one amount drag box with its
  unit, **per level**) and `costs` (one price drag box per level, "Level 1" and so on, in coins).
- Each number has a range (`upgrade_editor::amount_spec`, `COSTS`): max health 1 to 1000, move
  speed 1 to 200 percent, sword damage 1 to 1000, a price 1 to 1,000,000 coins. A typed number
  outside it is refused with the reason under the field and the old value stays; a drag stops at
  the ends. A change of effect kind keeps the amount if it is in the new kind's range, else
  clamps it.
- **Add level** adds a last level priced like the one before it (up to 10); **Remove level** takes
  the last one off (down to 1). The number of prices is the highest level.
- Problems are written next to the field they blame and block Save: a name of 0 or more than 32
  characters, a description that is not one line of at most 200, a duplicate id or name (names
  compare ignoring case), no levels or more than 10, a price of 0 or above 1,000,000.
- **New upgrade** adds `upgrade_N` called "Upgrade N" (the lowest free N) with one level, no popup.
  **Delete upgrade** removes it; its tooltip says players who bought it keep the levels but get no
  effect. **Reset upgrade** puts one upgrade back as saved (a never-saved upgrade is removed);
  **Reset all** puts the list back.
- A line under the heading shows what every level of every upgrade adds together: "All upgrades at
  maximum: +30 max health, +15% move speed, +9 sword damage" (`upgrades::max_totals`).
- **Save upgrades** writes `assets/upgrades.ron` through the staged writer. An invalid list writes
  nothing. The running game follows the edited list while you edit and uses the saved list after
  Save, without resetting the player (`GameState::sync_upgrades_from_editor`, `save_upgrades`).

### Levels already bought

The levels players bought live in `profile.ron`, by upgrade id, and the editor never touches that
file. Deleting an upgrade keeps its levels; they have no effect until an upgrade with that id is
listed again. A level above an upgrade's new highest level counts as the highest. Changing an
upgrade's amount or prices changes what bought levels give at once; nothing is refunded.

### Editor integration

- A dirty list is "Upgrades" in the Save / Discard / Cancel dialog, after Player and Charms. Save
  checks the list before the first write and saves it last (nothing refers to it); Discard drops the
  edits and puts the saved list back in play.
- **Test room (F6)** plays the edited, unsaved list and writes nothing (not the profile either). A
  list with a problem refuses the test ("the upgrades cannot be used"). The edits are still
  unsaved afterwards.
- Upgrade edits have their own undo history (one step per drag, per typing stretch or per button;
  `src/snapshot_history.rs`). **Ctrl+Z / Ctrl+Y** go to it only while the last mouse press was
  inside the Upgrades panel.

### Known limits

- No new effect kinds (max health, move speed and sword damage only), no editing of the shop's
  position (it is `shop` in `run.ron`), and one effect amount per level (the amount does not change
  from level to level).
- Upgrades only count in a game with a hub (`run.ron` naming a `hub_room`); in the shipped game
  without one the panel still edits and saves the list, and no upgrade is in effect.
- A hand edit that breaks the rules (for example a price above 1,000,000) fails loading with the
  error naming the upgrade; fix it in `assets/upgrades.ron`.

### Manual check: upgrades

1. Run the game with a hub, press F2, open **Upgrades**. Change Vitality's amount and its first
   price, **Save upgrades**, close the editor, walk to the shop in the hub and buy Vitality: it
   costs the new price and gives the new amount.
2. Type a price of 0 or 2000000: the field refuses it and says why. Make two names alike: the
   message shows and **Save upgrades** writes nothing.
3. Change an upgrade, then press F2: the dialog lists "Upgrades". Cancel keeps it; Discard puts the
   saved numbers back; Save writes them.
4. Change an owned upgrade's amount and press **Test room (F6)**: the test plays the new number and
   nothing is written. Leave the test; the edit is still unsaved.
5. Drag a price, then press Ctrl+Z with the pointer last pressed in the panel: the whole drag is
   undone in one step; Ctrl+Y redoes it. Press the canvas and Ctrl+Z undoes the room.
6. Delete an upgrade you own and Save: the stats lose its effect, the shop no longer lists it, and
   `profile.ron` still has its level.
