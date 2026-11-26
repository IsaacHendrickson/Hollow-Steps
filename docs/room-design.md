## Room design

Default room geometry is defined in Rust in `src/room.rs`, with rectangular bounds, a floor,
and exits anchored to the left, right, top, or bottom edge. Exit spans are
offsets along that edge; each exit stores a destination room and player-center
spawn coordinate in that room. Overlapping an exit with the player's body
after movement triggers one transition per update. Spawns sit outside exit
zones so arriving does not immediately send the player back.

`GameState` owns the active room and retains each room's items for the session.
Transitions reset movement and recompute grounding at the destination while
preserving inventory. Room geometry and transitions remain window-free library
logic; the binary only renders the active room and its exits. Enemy placements
are loaded from the matching RON files in `assets/rooms/`.

See [`AGENTS.md`](AGENTS.md) for the project conventions and the recommended
workflow for future changes.

## Room quality rules

A room passes Check dungeon when it can be finished. These rules are about whether it is worth playing. Each rule is a yes or no, so a room can be scored.

Purpose and pacing
- One idea per room: state it in one sentence; if the sentence needs "and", split the room.
- Teach, test, twist: a new idea first appears where failing costs little (a pit with a floor, a hazard with room to react), is tested one or two rooms later
  with a cost, and is twisted by combining it with an older idea.
- No more than two hard rooms in a row. After a key, an ability, a mini-boss or a gate comes a calm room (a short corridor with a coin trail is enough).
- Neighbouring rooms differ in size or shape. A dungeon has tall rooms, wide rooms and small ones.

Geometry
- Every gap and ledge is sized from the reach line (docs/reach.md). An ordinary crossing needs at most 80% of the no-ability reach. A gate crossing sits
  between the two envelopes with the margin the dungeon checklist asks for. The largest gap and ledge of each room is reported in px and as a percent of
  the reach.
- The player can see the landing before a jump. No blind drop into a hazard.
- Nothing hurts in the first second: no hazard, enemy patrol or projectile line comes within two player widths of an arrival point or the spawn.
- No landing spot lies within one player width of a hazard unless that is exactly the room's idea.
- No room is a flat floor with enemies on it. A room wider than the view has at least two height levels. A flat corridor is only for the rest role and
  needs something to look at.

Enemies
- Enemies are encounters, not scatter: one to three per room, each placed for a reason (guards a ledge, forces a timing, covers a coin). The reason is
  written in the plan.
- The first room with a new enemy kind holds one of it, alone, with space to learn it.
- A patrol range is readable: floor along its whole length, room to dodge, and the player can always hit it or avoid it.

Rewards and flow
- Risk pays: coins, charms and keys sit on the harder path or a detour. The direct path has at most a short coin trail as a guide.
- A side room has a payoff (coins, a key, an ability, a secret) or loops back. No empty dead ends.
- Every room has a landmark (a distinctive terrain shape, background or coin pattern) so it can be told apart on the map and in memory.
- Soft-locks: from any spot the player can reach, an exit or the arrival is reachable, or the spot is a deliberate one-way drop that the room's notes list.

A room's report (what its author writes in the handoff): the idea, the intended path, the largest gap and ledge (px and % of reach), where each enemy
stands and why, where the reward is, and which rules were broken on purpose and why.

## Lessons from playtests

One line per mistake found while playing, in the form: Don't <what>, because <what happened> (room, date). Every room chat reads this list; a line here
is a rule.

