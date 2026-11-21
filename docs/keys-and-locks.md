# Abilities, keys and locked exits

Two kinds of progress live only for the length of a run and are lost when it ends:

- **Abilities** (`src/ability.rs`): what the player can do. Today only `Ability::Dash`.
- **Keys** (`Item::Key(1..=8)`): items that open exits locked by the same number.

## Two ways to gate a room

- **Keys are hard gates.** The game enforces them: a locked exit refuses the player until the run
  holds the key.
- **Abilities are not checked at doors.** The room's geometry is the gate (a gap only a dash
  crosses). Nothing in a room file says "needs Dash". Annotating such gates is a later task.

## Abilities

`AbilitySet` (`has`, `grant`, `clear`) is held by the `Player`; the dash reads
`abilities.has(Ability::Dash)`, not the item list. `Item::grants()` says which ability a pickup
gives (`Dash` gives `Ability::Dash`; the shard and keys give none). `Player::collect_item` pushes
the item as before and also grants. `reset_for_run` clears the abilities with the collected
items, so a new run starts without the Dash and finds it again where it lies.

## Keys and locks

- `Item::Key(n)`: `n` from 1 to 8 (`exit_lock::check_key_id`), name "Key n", RON `Key(3)`. In
  `Item::ALL`, so the editor's Place list has "Item: Key 1" to "Item: Key 8". A room that holds a
  key outside 1 to 8 fails its checks.
- `Exit::locked_by: Option<u8>` (`Exit` stays `Copy`). `ExitPlacement::locked_by` is its room-file
  form: written as `locked_by:3` after `spawn` (edge exits and doors alike), left out when `None`,
  so every older room file loads unlocked. `check_lock` validates 1 to 8; a new exit starts
  unlocked.
- The boss lock (`Room::exits_locked`) is separate and wins: while the boss lives, no exit or door
  can be touched, so no key is looked at or spent.

### In play (`src/game/world_locks.rs`)

`GameState::pass_exit` runs inside `use_exit`, which edge exits and doors both call, before the
player is moved:

1. Unlocked: through.
2. Locked and the player holds that key: one key is taken from the items (`Player::take_item`),
   the live exit's lock is removed for the rest of the run, the player goes through. One key,
   one door: a second door with the same number stays locked.
3. Locked without the key: nothing happens and the notice "Locked: needs Key 3" shows for
   `NOTICE_SECONDS` (2 s; `src/notice.rs`, drawn by `src/hud_notice.rs` with `camera::anchored`).
   The notice does not run down while the editor is open.

### Opened doors survive an editor Save

`OpenedExits` (`src/exit_lock.rs`) remembers each opened exit by room id, edge, offset (and the
door's rectangle) and the key, never by list index, like `TakenItems`. After the editor's Save
rebuilds a room from its file, `restore_opened_exits` opens the matching exits again; an entry
whose exit was moved, deleted or given another key is forgotten. Opening a door in play never
changes a room file.

A **test room** is a sandbox: its exits are inert, keys found there are not kept, and nothing is
remembered. A **run reset** (`reset_run_state`) rebuilds the rooms from their files and forgets
`OpenedExits`: every door is locked again and every key lies in its room again.

## Editor

- Select an exit (edge exit or door): **Locked by** dropdown, None or Key 1 to Key 8
  (`src/editor_panel/lock_panel.rs`). A choice is one undo step and marks the room unsaved;
  choosing what is already set changes nothing; a refused value shows the message.
  `RoomDraft::set_exit_lock` does the edit.
- The canvas writes "Key 3" next to the band of a locked exit (in the middle of a door) and the
  number inside a key's marker (`src/editor_ui/lock_labels.rs`, `labels` is a pure function over
  the markers; none while one exit's arrival is shown).
- Where the text goes is one rule for the game and the editor: `exit_lock::label_rect`. In the
  game, `src/scene_locks.rs` outlines a locked band and writes "Key 3".
- A link or **Set exit target** changes `target_room` and `spawn` in place, so the exit keeps its
  lock (two-way links keep both exits' locks).

## Tests

- `src/ability.rs`, `src/player/ability_tests.rs`, `src/game/gate_tests.rs`: abilities.
- `src/exit_lock/tests.rs`: key ids, names, labels, `OpenedExits`, room-file form, validation.
- `src/exit_lock/editing_tests.rs`: set and clear in the draft, undo, links keep the lock, Place list.
- `src/game/world_locks/tests.rs`: blocking, consuming, staying open, doors, boss lock, Save,
  test room, run reset.
- `src/editor_panel/lock_panel/tests.rs`, `src/editor_ui/lock_label_tests.rs`: the dropdown and the labels.

## Manual check

Place Key 1 in a room and lock an exit with Key 1. Walk to the door without the key: the notice
shows. Pick the key up and walk in: the door opens and the key is gone. Come back: still open.
F2, Save: still open. Die and start a new run: locked again and the key is back in its room.
The Dash still works after picking it up.

## Known limits

- More abilities (wall jump, double jump) are not added; the Dash is the only one.
- A gate that needs an ability is not annotated or checked (next task).
- Keys are used up; there are no kept keys and no key list in the inventory.
- The "Up: enter" prompt still shows over a locked door.
- A test room's exits are inert, so locks cannot be tried there.
- The notice time is a constant, not data.
