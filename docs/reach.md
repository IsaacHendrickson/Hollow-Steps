# Reach: the editor preview and the line after a save

`src/reach.rs` measures what a jump covers on the real `Player` physics, for a player with no ability
and for a player with each ability (`Interval`):

- **no ability**: the longest gap and the highest ledge a jump manages with every move speed buff
  (charms, upgrades) at its maximum;
- **Dash**: the longest gap with one dash in the jump, at base stats. A dash only stops the fall for a
  moment, so it adds distance and no height;
- **Double Jump**: the longest gap and the highest ledge with the air jump, at base stats.

A gate that says "needs Dash" or "needs Double Jump" (`docs/gates.md`) is only honest while some gap
(or ledge) is longer than the first number and no longer than that ability's. That range is the
ability's *interval* (`Interval::gap(ability)`, `Interval::height(ability)`); an ability whose
interval is empty can be needed by no length at all. This file is about the places the editor shows
those numbers.

## How a number is measured

Always by stepping a real `Player` at the fixed step on a flat test world with a pit, never by a
formula that could drift from the physics. The player walks off an edge and jumps at the best tick
of the coyote window, jump held and moving on.

- Dash: one dash at the best tick of the flight.
- Double Jump: a second press at the best tick of the flight, jump held, on a `Player` that holds
  the ability. The air jump sets the upward speed to `double_jump_speed` (`player.ron`, Player
  panel, F4), so the best second press is near the top of the first jump.

`double_jump_speed` is one of the player's own numbers: charms and upgrades never change it, and a
test lists every charm effect to keep it that way. The shipped value is a placeholder, 0.8 times the
jump speed.

## Reach toggle (Room tools, View section, beside Grid and snap)

While it is on, the canvas draws outlines in world coordinates around the selected entrance: the room
spawn, or the selected arrival of another room's exit (anything else selected, or nothing, means the
spawn). Each outline is a box as wide as that jump's distance to either side of the entrance and as
tall as its height above the top of the body:

- **light blue, "without any ability N px high / N px far"**: the plain jump;
- **pink, "with Dash N px"**: a jump with a dash (as tall as the plain jump);
- **yellow-green, "with Double Jump N px high / N px far"**: a jump with the air jump, taller than
  the others.

A doorway between the plain outline and an ability's outline is one a player cannot reach without
that ability and can with it. A ledge above the plain outline and inside the Double Jump's needs the
Double Jump. The entrance is treated as the point the jump takes off from (the body standing there,
at the edge of its platform), so the outlines show the longest gap and the highest ledge, not where
the floor really is.

The Room tools panel names the abilities that have an outline ("Reach from the room spawn: the
outlines are a jump with no ability, and with Dash and Double Jump"). An ability that no gap or ledge
can need with the player's numbers gets no outline, and the panel says why in red, one message for
each empty interval, naming both numbers: "no gap can need Dash: without it the best jump with every
buff crosses N px, but with Dash a jump crosses only M px; ...", or "no ledge can need Double Jump:
without it the best jump rises N px, but with Double Jump a jump rises only M px; ...". Double Jump
adds both distance and height, so it has a message for each. When no ability has an interval,
nothing is drawn, the plain outline included.

The interval is measured from the numbers as the Player panel, Charms panel and Upgrades panel hold
them, unsaved edits included (`double_jump_speed` too), and is measured again only when those change
(`ReachKey`); it is not measured at all while the toggle is off. A measurement takes about 0.2 s in
a debug build (`cargo run` without `--release`, the shipped numbers, the machine this was written on),
so dragging a Player panel slider with Reach on is sluggish there. A release build was not timed.

## The line after a save

After the Player panel's Save or F4's Save writes `player.ron`, and only when a player number
changed, both show one more line under "Saved to": `jump 389 px, dash jump 509 px, double jump 266 px
high / 512 px far; run Check dungeon` (`reach::line`). It is for the saved numbers with the charms and
upgrades the game has, and names every ability's numbers (those were the shipped numbers when this
was written). When some ability's interval is empty the line is the red-panel explanation of each
empty one instead, with no numbers and no "run Check dungeon". The line goes away with the saved
message, and a Save that changed no player number shows none. Gates marked against the old numbers
are not rechecked by a save; **Check dungeon** does that.

## Known limits

- The preview is a box, not a path: it does not know where the floor is, so a doorway inside it may
  still need a route the box does not show (a wall in the way, say).
- There is no margin for a human who does not jump at the pixel; the numbers are the best a
  perfect player does.
- Entrances only: regions inside a room (a ledge halfway) are not entrances.
- A bot that searches over inputs to prove each annotated crossing is not built; gates are marked
  by the author, and Check dungeon trusts the marks.
- Abilities are measured one at a time. A crossing that needs both the Dash and the Double Jump
  (a dash after an air jump) is not measured: a gate may name both, but no outline shows what they
  reach together.
- Only Dash and Double Jump are abilities that gate a crossing. Not built: a wall jump, a glide, air
  jumps with their own cooldown, and a double jump that is refilled by a pogo hit (it refills on
  landing only). Dungeons are not generated.
- A measurement is costly in a debug build (see above). The no-ability scenario also measures a
  dash jump and a double jump that nothing reads; an existing test pins the scenario as exactly
  `Reach::of`, so that waste stays until that test is changed on purpose.
