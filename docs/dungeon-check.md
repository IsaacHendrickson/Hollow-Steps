# Check dungeon

`src/dungeon_check.rs` reads every room's layout and the run rules (`run.ron`) and lists what would
stop a player, or what makes the marks that say so (`docs/gates.md`) wrong. It is pure: the editor's
**Check dungeon** button (Room tools, "Dungeon") and `tests/shipped_content.rs` both call
`dungeon_check::check`. The editor checks every room as the session has it, so the unsaved draft
of the room being edited and any parked drafts count (`EditorSession::check_dungeon`).

## The graph

Nodes are `(room, entrance)` pairs (`gates::entrances_of`: the room's spawn, and the arrival of every
exit, in any room, that leads there, an exit into its own room included). Taking an exit from an
entrance leads to the arrival node of the room it opens into. An edge is closed while its exit is
locked by a key the walk does not hold, or a gate on (entrance, exit) needs an ability it does not
hold. Taking an exit into the hub ends a run, so no walk goes on from the hub.

## Where a run starts

With a hub, through each exit of the hub that leads to `start_room` (the arrival node). If the hub has
none, that is reported and the check starts at `start_room`'s spawn so the rest still says
something. With no hub, at `start_room`'s spawn.

## The walk

Starts with no abilities and no keys. Entering a room picks up every item in it (no regions inside a
room), a held key opens every lock of its number, a gate opens when every ability it needs is
held; repeat until nothing changes. This is optimistic: it holds everything it ever found, so a room it
cannot reach is really unreachable.

## Problems, in the order reported

| Problem | Meaning |
| --- | --- |
| run.ron names a room that is not a room | nothing else is checked |
| The hub has no exit to the start room | the check starts at the spawn instead |
| Room can never be reached | not the hub, not the final boss room |
| The final boss room can never be reached | reported once, not also as unreachable |
| Locked exit, no key can be reached | in a reachable room; its key lies nowhere the walk gets to |
| Locked exit, the only key lies behind that same lock | told apart by walking again with that one lock open |
| A key is used up by the exit it opens, but more exits are locked by it than keys are placed | counted over every room |
| Exit needs an ability that cannot be picked up before it | the gate's entrance is reached, its ability never is, and no other entrance lets the exit be used |
| Trap | an entrance the player can reach where no exit can be taken (below) |
| A gate names an entrance the room does not have | `docs/gates.md`; changes no other result |

### Trap

A second search follows each way a run can go and keeps only what it picked up on that way (the walk
above holds everything it ever found). A state is a trap when the room has no exit that can be taken with
what is held. A dead end whose way back is open is not one, an exit into the hub counts as a way out, and
the final boss room is exempt (defeating its boss ends the run). Reported once per entrance, with the
fewest abilities that get stuck there ("arriving at the arrival from First (right edge exit at 100) with no
abilities, no exit can be taken").

## Known limits

- A key is looked at by its number, never counted along the way: how many locks and keys there are is
  its own rule. Which door spends a key is not searched, so a dungeon with enough keys can still be
  unwinnable if the player spends one on the wrong door.
- Everything in a room can be picked up once the room is entered; regions inside a room are not modelled.
- A gate is a claim the author makes. Nothing proves that a marked crossing needs Dash, or that an
  unmarked one does not (a bot that tries each crossing is a later task).
- A boss that keeps its room's exits locked until it falls is not modelled; only the final boss room is exempt
  from the trap check.
- The checker does not know a run can end by dying, so a room that is hard is still not a problem.
