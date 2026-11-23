# Dungeon generator

`generate` (`src/dungeon_gen.rs` and `src/dungeon_gen/`) builds a dungeon from a pool of pieces
(`docs/pieces.md`) and a seed, and proves it with the dungeon checker (`docs/dungeon-check.md`).
It is pure: no files, no window, no clock. The game calls it when a run starts (see "A generated
run" below), and `generate` itself does not look at the `enabled` switch of the `generator`
section in `variety.ron` (`docs/variety.md`): the game's own decision does (`decide_generation`).

```rust
generate(pieces, hub, config, rules, seed) -> Result<DungeonPlan, GenError>
```

- `pieces`: every room that has a piece mark, as `(RoomId, RoomLayout)`, in any order.
- `hub`: the hub room, `(config.hub_room, layout)`, with an exit to `config.start_room`.
- `rules`: the `generator` section (`GeneratorRules`): `rooms`, `spine`, `keys` (each `(min: .., max: ..)`),
  `mirror_percent`, `attempts`. Every number is validated first and a bad one is refused naming the
  field (`generator.rooms.min must be ...`).
- `DungeonPlan`: `seed`, `rooms` (`PlannedRoom { instance, piece, mirrored, layout }`, the start room
  first, then the spine up to the boss room, then the side branches), the rewired `hub` layout and
  `attempts` (the attempt that worked, from 1). `plan.catalog()` is every room as the checker takes
  it, the hub first.

The same inputs always give the same plan, whatever order `pieces` comes in: the pool is sorted by
room id first, and every random choice comes from `stream(seed, "dungeon-gen", key)` with the
attempt number mixed into the key, so attempt 2 is a fresh draw (`docs/variety.md`, streams).

## How an attempt goes

1. Each piece is mirrored (`mirror_layout`) with `mirror_percent`, drawn per piece.
2. A tree grows: a **spine** from a Start piece to a Boss piece (length within `spine`), the links on
   it that get a lock, rooms that make room for each key and ability, then side branches of one
   to three rooms until the count is within `rooms`. Pieces are picked by weight; a piece is used at
   most once.
3. The keys and abilities that are needed are matched to slots (a bipartite matching, so one slot
   never serves two items).
4. The rooms are built (rules below) and `dungeon_check::check(&plan.catalog(), config)` judges the
   result. Empty means done; anything else starts the next attempt, up to `attempts`.

## The rules a plan follows

1. **Ids.** The Start piece becomes `config.start_room`, the Boss piece `config.final_boss_room`, every
   other piece keeps its id. A piece is used at most once.
2. **Mirroring.** Each piece is mirrored with `mirror_percent` (0 never, 100 always); a mirrored piece
   is `mirror_layout` of the authored room.
3. **Ports.** An exit is a port. Two ports pair when their edges are opposite (Left with Right, Top
   with Bottom) or both are doors. Joined exits are two-way: each has `target_room` set to the other
   room, `spawn = arrival_next_to(...)` of the exit it meets, and `arrival_error` of that spawn is
   `None`.
4. **Unpaired exits** are removed, and so are the gates that name them (as the exit or as the exit
   they arrive from).
5. **Arrival gates** are carried to the new neighbour at the same port (a gate that names an arrival
   keeps its exit and needs, and now arrives from the neighbour's exit that leads here).
6. **The hub's** exits to the start room keep their target and get the start room's spawn.
7. **The plan is a tree.** The spine is within `spine`, the room count within `rooms`.
8. **Locks.** `k` links of the spine, with `k` within `keys`, are locked on the exit that leads away
   from the start, numbered 1 to `k` in spine order.
9. **Keys.** Key `i` lies in a Key or Any slot of a room the player reaches without crossing lock
   `i` or any later one.
10. **Abilities.** Each ability a gate needs is placed once, in an Ability or Any slot reachable before
    the first gate that needs it. An ability item a piece already holds counts as placed.
11. **Slots.** A filled slot becomes an item at the slot's position; an empty slot is dropped. A
    planned room carries no piece mark, no slots and no map cell.

A plan is valid when the checker finds nothing. `tests/support/plan_rules.rs` checks rules 1 to
11 on every plan from the plan and the pool alone, in `tests/dungeon_generator.rs`.

## How to read a `GenError`

The first group is refused before any attempt, because no seed could build from the pool or rules.

| Text | Meaning and what to change |
| --- | --- |
| `the generator rules are not valid: ...` | A number in `generator` is out of range; the text names the field. |
| `... is in the pool but has no piece mark` | A room without a mark was passed in `pieces`. |
| `... is in the pool twice` | Two rooms with one id. |
| `... is a piece and also the hub` | Unmark the hub or pass another room as the hub. |
| `the pool has no Start piece` / `no Boss piece` | Mark a room with that role. |
| `the pool is not fit to build from: ...` | `piece::pool_check` found problems (`docs/pieces.md`); each is quoted. |
| `run.ron's hub_room is ..., but the hub given is ...` | `config.hub_room` and the hub passed must be the same room. |
| `the hub ... has no exit to the start room ...` | Give the hub an exit whose target is `start_room`. |
| `rooms.min is N but the pool can fill at most M rooms` | Fewer pieces than `rooms.min` (or `spine.min`): lower it or add Middle pieces. A Start and a Boss piece count once each. |
| `keys.min is 2 but only 1 Key slot exists` | Fewer Key and Any slots than `keys.min` (Boss pieces' slots do not count). Add slots or lower `keys.min`. |
| `keys.min is N but a spine of at most M rooms has only ... links to lock` | Raise `spine.max` or lower `keys.min`. |
| `no two pieces in the pool have exits that can be paired` | No Left/Right, Top/Bottom or door/door pair exists among any two pieces, even mirrored. |

The last one comes after the attempts:

`no valid dungeon in N attempts; the last attempt stopped at: ...` quotes what stopped the **last**
attempt, up to six lines and then "and N more". Either the generator stopped by itself or the
checker's `Problem::text` follows:

- `no piece can follow X on the spine`: every unused piece that fits has no exit that pairs with
  X's free ones (or would leave the spine unable to reach the boss).
- `the boss piece X has no exit that pairs with a free exit of Y`: the end of the spine could not
  meet the Boss piece.
- `no room with a slot for Key N can be joined before it`: the lock was drawn but no piece with a
  Key or Any slot can be joined in the part of the dungeon that is open before it. Add more
  dead ends with Key slots next to common ports, or lower `keys`.
- `no room with a slot for Dash can be joined before it` (and the same for any ability): as above for
  an ability a gate needs.
- `no room is left to hold X`: the `rooms.max` count is used up.
- `only N rooms could be joined, and rooms.min is M`: the side branches ran out of pieces that fit.
- `no slot is left for X`: the slots were all taken.
- Anything else is the dungeon checker's text, which means the plan was built but a rule of
  `docs/dungeon-check.md` failed: read it there. This should not happen; it is a bug if it does.

More attempts help when failures are about luck (a spine that dead-ends); they do not help when the
pool cannot satisfy the rules at all.

## A generated run

`start_run` (`src/game/run_link.rs`, with `src/game/generate_link.rs`) decides whether the run
plays a generated dungeon. The decision is `variety::decide_generation(choice, rules, pool_has_ends)`,
a pure function: F4's **Generate next run** choice (Auto, Never, Always) and `generator.enabled`.

| choice | result |
|---|---|
| Never | the fixed dungeon, whatever the file says |
| Always | generate; if the pool cannot build, the run does not start (below) |
| Auto | generate when `generator.enabled` is on **and** the pool has a Start and a Boss piece; else the fixed dungeon |

**The plan** is made from the room files, never from the live rooms: `world_load::generated::read_sources`
reads every room file (`mirror::room_ids`), keeps the layouts that carry a piece mark, and takes the
hub's layout from its file (falling back to the live hub when there is none). It then calls
`generate(pieces, hub, run.ron, rules, run seed)`. The editor's unsaved drafts are not read: only
Save changes what a run is built from.

**The world** (`world_load::load_world` with `Some(plan)`, built by `world_load/generated.rs`) is the
rewired hub plus the plan's rooms and **nothing else**: every other authored room is absent for the
run. Each planned room is built from a `RoomDraft` made from the plan's layout and the piece's own
enemy and boss placements, read from the piece's file and mirrored with `mirror_document` when the
plan mirrored the piece. It is built with `RoomDraft::build` and `place_boss`, the same calls
`load_extra_rooms` uses, so enemies, bosses, items, charms, puzzles and gates behave exactly as in an
authored room. The `min_tier`, `group` and `chance` filters and the difficulty layer apply as to any
run, and `validate_destinations`, `validate_room_charms` and the terrain look check run on the
generated world. An error names the piece file it came from.

- **Start.** The hub's exit is rewired to the start room (the Start piece, which takes the id
  `start_room` from `run.ron`); `use_exit` takes the arrival from the rebuilt hub's exit, as in a
  flipped run, so the player lands at the start spawn.
- **Clearing.** The Boss piece takes the id `final_boss_room`, so its boss falling clears the run
  as it always did. Every other piece keeps its own id.
- **Kept for the run.** `RunState` holds the plan until the next reset. F5 rebuilds the same rooms
  from it (layouts and ids identical). The hub phase and the result screen after Continue use the
  authored world; the result screen itself still shows the generated one.
- **Never flipped.** A generated run is not a flipped run and mirrors no room by seed: the plan has
  already chosen each piece's side (`mirror_percent`).
- **Resets.** Taken items, opened locks and puzzle state are cleared by the run reset like any
  run's; the same seed builds the same dungeon, whole again.
- **Failure.** When the generator gives no plan (or a file cannot be read) the run does not start:
  the message is `Could not start the run: <GenError>`, the player stays in the hub in the authored
  world, and **no seed is used up**. This only happens with Always (Auto with no Start or no Boss
  piece plays the fixed dungeon).
- **The editor.** While a run is generated (and on its result screen), F2 and Open room refuse
  (`docs/variety.md`, "The editor during a generated run"). Test room is unaffected.

The preview in the editor (`docs/variety.md`, "Preview generated dungeon") shows what a seed would
build, from the pieces as the editor holds them.

## Manual playtest

1. Make pieces (Start, Middle, Boss; a few with Key and Ability slots), set `generator.enabled: true`
   in `assets/variety.ron` (restart the game) and Save. F4 Generate next run stays on Auto.
2. Walk out of the hub: the run starts. Open the map (M) and note the rooms you can see.
3. Die and start again: a different route (different seed, shown on the result screen and in the
   pause menu as `Seed <hex>`).
4. F4, Run (debug), Seed: tick **Fixed seed** and type the seed from a result screen: the same route
   again. Without the box ticked the next run is different again.
5. Editor, Room tools, Dungeon piece, **Preview generated dungeon**: type the same seed: the same
   map as the run, with the same keys and abilities listed. **Roll next** steps to another seed.
6. During the run press F2: refused, saying to abandon the run or set Generate next run to Never.
7. Abandon from the pause menu, Continue, press F2: the editor opens.
8. Press F5 in the middle of a run: the dungeon stays the same.
9. F4, Generate next run: **Never**. Start a run: the old fixed dungeon is back.
10. Generate next run: **Always** with the pieces' Boss deleted (or the pool removed): the run does
    not start, the message says why, and the next seed is unchanged.

## Known limits

- The dungeon is a tree: no loops, no shortcuts back.
- No one-way drops (`docs/gates.md`): every link is two-way.
- Locks are only on the spine, never on a side branch.
- A piece cannot carry a lock or a key (pieces with authored locks and keys are refused by
  `pool_check`): the generator decides what opens what.
- Unused ports become walls: an exit with no neighbour is removed, and so is any gate that names it.
- Physical gates are proved only by the checker's rules (`docs/dungeon-check.md`); the bot that
  searches for inputs comes later, so a gate that is possible on paper may still be hard in play.
- No variants (a piece swapped for another that has the same ports); that is the next prompt.
- A generated run plays no authored room except the hub: the shipped Second room, first room and boss
  room are not in it, and a game with no pieces plays as it always did.
- The plan is read from the saved files, so an unsaved piece is in the preview but not in a run
  until it is saved.
- Failed attempts are common when keys are asked for: the lock and key are placed after the
  spine is drawn, so many attempts stop on `no room with a slot for Key N`. Attempts are cheap: on the
  test pool (2 Start, 2 Boss, 15 Middle pieces) seeds 0 to 1999 needed 2.5 attempts on average and 20
  at worst, against the shipped default of 200.
