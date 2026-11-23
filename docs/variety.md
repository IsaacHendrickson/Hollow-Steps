# Variety: the run seed and the dungeon flip

Every run has a **seed**, one `u64` that makes a run's random choices repeatable. The seed is
the first thing that makes one run differ from the next without any file being edited. Optional
enemy placements are the first thing that uses it.

## The seed

- **Where it comes from.** `SeedSource` (`src/seed.rs`) hands out one seed per run: it walks a
  SplitMix64 sequence from a start value. `main.rs` reads the system clock **once**, at
  start-up, and gives the game a source started from it (`GameState::use_seed_source`); nothing
  else reads a clock. A game that is never given a source starts from a fixed constant, so no
  test is random.
- **When a run takes it.** `start_run` takes the next seed and the run keeps it for its life,
  through the result screen. `GameState::run_seed()` is `Some` during a run and on the result
  screen, and `None` in the hub, in a game with no hub and in a **test room** (a sandbox takes no
  seed and uses none up). `last_run_seed()` is the seed of the last run started, even after it is
  over (F4's label uses it). A run that cannot start (a room file is broken) takes no seed.
- **Fixing it.** `set_fixed_seed(Some(seed))`: every run then starts with exactly that seed, and
  the source does not move, so turning it off goes on with the sequence it was in. A run already
  going keeps the seed it started with.
- **Not saved.** The seed is not in `profile.ron`: it describes one run, and a fresh launch
  starts from the clock again.

## The streams

One seed feeds many uses, and no use may disturb another. `stream(seed, label, key)` returns a
`particles::Rng` (SplitMix64) for one use:

- `label` says what the stream is for (`"enemies"`), `key` says which one (a room and a group).
- The label is hashed with FNV-1a, then the seed, the hash and the key are mixed through
  SplitMix steps. The same three values always give the same stream; a different seed, label
  or key gives a different one, and for one seed and label two keys never start in the same state.
- The first numbers of three streams are pinned in `src/seed/tests.rs`: if the mixing changes,
  the test fails loudly, because every run's enemies would change with it.

## A seed as text

`format_seed` writes 16 lowercase hex digits with leading zeros (`00000000000000ab`).
`parse_seed` reads 1 to 16 hex digits in either case with an optional `0x`; an empty text, a
bare `0x`, a character that is not a hex digit (a space too) and more than 16 digits are each
refused with a message that says why.

## Showing the seed

- **Result screen.** The last line is `Seed: <16 hex digits>` (`run::seed_line`), under the tier line
  after a clear, in a dimmer colour. It stays on the screen as long as the result does. Type the
  digits into F4 to play the same run again.
- **F4, Run (debug), Seed row.** A **Fixed seed** checkbox, a **text box** and a read-only label.
  The text box takes the seed as hex (1 to 16 digits, either case, an optional `0x`, spaces
  around it ignored). AGENTS.md says text boxes are for names and numbers get drag boxes; a seed is
  an identifier, not a tuned number (it is copied from the result screen and has no range to
  drag over), so a text box is right here.
  - Ticking the box fixes the seed to the text. A text that is not a seed is refused with the
    reason shown under the section's buttons, and the box stays unticked.
  - While the box is ticked, typing changes the fixed seed at once; a text that is not a seed
    keeps the old seed and shows why.
  - Unticking goes back to the source (the clock at launch). The text box is not read then.
  - The label reads `Seed of this run: ...` during a run and on its result screen, `Seed of the last
    run: ...` in the hub afterwards, and says no run has started before the first one.
  - The row does not touch the profile. With no hub in `run.ron` the whole section only says there
    are no runs, so there is no row.

## Optional enemy placements

An enemy placement in a room file can say how sure it is to be there. Two optional fields:

```ron
(definition: "chase", patrol_left: 100.0, patrol_right: 200.0, group: Some(1)),
(definition: "chase", patrol_left: 460.0, patrol_right: 540.0, chance: 50),
```

- `group: Some(1)` to `Some(9)`: the placement is one of the **alternatives** of that group in its
  room. A room can have up to nine groups; groups of different numbers do not touch each other.
- `chance: 0` to `100` (percent, 100 when the file says nothing): how likely the enemy is to be in
  the room.
- Both are left out of the file when they are the default, so an older room file loads and is saved
  back unchanged. A group outside 1 to 9 or a chance over 100 is refused when the file loads, and
  so is a **grouped placement whose chance is not 100** (a group keeps exactly one member whatever
  the dice say, so a chance next to a group would mean nothing). The error names the placement
  (`enemy 2 ("chase")`) and the file. A boss placement takes neither field.

## The rule

It is applied once, where the game builds the rooms **of a run** (`game::tier_link::Layer`), and
nowhere else:

1. Every placement of the room is numbered by what it is (`variety::mark_identities`): its
   definition's name, its patrol and how many identical placements come before it (so two twins
   still differ, and nothing else about the place in the list matters). It is made before the tier
   leaves anything out, so the tier does not move the picks.
