# Variety: the run seed, the dungeon flip and flippable rooms

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

# Whether a run flips (`variety.ron`, F4)

`data/variety.ron` is the embedded default and `assets/variety.ron` replaces it **whole** (read with
`assets::read_override`, unknown fields refused, a bad file stops the game starting and the error
names it). It has one number:

| field | meaning |
|---|---|
| `flip_percent` | chance, 0 to 100 (finite), that a run plays the dungeon mirrored |

The embedded default is `0.0` (never), so a game with no file plays as it always did. The shipped
`assets/variety.ron` says `50.0`. F5 does not re-read it: restart the game after editing it.

**The decision.** When a run starts, `variety::decide_flip` looks at F4's choice:

- **Auto** flips when `stream(seed, "flip", 0).unit() * 100 < flip_percent`, so the same seed always
  gives the same answer, and 0 never flips and 100 always does;
- **Never** and **Always** ignore the file and the seed.

The answer is kept in `RunState` until the run ends (the result screen still belongs to that run;
the hub, a game with no hub and the test room are never flipped). Changing F4 during a run does not
change that run, only the ones after it.

**F4 row.** The "Run (debug)" section has "Flip next run" with three radio buttons (Auto, Never,
Always) and a line saying what the next run will do and whether this one is flipped. It does not
edit `variety.ron` and is not saved in the profile, so it is back on Auto the next time the game
starts.

# A flipped run in play

`world_load::load_world` is where it happens, and the only place. When the run's `Layer` says
`flip`, `mirror::FlippedDocuments::read` reads every room file (the three fixed rooms and every
`room_<n>.ron`), runs `flip_world` over all of them but the hub's, and the same room builders as
ever (`room::load_rooms_from`, `editor_files::load_extra_rooms_from`) build from those documents.
Nothing else about building a room changed: the plain `load_rooms` and `load_extra_rooms` call the
`_from` versions with a reader that reads the file, so the editor sees exactly what it always did,
and the difficulty layer goes on top of the flipped rooms as before. A file the flip could not
read is read by the builder itself, so every loading error is the plain one, with the same words.

- **Start of a run.** `start_run` decides, builds the world with the flip in the layer, and keeps
  the layer for the run. `use_exit` then takes the arrival point from the **rebuilt** hub's exit
  (the same exit: same edge and offset, or the same door), not from the copy it was called with,
  because `flip_world` moved that point into the mirrored start room.
- **F5.** The reload uses the held layer, so a mid-run F5 flips again the same way. In the hub the
  layer is `BASE`, so nothing flips.
- **Never flipped.** The hub (its walls, spawn and the side its door is on), a game with no hub, and
  the world after the result screen's Continue (`continue_after_result` rebuilds with `BASE`). The
  result screen itself still shows the frozen flipped world, so it counts as a flipped run for the
  editor's refusal and for F5.
- **The editor.** While a run (or its result screen) is flipped, F2 and Open room refuse, with a
  message that says to abandon the run from the pause menu or to set Flip next run to Never in F4.
  The editor reads the files, so it would show the unflipped dungeon and Save would put that over
  the flipped rooms being played. Everything else is untouched: the editor works in the hub and in
  an unflipped run, and a test room plays the files as they are. Re-applying the flip in Save would
  be a small change (Save rebuilds its rooms with the held layer already), but the editor would still
  show unflipped rooms, so the refusal is the rule.
- **A run started from the editor.** Open room in the hub can leave the player in another room, and
  the run starts when the editor closes. When that run is flipped the player is put at the room's
  (mirrored) spawn, because the spot he stood on belongs to the unflipped room.
- **The map.** It is built from the live rooms, so it shows the mirror image with no code of its
  own: each room's `map_cell` is negated in x by the mirror, the hub's is not, and rooms without a
  cell are placed from the exits, which are mirrored.

## Known limits

- The hub is never flipped, and its door keeps its side. The player can therefore arrive on the far
  side of the start room, and the start room's way back to the hub is on the other wall.
- The look and background (terrain style, background layers) are not mirrored; they are drawing
  only.
- A flipped run closes the editor (see above), until the run ends or is abandoned.
- Enemies start at the left end of their patrol, facing right, so a mirrored patrol begins at the
  mirror of the old *right* end: the first moments of a room are not the mirror of the original.
- Rooms with no `layout` (a legacy placement file) cannot be mirrored and are left as they are,
  with the arrival points into them. Every shipped room has a layout.

## Manual check

1. F4, Run (debug), Flip next run: **Always**. Walk out of the hub: the run starts, every room is
   a mirror image of the editor's (exits, hazards, terrain, doors, enemies), the hub did not change,
   and every exit leads to the right room and lands you beside the exit you came through.
2. F4, Flip next run: **Never**. Abandon, return to the hub, start a run: everything is as authored.
3. **Auto** with `flip_percent` 50: start several runs with different seeds; about half flip, and
   the same fixed seed gives the same answer.
