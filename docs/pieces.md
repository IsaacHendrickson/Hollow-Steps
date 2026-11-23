# Dungeon pieces

A **piece** is a room that is marked as something a generator may build a dungeon from, with
**pickup slots**: spots where the generator may put a key or an ability. This slice is the data, the
editor and the checks. Nothing builds a dungeon from pieces yet and nothing in play reads the mark
or the slots, so a room that is a piece plays exactly as it did.

## What a piece is

Two optional fields of a room file (`RoomLayout::piece` and `slots`, `src/piece.rs`). Both are left out
of the file when empty, so every older room loads unchanged and a room that is not a piece
saves as it always did.

```ron
piece: Some((role: Middle, weight: 3)),
slots: [
    (position: (420.0, 440.0), kind: Key),
    (position: (500.0, 440.0), kind: Any),
],
```

- **Role.** `Start` (where a run begins), `Middle` (anywhere between), `Boss` (the last room).
- **Weight**, 1 to 10, default 1: how likely a generator is to pick this piece. Stored and
  validated here; nothing reads it yet.
- **Pickup slot.** A point (the center an item's center would have) and a kind: `Key`, `Ability` or
  `Any` (either). A room holds at most 8.

A mark and slots are checked with the room (`RoomLayout::check_piece`, run by every validation):
the weight is 1 to 10; at most 8 slots; every slot is a finite point inside the room, outside every
solid (the check an item gets); no two slots overlap (closer than an item's size on both axes).
Each failure is a named error that says which slot. A room that is not a piece keeps its slots and
they are checked the same.

The mirror (`docs/variety.md`) keeps the mark and flips the slots with the room.

## In the editor

Room tools has a **Dungeon piece** section.

- **This room is a piece** box, then **Role** and **Weight**. One undo step each (a dragged
  weight is one step).
- **Add pickup slot** puts an `Any` slot on the floor beside the spawn. Drag it like a hazard; the
  selected slot's panel has a **Kind** dropdown; Delete (or Delete selected) removes it. A slot is a
  dashed teal ring the size of an item, with K, A or ? in it.
- Unmarking a room keeps its slots; they do nothing until it is marked again.
- **Test room** plays the room without any slot: a slot is never a pickup.
- The unsaved-changes guard asks about a changed mark or slot like any other change.

## The pool rules

**Check dungeon** (the Room tools button, and `tests/shipped_content.rs`) ends with
`piece::pool_check(rooms, config)` (`src/piece/check.rs`). The pool is every room that carries a
mark; with none, nothing is reported and every other result stands. Otherwise (full text in
`docs/dungeon-check.md`):

1. At least one Start piece and one Boss piece.
2. Every piece has at least one exit.
3. No piece has a locked exit or a Key item: keys come from the generator through slots, so a
   key in a piece is a key the generator did not place. The problem says so and names the room.
4. A piece whose id is `start_room` or `final_boss_room` in `run.ron` has the matching role.
5. Every gate in a piece that names an arrival can be carried to another neighbour (below).
6. A piece with a gate that needs an ability has another piece with an Ability or Any slot, so
   the ability can be found somewhere.

A piece is walked like any room when the player can reach it. It is only never reported as "can never
be reached", because a pool piece belongs to the generator's pool, not the fixed dungeon.

## Marking a Dash crossing so it can travel

A gate (`docs/gates.md`) says that getting from one **entrance** to one **exit** needs an ability. An
entrance is the room's spawn, or the arrival of some other room's exit that leads here: *coming in from
room N through its exit E*. That names a neighbour, and a piece will have other neighbours once a
generator joins it elsewhere. A gate can still follow it if the piece has exactly one exit into room N:
that exit is the **port**, and a neighbour joined at the port is joined where the gate says.

To mark a Dash crossing in a piece:

1. Select the exit the crossing leads to. Under **Reaching this exit**, find the row of the
   entrance the player crosses from and tick **Needs Dash** (or **Needs Double Jump**).
2. A crossing from the room's own spawn is the piece's own: it always travels (nothing to do).
3. A crossing from an arrival needs the piece to have **exactly one exit into that room**. Link each
   port to its own room: two exits into the same room are two ports, and the gate cannot say which
   (rule 5 reports it, "has 2 exits into ..."); no exit into it at all means the gate names an
   arrival the piece cannot have.
4. Put an Ability or Any slot in some other piece (rule 6). A piece that needs a Dash is
   only fair when a Dash can lie somewhere else in the pool.

The rule checks that a gate *can* be carried. Carrying it to the neighbour a generator picks is the
generator's work, and a later task.

## A manual check

1. Mark two rooms as pieces, one Start and one Boss, and add two slots to one of them (a Key and
   an Ability slot).
2. Save, then press F5 (reload): the marks, roles, weights and slots are the same.
3. Check dungeon: the pool is a Start and a Boss piece with no problem about roles. Mark a third
   room Middle and place a Key item in it: Check dungeon reports it, names the room and says keys come
   from slots.
4. Select a piece, delete its exit (or use a room with none): Check dungeon reports the piece
   with no exit.
5. Unmark one room: its slots are still there (select one) and no longer count in the check.
6. Press Test room on a room with slots: no slot shows as a pickup.

## Known limits

- Slots hold keys and abilities only. No coins and no charms.
- A piece cannot hold a lock (rule 3): the generator places keys and decides what they open.
- One-way links are not pieces' business yet: a piece's exits are counted as plain exits.
- The pool rules only say a pool is fit to be built from; they do not prove a dungeon can be assembled
  from it, or that it can be won. `generate` (`docs/dungeon-gen.md`) builds from the pool by weight,
  places items in slots and carries gates, and says in a `GenError` when it cannot. Nothing in the
  game calls it yet.
- A piece in a fixed dungeon is checked like any room, and the pool rules apply to it as well.