2. The **min tier** is applied first: a placement below the run's tier is out before any draw.
3. From each group, exactly **one** of the members that are left stays: the first number of
   `stream(seed, "enemies", key)`, taken modulo the number of members, with the key from the room
   and the group. A group with one member left keeps it, so a tier that leaves out two of three
   never leaves the group empty.
4. Every ungrouped enemy with a chance under 100 rolls: the first number of the stream whose key
   comes from the room and the enemy's number, modulo 100. It stays when the roll is under its
   chance. Chance 100 never rolls and always stays, chance 0 never stays.

The same seed always gives the same enemies (the draws read nothing else), a different seed gives
other ones, and the room's other enemies, its boss (never touched; its exits lock as before) and
its order are as before. The pinned results of five seeds are in `src/variety/tests.rs`, checked
against a separate implementation.

## Where it does not apply

Outside a run **nothing is left out**:

- the hub, and a game with no hub (a layer with no seed);
- a **test room** (a sandbox takes no seed), even inside a run, and the run's own rooms are back
  when it ends;
- `load_rooms`, `load_extra_rooms` and the editor, so the editor shows every placement and
  **Save writes every placement** to the file, whichever seed the run has;
- a room file read for any other reason.

In the middle of a run, **F5** and an editor **Save** rebuild the world with the run's own seed, so
the same enemies come back. After a file edit, an ungrouped enemy that did not change keeps its
roll even when other placements are added or moved (its draw is keyed by its own number); a group
picks among its members, so its pick can change when its members do.

## In the editor

Select an enemy: under **Min tier** in the Placement section are two rows.

- **Group**: a **None** checkbox and a drag box from 1 to 9. Unticking None puts the enemy in group
  1; the drag box can only be used while the enemy is in a group.
- **Chance %**: a drag box from 0 to 100.

Each drag is one undo step, so is each tick of None; each marks the room unsaved, so the
unsaved-changes dialog sees it. A change the rules refuse (a group for an enemy whose chance is not
100, a chance for an enemy in a group) leaves the room as it was and says why in the editor's
message line. The Test room plays every placement.

## A manual check

1. Open a room with three enemies in the editor and give each **Group** 1 (untick None). Save. Add a
   fourth enemy, give it **Chance %** 50 and Save again. Add a fifth with Chance % 0.
2. F4, Run (debug), Seed: tick **Fixed seed**, type `1` and start a run. Walk to that room and
   note which enemies are there: one of the three grouped enemies, the chance 50 one or not, and
   not the chance 0 one.
3. Die or leave the run and start another with the same seed: the **same** enemies.
4. Change the seed (`2`, `3`, ...): other enemies in some of them (two seeds can pick the same
   one; try a few).
5. Press F5 in the room: the same enemies stay.
6. End the run: the result screen's last line is `Seed: <16 hex digits>`. Type those digits into
   the F4 text box with Fixed seed on and start a run: the run plays out as before.
7. In F4, type `xyz` and tick Fixed seed: it is refused with the reason shown, and the old seed is
   kept. Try an empty box, `0x` and 17 digits too.
8. In the hub, and in the editor, and in **Test room**: all five placements are there.
9. Open the room in the editor during a run and Save: the file still has all five placements.
10. Un-fix the seed: each new run takes the next seed, and the result screen shows a different one.

