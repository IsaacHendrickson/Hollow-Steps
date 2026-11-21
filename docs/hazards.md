# Terrain hazards

A hazard is a rectangle in a room that hurts the player when touched and throws the player away
from it. It is not solid: the player walks into it. Standing on top of one (feet exactly on its top
edge) or touching only a side is safe; the body has to overlap it.

## Rules in play

- `Hazard::hit` (`src/hazard.rs`) gives the hit for a touching body: the hazard's `damage`, a
  horizontal push of `knockback_speed` away from the hazard's center (to the right when the body is
  exactly in line with the center), an upward push of `lift_speed`, and the push lasts
  `knockback_secs`. Walking speed is 220 px/s, so a push under that is undone by holding the key.
- `DamageResolver::gather` (`src/damage.rs`) lists hazard hits first, then enemy and boss attacks.
  The existing rules decide the rest: the strongest damage wins and a hit weaker than an enemy's
  contact damage gives way. The player's one-second immunity after any hit applies, so a hazard
  hits once, then again only if the player is still inside it when the immunity ends.
- A respawn, and the start of a Test room, leave the player immune for the immunity time (one second), so walk in after it ends when trying a hazard.
- Hazards are not drawn as solids and never block movement; `scene_hazards` draws spikes.

## Room file

`hazards` is a list written only when it is not empty, so older room files load unchanged:

```ron
hazards: [
    (rect: (212.0, 444.0, 64.0, 16.0), damage: 10, knockback_speed: 420.0,
     lift_speed: 320.0, knockback_secs: 0.25),
],
```

Checks (`RoomLayout::validate_geometry`): finite numbers, at least 8 px wide and tall, fully inside
the room, damage 1 to 999, knockback speed and lift speed 0 to 2000 px/s, knockback time 0 to 2 s,
and the room's spawn point must not be inside a hazard. All fields are required.

## Editor

- **Add hazard** (Room tools) puts a standard 64 x 16 hazard on the floor next to the spawn point (to
  its right, else to its left). Drag it by its magenta box like an item; it moves by its center and
  stays inside the room. Delete, the Delete key and the Erase tool work as for other markers.
- Selecting a hazard shows **Hazard width / height** (the center line and the bottom edge stay put, so
  a strip grows upward from the floor), **Damage**, **Knockback speed**, **Lift speed** and
  **Knockback time**. A drag of a field is one undo step. A refused value (for example a width that
  would reach the spawn) is not applied and the reason is shown.
- New hazards start with the numbers in `src/hazard.rs` (`NEW_*`); each hazard then keeps its own.
- Test room (F6) and Save include hazards like any other part of the room.

Known limits: other rooms' exit arrivals are not checked against hazards (an arrival inside one
hurts as soon as the player is not immune); hazards do not hurt enemies; no resize handles on the
canvas (use the width and height fields); no hazard in the shipped rooms yet.

## Manual check

1. F2, **Add hazard**: a magenta box and magenta "Hazard" fields appear; Test room (F6): spikes are drawn.
2. Wait a second, walk into the spikes: health drops by the damage and the player is thrown back and up.
3. Raise Damage, widen the hazard (drag it away from the spawn first), Undo once per drag.
4. Set Knockback speed to 0: the player is only lifted. Save, restart: the hazard is still there.
