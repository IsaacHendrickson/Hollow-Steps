# Terrain hazards

A hazard is a rectangle in a room that hurts the player when touched and throws the player away
from it. It is not solid: the player walks into it. Standing on top of one (feet exactly on its top
edge) or touching only a side is safe; the body has to overlap it. Its look (see "Look" below)
never changes the hit: collision is the rectangle, whatever is drawn in it.

## The hit

`Hazard::hit` (`src/hazard.rs`) first decides which side was touched: the side the body has gone the
least far into (overlap across versus overlap down). Then:

| Touched from | The player is thrown | Numbers used |
|---|---|---|
| the top (landing, or standing in it) | straight up | `knockback_speed` |
| underneath (jumping into a ceiling hazard) | straight down | `knockback_speed` |
| a side (walking or dashing in) | away from the hazard sideways, and up | `knockback_speed`, `lift_speed`, `knockback_secs` |

- The vertical speed is **set**, not added: a player falling at 900 px/s who lands on a hazard ends up
  moving up at `knockback_speed`. A player already moving that way faster is not slowed. A speed of 0
  leaves the player's vertical (or sideways) motion alone but still takes the damage.
- Exactly level with the hazard's center counts as the top; exactly in line with it as the right.
- Walking speed is 220 px/s, so a sideways push under that is undone by holding the key.
- `DamageResolver::gather` (`src/damage.rs`) is given the player's velocity for this, and lists hazard
  hits first, then enemy and boss attacks. The existing rules decide the rest: the strongest damage
  wins and a hit weaker than an enemy's contact damage gives way. The one-second immunity after any
  hit applies, so a hazard hits once, then again only if the player is still inside it when the
  immunity ends. A respawn, and the start of a Test room, leave the player immune for that second, so
  walk in after it ends when trying a hazard.

## Look

`src/hazard_look.rs` turns the rectangle into a draw list (`terrain_style::Shape`), the same list type
the terrain uses, so the game (`scene_hazards`) and the editor canvas (`editor_visuals`) draw it with
one adapter. It never draws outside the rectangle.

- **Shape** (`style`): `Spikes` (a row of whole triangles about 16 px wide, two tones), `Lava` (a body
  with a darker deep band, a bright crust whose waves move in play, a few glowing specks) or `Thorns`
  (a dark vine with thin, leaning thorns of uneven height).
- **Facing** (`facing`): `Up` (on a floor), `Down` (on a ceiling), `Left` (on a wall to the right of
  it) or `Right` (on a wall to the left of it). Points and the lava surface face that way and the
  base is the opposite edge. A tall, thin hazard against a wall is a **row** of spikes when it faces
  left or right; facing up it would be one needle as tall as the hazard.
- **Color** (`color`): each part 0 to 1, or none for the shape's own (steel, orange, green). Every
  other tone of the look is mixed from it.

Spikes and thorns keep their look when the hazard is moved or grown (each thorn draws from its own
number); a new size reshuffles only lava's specks. Nothing reads the clock: lava takes `time`.

## Room file

`hazards` is a list written only when it is not empty, so older room files load unchanged. `style`
and `facing` may be left out (spikes, up) and `color` is only written when set:

```ron
hazards: [
    (rect: (212.0, 444.0, 64.0, 16.0), damage: 10, knockback_speed: 420.0,
     lift_speed: 320.0, knockback_secs: 0.25, style: Lava, facing: Up,
     color: Some((0.9, 0.3, 0.1))),
],
```

Checks (`RoomLayout::validate_geometry`): finite numbers, at least 8 px wide and tall, fully inside
the room, damage 1 to 999, knockback speed and lift speed 0 to 2000 px/s, knockback time 0 to 2 s,
a color (if any) 0 to 1 in each part, and the room's spawn point must not be inside a hazard.
The first six fields are required; unknown fields are refused.

## Editor

- **Add hazard** (Room tools) puts a standard 64 x 16 spike strip on the floor next to the spawn point
  (to its right, else to its left). Drag it by its box like an item; it moves by its center and stays
  inside the room. Delete, the Delete key and the Erase tool work as for other markers.
- On the canvas a hazard is drawn in its real look under a faint magenta box (lava at rest).
- Selecting a hazard shows **Hazard width / height** (the center line and the bottom edge stay put, so
  a strip grows upward from the floor), **Damage**, **Knockback speed**, **Lift speed**, **Knockback
  time**, **Shape** and **Facing** (dropdowns), and **Color** (a swatch that opens the picker, and
  Reset to go back to the shape's color). A drag of a field, or of the color picker, is one undo step;
  a choice is one step. A refused value (for example a width that would reach the spawn) is not
  applied and the reason is shown.
- To make a spike wall: Add hazard, set width 16 and height 400 (or what the wall needs), set
  **Facing** to Left or Right, drag it against the wall.
- New hazards start with the numbers in `src/hazard.rs` (`NEW_*`); each hazard then keeps its own.
- Test room (F6) and Save include hazards like any other part of the room.

Known limits: other rooms' exit arrivals are not checked against hazards (an arrival inside one
hurts as soon as the player is not immune); hazards do not hurt enemies; no resize handles on the
canvas (use the width and height fields); facing changes only the look, never the hit (the throw
depends on the side touched, not on which way the spikes point); no hazard in the shipped rooms yet.

## Manual check

1. F2, **Add hazard**: a spike strip appears with the hazard fields; Test room (F6): spikes are drawn.
2. Wait a second, walk into the spikes from the side: health drops by the damage and the player is
   thrown away and up.
3. Jump onto the spikes from above: the player is thrown straight up, whatever the fall speed.
4. Set Shape to Lava and Thorns; pick a color; Reset. Each is one Undo. The canvas follows.
5. Width 16, height 400, Facing Left, drag against the right wall: a column of spikes along the wall.
6. Set Knockback speed to 0: no throw from above. Save and restart: the hazard is still there.
