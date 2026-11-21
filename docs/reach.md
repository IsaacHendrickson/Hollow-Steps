# Reach: the editor preview and the line after a save

`src/reach.rs` measures what a jump covers on the real `Player` physics (`Interval`: the longest gap
without Dash, with every move speed buff at its maximum, and the longest with Dash). A gate that
says "needs Dash" (`docs/gates.md`) is only honest while a gap is longer than the first and no
longer than the second. This file is about the two places the editor shows those numbers.

## Reach toggle (Room tools, View section, beside Grid and snap)

While it is on, the canvas draws two outlines in world coordinates around the selected entrance:
the room spawn, or the selected arrival of another room's exit (anything else selected, or nothing,
means the spawn).

- **light blue, "without Dash N px"**: the box a jump without Dash sweeps, N px to either side of the
  entrance and up to the top of the jump;
- **pink, "with Dash N px"**: the same with a Dash jump.

A doorway between the two outlines is one a player cannot reach without Dash and can with it.
The entrance is treated as the point the jump takes off from (the body standing there, at the edge
of its platform), so the outlines show the longest gap, not where the floor really is.

When the player's numbers leave no gap that needs Dash (the buffed jump without Dash is as long as
the Dash jump), nothing is drawn and the Room tools panel says why in red (`Interval::error`).

The interval is measured from the numbers as the Player panel, Charms panel and Upgrades panel hold
them, unsaved edits included, and is measured again only when those change (`ReachKey`); it is not
measured at all while the toggle is off (a measurement takes about 50 ms in a debug build).

## The line after a save

After the Player panel's Save or F4's Save writes `player.ron`, and only when a player number
changed, both show one more line under "Saved to": `jump 183 px, dash jump 296 px; run Check
dungeon` (`reach::line`). It is for the saved numbers with the charms and upgrades the game has.
When the saved numbers leave no gap that needs Dash, the line is the same red-panel explanation instead
(`Interval::error`). The line goes away with the saved message, and a Save that changed no player
number shows none.
Gates marked against the old numbers are not rechecked by a save; **Check dungeon** does that.

## Known limits

- The preview is a box, not a path: it does not know where the floor is, so a doorway inside it may
  still need a route the box does not show (a wall in the way, say).
- There is no margin for a human who does not jump at the pixel; the numbers are the best a
  perfect player does.
- Entrances only: regions inside a room (a ledge halfway) are not entrances.
- A bot that searches over inputs to prove each annotated crossing is not built; gates are marked
  by the author, and Check dungeon trusts the marks.
- Only Dash is an ability that gates a crossing, and dungeons are not generated.
