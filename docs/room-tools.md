# Room tools: the preview and the path replay

Two dev tools for making rooms that are good, not only finishable (`docs/room-design.md` has the
rules they serve). Neither is part of the game and neither changes it.

- **Room preview**: prints a room as text, so a room can be read, measured and compared in a
  chat or a diff. `src/room_preview.rs`, run with `examples/room_preview.rs`.
- **Path replay**: plays a short script of inputs on the real `Player` in a room and says whether it
  gets to an exit; with a gate's ability or a lock's key taken away, it proves the gate or lock
  is needed. `tests/support/path.rs`, used from tests.

## Room preview

```text
cargo run --example room_preview -- first
cargo run --example room_preview -- hub
cargo run --example room_preview -- room_0 --cell 16
cargo run --example room_preview -- assets/rooms/room_0.ron
```

The room is `first`, `second`, `boss`, `hub` (the hub room of `run.ron`), a number or `room_<number>`
(a saved room, `room_0` is the hub), or the path of a room file (`<assets>/rooms/<name>.ron`). `--cell N` is pixels per
character. `--assets DIR` picks another assets folder (default: the one next to the crate, or the one
the room file lies in).

The room is loaded by the code the game uses (`load_rooms` for the three built-in rooms,
`RoomDraft::build` and `place_boss` for a saved one), so the text shows what the game would load,
and a room the game refuses is refused with the game's message. It shows the room as authored:
nothing is rolled (a placement's tier, group and chance are listed in the table, not applied).

### The cell

The default cell is the smallest of 4, 8, 16, 32 and 64 px per character that keeps the widest room
of the assets within 200 characters, so every room is drawn at the same scale (the shipped rooms
are 960 px wide at most: 8 px, 120 characters). The cell used is printed in the header. A smaller
cell shows more, a larger one fits a screen: a platform 8 px thick is exactly one `#` at 16 px and
half a cell at 8 px.

### The header and the ruler

```text
Room first: 960 x 460 px at (0, 0), floor y 460
Cell: 8 px per character; the picture is 120 x 58 characters
Ruler: player 32 x 48 px (4.0 x 6.0 cells)
  jump, no ability, no buffs           far 307 px (38.4 cells), high 163 px (20.3 cells)
  jump, no ability, every buff at max  far 389 px (48.7 cells), high 163 px (20.3 cells)
  jump with Dash                       far 509 px (63.6 cells), high 163 px (20.3 cells)
  jump with Double Jump                far 512 px (64.0 cells), high 266 px (33.3 cells)
```

The ruler is measured on the real `Player` by `src/reach.rs` (`docs/reach.md`) at the shipped tuning
(`player.ron`, the vitals, the charms and the upgrades), the same numbers as the editor's Reach
preview and Check dungeon. *Far* is the longest gap between two platforms at one height; *high* is the
highest ledge. **No buffs** is the base player. **Every buff at max** is the best a player gets with no ability, with
the charms and upgrades at their maximum: a gap longer than this needs an ability. **Dash** and
**Double Jump** are base stats with that ability. Cells convert a length in the picture: a gap that is
12 characters wide is 96 px at 8 px per character. A line per ability is added by itself when the
repo gets a new ability.

### The picture

One character per cell, y growing down; the left margin gives y in px every ten rows and the line
under it gives x in px every ten columns. Positions are room coordinates, the numbers in the room file.

| Mark | Meaning |
| --- | --- |
| `.` | air |
| `#` | terrain that covers at least half the cell |
| `=` `\|` `+` | terrain that covers less: a flat sliver, an upright one, a corner. A thin platform never disappears |
| `@` | the spawn: the player's body, 32 x 48 px |
| `1`-`9`, `a`-`z` | the exits in file order (a door is an exit too); the table says which is which |
| `^` | hazard (a hazard marks every cell it touches) |
| `%` `/` `~` | breakable wall, switch, barrier |
| `$` `D` `J` `K` | amber shard, Dash, Double Jump and key pickups |
| `C` `*` `S` | charm, coin, pickup slot |
| `E` | an enemy's body |
| `<-->` | an enemy's patrol range (body centers); the ends show over the body |
| `B` | the boss's body |

Terrain is judged by how much of a cell it covers, counting overlapping solids once. Exits, hazards
and puzzle pieces mark every cell they touch; bodies and pickups mark the cells they cover at least
half of (or the cell of their center). Where two things share a cell the more important one wins
(spawn, exits, boss, enemy, hazard, pickups, puzzle pieces, thin terrain, terrain, patrol line); the table lists
everything, so nothing is lost. Terrain below the floor line is drawn as `#`; with none, the bottom
edge of the picture is the floor.

### The table

One row per placed thing: mark, kind, the top-left corner and size in px, and the details: an exit's
side, offset and length (or `door`), the room and arrival point it leads to, `locked by Key N` and
each gate (`gate: needs Dash from the spawn`); a hazard's style, facing and damage; an item's kind;
an enemy's definition, patrol endpoints and length, and its tier, group and chance when they are not the plain case; the boss. Every terrain
rectangle has a row.

### Reading a room for the rules

Measure with the ruler and the table: the widest gap and the highest ledge in cells and px, each
as a percent of the *far* and *high* numbers (rule: an ordinary crossing is at most 80% of the
no-ability reach); how far a hazard, patrol or the arrival lies from the spawn (rule: two player
widths, 64 px); where the enemies stand. Compare the picture with the editor canvas when something looks
wrong: the preview is built from the same loaded room.

### Limits

- A text picture, not to scale at thin sizes: a 4 px ledge and a 12 px one can both be one `=`. Use
  the table for exact numbers and a smaller `--cell` for detail.
- Backgrounds and terrain looks are not drawn. The arrival points other rooms give this room are not
  drawn (only the spawn is).
- Enemies are shown where the game spawns them; their patrol line is the placement's range, not a
  simulation.

## Path replay

`tests/support/path.rs` (with `path/`) replays a script on the real `Player`, 120 ticks a second, at the
shipped tuning (`PlayerConfig::default()`), on a room's own terrain, hazards and exits. A room is a
`RoomLayout`: build one in code (as `tests/path_replay.rs` does), or read a room file with
`editor_files::read_document(path)`, whose `RoomEnemies` holds the layout in `layout` (an `Option`).
Only `tests/shipped_content.rs` reads `assets/`; other tests use `tests/fixtures/rooms/` or rooms built in code.

### Writing a script

```rust
mod support; // at the top of the test file

use hollow_steps::ability::Ability;
use support::path::*;

// A dash gap: two platforms, 400 px of floor then a gap between the jump's reach (307 px) and the dash's (509 px).
let room = gap_room(408.0);
let script = PathScript::new([
    run_right(160),   // hold right for 160 ticks
    jump(170),        // press jump, keep it down for 170 ticks, while the script goes on
    run_right(70),    // still holding right and jump: the climb
    dash(),           // press dash near the top of the jump (the heading is kept)
    run_right(200),
    wait(40),         // release everything and settle
])
.with_ability(Ability::Dash);
let far_side = Target::area(908.0, 552.0, 600.0, 48.0); // standing on the far platform

assert_reaches(&room, Arrival::Spawn, &script, far_side);
assert_blocked_without(&room, Arrival::Spawn, &script, far_side, Ability::Dash);
```

(`gap_room` is a helper that builds the `RoomLayout`; the whole test is `a_dash_gap_needs_the_dash` in `tests/path_replay.rs`.)

- `run_left(n)`, `run_right(n)`: hold a direction for `n` ticks. `wait(n)`: hold none.
- `jump(hold)`: press on one tick and keep the key down for `hold` ticks (the press included) while the next
  steps run, so a dash can come in the middle of a held jump. It takes one tick of the script.
- `dash()`: press on one tick. A jump or a dash keeps the heading of the step before it.
- `.with_ability(a)`, `.with_key(n)`: what the player holds. `.with_config(c)`: other player numbers.
- One tick is 1/120 s: the move speed (220 px/s) is about 1.83 px a tick, so a run of
  `(distance / (config.move_speed * FIXED_STEP_SECONDS))` ticks covers a distance. Size a crossing from the
  reach numbers (the ruler), not by trial: a jump from the edge covers its *far* number at best, so
  prove a gap of 70% of it with a plain jump.

### Proving a path

Arrivals: `Arrival::Spawn`, or `Arrival::of(&exit)` for the arrival point an exit of another room gives. Targets:
`Target::Exit(index)` (the exit's trigger area, reached the first tick the body touches it; a locked exit counts only
when the script holds its key; a door also needs the body's center within its width, and Up is not modelled) or
`Target::area(x, y, w, h)` (the body's center is inside it at the end of the script).

- `assert_reaches(room, arrival, script, target)`: the script gets there without touching a hazard.
- `assert_blocked(room, arrival, script, target)`: it does not.
- `assert_blocked_without(room, arrival, script, target, lack)`: it gets there as it stands, and does not once
  `lack` (an `Ability`, or `Lack::Key(n)`) is taken away. A script that never gets there proves nothing about a
  gate, so that half is required too.
- `check_reaches`, `check_blocked`, `check_blocked_without` return the failure text instead of panicking.

A failure prints what was tried (arrival, what is held, the target, the script length), how it ended, and a
trace: a row every 10th tick with position, velocity, grounded or air, the nearest terrain edge and any hazard
touched, then the last tick:

```text
the path does not reach its target
from the spawn, holding Dash, to area x 908..1508, y 552..600; the script is 425 ticks
ended at (792.1, 976.0) after 425 ticks, on the ground
 tick        x        y       vx       vy  state    nearest terrain edge         hazard
  170    411.7    549.4    220.0   -439.2  air      terrain #0 dx 0 dy 27        -
  190    477.3    503.2    800.0      0.0  air      terrain #0 dx 61 dy 73       -
  300    766.0    907.7    220.0    900.0  air      terrain #1 dx 26 dy 0        -
```

(`dx 26` is the gap still to the far platform when the player fell past it.) A hazard the body touches ends
the replay at that tick: the player is hit.

### The stop rule

A script that cannot pass after two serious tries means the room is wrong, not the script. Read the trace
and the ruler: when the gap is not under the reach the script is meant to use (70% for a plain jump; between the
no-ability and the ability numbers for a gate), change the gap, the ledge or the platform, then write the script
once more. Do not keep tuning ticks until a number fits.

### Limits

- Scripts are written by hand; nothing searches for inputs. A script that passes proves a path exists; one that fails
  proves nothing alone, which is why `assert_blocked_without` needs the script to pass first.
- Enemies, projectiles, the nail, a door's Up press, switches and breakable walls are not played (walls and barriers are
  solid as the room starts). A hazard ends the replay at the touch; knockback and the second of immunity are not played.
- Only Dash and Double Jump are abilities; a crossing that needs both is a script holding both.