4. In a flipped run open the map (M): it is the mirror image of the authored one.
5. In a flipped run press F2: refused, with the reason. Abandon from the pause menu, Continue: the
   hub is unflipped and F2 opens the editor.
6. Die in a flipped run, press Continue: the hub, and the next run with Never, are unflipped.
7. F5 in the middle of a flipped run: the rooms stay mirrored.

# Flippable rooms

A room can be marked **flippable** in the room editor (Room tools, "Seed flip", the "Flippable"
box; `flippable: true` in the room file, absent means no and older files load as no). A run's seed
then decides, **for each marked room on its own**, whether that room is played mirrored left to
right. The editor, Save, the Test room and the files always have the room as authored.

**The draw.** `variety::RoomFlips::flips(room)` is `stream(seed, "flip-room", key of the room)`'s
first number as a percent, under `room_flip_percent`. The key is the room's id, so the same seed
always mirrors the same rooms, a room's draw does not move when rooms are added, and it shares no
numbers with the dungeon flip (`flip`) or the enemy picks (`enemies`). `room_flip_percent` is a
second number in `variety.ron`, 0 to 100, **50 when a file says nothing** (so a file from before
it still loads); each marked room is mirrored in half of the runs by default. Both shipped and
embedded files say `50.0`. Only marked rooms are ever asked, and **the hub is never mirrored**,
marked or not.

**With the whole-dungeon flip.** The two combine as an exclusive or: a room is mirrored when the
dungeon flips **or** its own draw says so, **but not both**. A marked room whose draw says flip in
a mirrored dungeon is therefore mirrored back. The reason is the odds: with "or", a marked room
would be mirrored in three runs of four whenever the dungeon flips half the time, and with the
exclusive or it is mirrored in half of them whatever the dungeon does (a test sweeps 200 seeds).
The map follows the dungeon alone: a room's `map_cell` is a place on the map, not a part of the
room, so a room mirrored by itself keeps its cell and only a flipped dungeon negates cells.

**How it is built.** `mirror::FlippedDocuments::read_with(assets, hub, dungeon, rooms)` picks the
set of rooms to mirror and hands it to `flip_world`, which was written for any set of rooms: it
mirrors each one, mirrors the arrival `spawn` of every exit that leads into one, and moves gates
and entrances with their exits. `world_load` calls it when the layer says to flip the dungeon or
names some rooms (`Layer::room_flips`). Nothing is written and the loaders are the same ones.

**Which rooms, per run.** `decide_room_flips(choice, rules, seed)` makes a `RoomFlips`, kept in the
layer for the run's life (F5 rebuilds the same rooms):

- **Auto** asks each marked room's draw against `room_flip_percent`;
- **Never** mirrors no room, **Always** mirrors every marked room (F4's "Flip rooms next run").

F4's Run (debug) section has the second row, "Flip rooms next run", beside "Flip next run", with
a line saying what the next run does and how many rooms this run mirrored. It edits no file.

**The editor.** Like a flipped dungeon, a run that mirrored at least one room closes the editor
(F2 and Open room refuse, naming Flip rooms next run), because the editor reads the unmirrored
files and Save would put them over the rooms being played. A run in which no room was mirrored
(none is marked, or no draw said flip) leaves the editor available. `GameState::run_flipped` is
true for either kind of run; `dungeon_flipped` and `flipped_room_count` tell them apart.

**Known limits of a single mirrored room.**

- A neighbour that is not mirrored still sends the player through the same wall as before, so the
  mirrored room is entered from the side its authored layout enters from the other way round: leave
  a room by its right exit, arrive in a mirrored room at its right side, beside the exit back, and
  cross it right to left. Every arrival is a free spot beside the matching exit (a test checks
  every exit), but a mirrored room is crossed against the direction of the rooms around it. Mark
  rooms where that suits: a room whose exits are on the same side, a closet behind a door, a
  boss arena, a vertical shaft.
- Look and background are not mirrored (drawing only), as for the dungeon flip.
- An enemy's first moments in a mirrored room are not the mirror of the original's (see above).
- A dungeon check (`Check dungeon`) and the reach envelope work on the files as authored. They do
  not see the mirrored rooms: a Dash gate is a claim about geometry, and the symmetry test
  (`tests/flip.rs`) is what says the mirrored geometry plays alike.

**Manual check.**

1. Open a room in the editor, tick "Seed flip" > "Flippable", Save. F4, "Flip rooms next run":
   **Always**, "Flip next run": **Never**. Start a run: that room is mirrored (exits on the other
   sides), every other room is as authored, every exit leads to the right room.
2. F4, "Flip rooms next run": **Never**: the run is as authored.
3. **Auto** with `room_flip_percent` 50: start several runs with different F4 seeds. About half
   mirror the room, and the same fixed seed gives the same answer.
4. In a run that mirrored the room press F2: refused, with the reason. Abandon, Continue, F2: the
   editor opens, and the room is as authored there.
5. Mark the hub flippable: it is still never mirrored.
