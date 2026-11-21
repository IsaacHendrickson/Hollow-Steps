# Gates: marking what a crossing needs

A key blocks entry and the game enforces it (`docs/keys-and-locks.md`). An ability such as Dash is
different: **the game never checks it.** A room's geometry needs it (a gap too wide to jump, a
ledge too high) and the author *marks* the crossing so the tools know. The mark is a claim that
nothing enforces: play never reads it. Only the dungeon checker (`src/dungeon_check.rs`) and the
room editor use it. The reach envelope (`src/reach.rs`) is how the editor helps the author size
the gap so the claim is likely true; the proof, a bot that tries the crossing, is a known limit.

## The model

A **gate** says: in this room, going from entrance X to exit Y needs these abilities.

```ron
gates: [
    (from: Spawn, exit: Edge(edge: Right, offset: 120.0), needs: [Dash]),
    (from: Arrival(room: Custom(2), exit: Edge(edge: Left, offset: 40.0)),
     exit: Door(x: 600.0, y: 300.0, width: 48.0, height: 72.0), needs: [Dash]),
],
```

- **Directed.** Going from X to Y is its own fact. Y to X is free unless a gate says otherwise
  (falling into a pit needs nothing, climbing out may need Dash).
- **Absent means free.** A room with no gates behaves as before; `gates` is left out of the file
  when empty (`#[serde(default, skip_serializing_if = "Vec::is_empty")]`), so older room files load.
- **`needs` lists abilities and means all of them.** It is never empty (an empty list is the
  same as no gate) and holds each ability once. The ability names come from `Ability::ALL`, so
  a new ability appears in the editor by itself.
- **An entrance** is the room's `Spawn`, or an `Arrival` of another room's exit that leads here,
  named by *where it comes from*: that room and that exit. Never by where it lands.
- **An exit** is identified by `(edge, offset)`, the way `OpenedExits` does, never by its place in
  the list (the editor reorders and deletes exits). A door has no edge or offset, so its
  rectangle tells it apart: `Door(x, y, width, height)`.

## What the two open questions are settled as

**An arrival point moves.** Nothing changes in the gate. An entrance is named by its source, so
moving the point where the player lands (`move_inbound_arrival`, or dragging it on the canvas)
cannot detach a gate. If the arrival moves to a place that no longer needs Dash, or needs more,
the author changes the gate; the editor does not guess.

**A gate names no entrance.** This happens when the other room's exit was moved, re-linked
elsewhere or deleted. The gate is *kept in the file*, because the author's work is not ours to
throw away while a link is being redone. It is reported, not enforced:

- the Room tools panel lists it as a problem, with a Remove button;
- Check dungeon reports it, naming the room (it applies to no entrance, so it changes no result);
- Save is not blocked.

Moving an exit **in the room it belongs to** re-keys that room's gates on it (they follow the exit),
and deleting the exit deletes its gates. The same edit made to an exit in room B leaves room A's
gates that come from it orphaned (a known limit; Link... again and tick the boxes).

## Validation

A room with gates is valid when: every `needs` is non-empty with no repeated ability, every `exit`
names an exit the room has, and no two gates have the same `from` and `exit`. An unknown ability
is a parse error naming the room file. Whether an entrance exists is not validation (it depends
on other rooms); see above.

## Where it lives

- `src/gates.rs`: the types, the `RoomLayout` and `RoomDraft` methods, validation, re-keying.
- `src/gates/session.rs`: the entrances of the room being edited, orphans, setting a gate.
- `src/editor_panel/gate_panel.rs`: "Reaching this exit" and the orphan list.

## Known limits

- Nothing proves a marked crossing: no bot yet. Use Test room, with and without the ability.
- A gate covers a whole entrance to an exit; regions inside a room are not modelled (a pit is
  its own small room for now).
- Exits of another room that move or are deleted orphan the gates that come from them.