# The dungeon flip

The same fixed dungeon can be played **mirrored left to right**, so one dungeon plays as two. This
section is the pure half: a room seen in a mirror (`src/mirror.rs`, `src/mirror/world.rs`). Nothing
there reads a file, a seed or the game; `game::world_load` calls it when a run is flipped.

## The symmetry assumption

A flipped room only plays the same as the original if **the player is left-right symmetric**: the
same run, jump, air jump, dash and nail swing, mirrored. That is an assumption about `Player`, so it
is proved, not trusted: `tests/flip.rs` steps the real `Player` through a scripted sequence on a
small world and on its mirror with every move input mirrored, and asserts at every tick (within
1e-3) that x and horizontal velocity are negated, y and vertical velocity are the same, and
grounded, facing and the strike hitbox agree. **If that test ever fails, the flip must not ship**:
something in the player now treats left and right differently. Re-run it after any change to
`player.rs`, collision or the dash.

One thing that is not symmetric by construction: an enemy starts at the **left end of its patrol**
facing right (`Enemy::from_definition`). A flipped placement swaps its ends, so it starts at the
mirror of the original's *right* end. Its patrol and everything after the first moments are mirrored.

## A room in a mirror (`mirror_layout`, `mirror_document`)

The mirror is the vertical line through the middle of the room's `bounds`, so a room maps onto
itself: a point `x` goes to `2 * bounds.x + bounds.w - x`, a rectangle by its far edge, y never
changes, and `bounds`, `floor_y` and the name stay. Mirroring twice gives the room back (exactly,
for whole-number coordinates).

| Moved | How |
|---|---|
| spawn, items, charms | the point is mirrored |
| terrain, hazards, breakables, switches, barriers | the rectangle is mirrored by its far edge |
| a hazard's `facing` | Left and Right swap (Up and Down stay) |
| an edge exit on Left or Right | the edges swap; the offset (from the top) stays |
| an edge exit on Top or Bottom | `offset` becomes `bounds.w - offset - length` |
| a door | its rectangle is mirrored |
| a gate's `exit` | follows its exit (the exit is looked up in the room, a reference has no length) |
| `map_cell` | the column is negated, the row stays |
| an enemy's patrol | the ends swap and are mirrored: the left end is the mirror of the old right end |
| the boss | its position is mirrored |

**Not moved:** the terrain look and the background layers (drawing only, a known limit), and an
exit's arrival `spawn`: that is a point in *another* room, which only `flip_world` can mirror. A
document with no `layout` has no bounds, so `mirror_document` returns it unchanged.

## A whole dungeon (`flip_world`)

`flip_world(&mut documents, flipped)` mirrors every room `flipped` names and makes the links agree:

- the `spawn` of every exit that leads into a flipped room is mirrored in **that room's** bounds, so
  the player still arrives beside the matching exit (an exit into its own room is mirrored once);
- a gate's `Entrance::Arrival` follows the exit it names when that exit's room is flipped. The exit
  is looked up in its room **as it was before the flip**, because a top or bottom exit needs its
  length.

The game flips every room except the hub together, because a room mirrored alone would break the
dungeon (you would leave through one wall and arrive at the neighbour's same wall). The function is
still correct for any set of rooms, and the tests run all 32 sets of the fixture dungeon.

## What the tests prove

- `src/mirror/tests/layout.rs`: one test per kind of thing moved, the four edges and a door, patrols
  and the boss, mirroring twice (exact for whole numbers, within 1e-3 otherwise).
- `src/mirror/tests/world.rs`: arrivals into flipped rooms are free spots **and** the spot beside
  the matching exit, gates and entrances still agree with the exits, flipped rooms pass
  `RoomLayout::validate_geometry`, flipping twice gives the dungeon back.
- `src/mirror/tests/dungeon.rs`: the dungeon check gives the same number and kinds of problems before
  and after the flip, for a good dungeon and one made bad on purpose.
- The fixture dungeon (`src/mirror/fixture.rs`) is built in code: a hub, a start room with a key,
  the Dash and a hazard, a locked exit with a Dash gate, a door, a top and a bottom exit.
