# Door exits

A door is an exit that stands anywhere in a room instead of on one of its four edges. The
player uses it by pressing Up (the pickup key) in front of it. Edge exits are unchanged: they
still trigger by walking into them.

## Rules in play

One function decides both entering a door and the "Up: enter" prompt over it, so the two
cannot disagree: `GameState::door_in_reach` (`src/game/world_doors.rs`), built on
`Room::door_at` (`src/room.rs`).

- The player stands on the ground, the body reaches the door's rectangle, and the player's
  center is within the door's width (edges count).
- The room's exits are unlocked (a boss room's doors stay shut until the boss falls) and it is
  not a test room (its exits would lead into saved rooms).
- The Up press goes to an item or charm in reach first; Up together with the attack key is an
  upward strike, never a door.
- Walking into a door does nothing; `Room::exit_at` ignores doors.

## Where the player arrives

An arrival in front of a door is the door's bottom edge, centered (the nearest free spot to
either side if a solid is in the way). Unlike an edge exit, a door may contain the arrival
point: a door needs Up, so arriving in one is not an immediate return trip. Arriving inside an
edge exit is still refused.

## Room file

A door has a `door` rectangle `(x, y, width, height)` in room coordinates (like `terrain`) and
no `edge`, `offset` or `length`; an edge exit is written exactly as before, so no existing file
changes. `ExitPlacement` is (de)serialized by hand in `src/door_exit.rs` for this: RON would
otherwise write `Some(...)` around an optional field and need dummy edge values for doors.
A door must be at least 16 wide and high, inside the room above the floor, and not inside a
solid (standing on top of one is fine). A door does not cut an opening in the room's wall.

## Editor

- **Add door** (uses the "Exit target" list, like Add exit) and **Add door and link...** (opens
  the room chooser, Cancel removes the door) put a door at the room's spawn point.
- A door is drawn orange on the canvas; drag it like any exit (it moves by its center).
- Selecting a door shows **Door width** and **Door height** (a drag is one undo step; the
  bottom edge and center line stay put).
- Linking works as for edge exits: connect an exit of B (a door or an edge exit) and the
  arrival is placed in front of it.

Known limits: **New exit here** in the link flow still places an edge exit (add a door to B
first, then link to it); moving or deleting a linked door later does not update its partner.

## Manual check

1. F2, **Add door and link...**, choose Second, select one of its exits, Connect, Save.
2. In the first room walk onto the door: nothing happens, an "Up: enter" prompt appears.
3. Press Up: you arrive in Second in front of the exit you chose. Jump while standing at the
   door: no prompt in the air. Stand just beside the door: no prompt.
4. Select the door in the editor, change width and height, drag it onto a platform, Undo.
5. Start a boss fight in a room with a door: the door is red and Up does nothing until the
   boss falls. In a test room (F6) doors do nothing.
