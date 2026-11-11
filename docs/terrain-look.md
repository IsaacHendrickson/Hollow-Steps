## Terrain look: design note

How a room's solids are drawn. Collision is never involved: the solids are the same
rectangles `src/terrain.rs` collides with, and `src/terrain_style.rs` only decides how they
look. It is pure (no window, no art files); `src/terrain_draw.rs` draws what it returns.

### What a solid gets

- **Fill**: the palette's `fill` over the whole rectangle, a lighter band (30% of the height,
  at most 14 px) under every open stretch of the top and a darker band over every open stretch
  of the bottom, so the gradient never shows a seam where another solid sits against it.
- **Inside the fill**: seeded 1 to 2 px speckles (light, dark and a few in the accent) and
  short cracks. Capped at 256 speckles and 16 cracks per solid.
- **Along exposed edges only**: a 2 px dark outline in `edge`; on an open top a 2 px lit strip
  in `highlight` and a jittered stone lip (trapezoids rising at most 3 px, tapering to nothing
  at the ends of the stretch); on an open top, moss tufts in `accent` (up to 6 px, only when
  `moss` is on); under an open bottom, stalactites (up to 12 px).

An edge stretch is *open* unless another solid, or the ground, fills the space just outside
it. Solids that touch, stack or overlap hide the seam between them (`terrain_style::exposure`);
when two solids share an edge line, the larger one keeps it. The floor counts as ground
(`ground_cover`), so a wall standing on the floor has no bottom outline or stalactites. The
floor itself is still drawn by `scene.rs` as before.

### Determinism

Every solid is seeded from its room, its own position and the palette's `seed_offset`, and
each feature (speckles, cracks, lip, moss, stalactites) has its own random stream, keyed by
where along the edge it works. So adding, removing or reordering other solids, or switching
moss off, never changes how a solid looks; moving a solid gives it the look of its new spot.
The generator is `particles::Rng`; nothing reads the clock.

The draw list stays within 6 px above a solid, 12 px below it and never past its sides.

### Palette: `layout.terrain_style`

Optional in a room file; every field is optional too, and anything missing is the default look
(the dark blue-grey of the original flat terrain). A default palette is not written back when
the editor saves.

```ron
terrain_style: (
    fill: (0.235, 0.165, 0.125),       // each color is (red, green, blue), 0 to 1
    edge: (0.075, 0.045, 0.035),       // outline
    highlight: (0.800, 0.580, 0.340),  // lit top strip and lip
    accent: (0.420, 0.620, 0.260),     // moss and some speckles
    seed_offset: 7,                    // change it to reshuffle this room's speckles and moss
    moss: true,
),
```

Colors must be finite and between 0 and 1; a bad one is a load error naming the file and the
color. The editor has no controls for the palette yet: edit the file and press F5.

### Drawing and caching

`TerrainLook` keeps the last draw list and the room, solids, ground and palette it was built
from, and rebuilds only when one of them changes. `scene.rs` draws it through the world
camera and `terrain_draw.rs` skips shapes outside the view. The list is drawn directly each
frame; it is not baked into a texture, because a texture would have to cover the whole room
(rooms may be larger than the screen) and could blur or seam under the follow camera. The
room editor keeps drawing solids as plain rectangles.
