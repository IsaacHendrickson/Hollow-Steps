## Terrain look: design note

How a room's solids and its boundary are drawn. Collision is never involved: the solids are
the same rectangles `src/terrain.rs` collides with, the boundary is visual only, and
`src/terrain_style.rs` only decides how they look. It is pure (no window, no art files);
`src/terrain_draw.rs` draws what it returns.

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
when two solids share an edge line, the larger one keeps it. A wall standing on the floor has
no bottom outline or stalactites because the boundary's floor slab (below) is under it; when
the boundary is off, the floor counts as ground instead (`ground_cover`) and `scene.rs` draws
the flat floor strip as before.

### Room boundary

`build_room` draws a whole room: the solids plus, when the look asks for it (`draw_boundary`,
on by default), four slabs around the playable area. The slabs are ordinary rectangles fed to
the same code as the solids, so they get the same fill, speckles, cracks, outline, lit top, lip,
moss and stalactites, and the seams between them are hidden like any other touching solids.
They are **visual only**: `Room::bounds` and `floor_y` still do all the blocking.

Geometry (`terrain_style::boundary_slabs`; the choice is documented in `boundary.rs`). The
slabs sit *outside* the playable rectangle, flush against it, and never overlap each other:

- the **ceiling** runs over the whole top, corners included;
- the **left and right walls** stand between the ceiling and the floor slab;
- the **floor slab** starts at `floor_y` (the top of the floor) and runs `thickness` below the
  bottom of the bounds and out under the walls. When `floor_y` is above the bottom of the
  bounds it also fills the ground under the floor line inside them, so the room's own floor
  strip is replaced too.

So for a room with bounds `(x, y, w, h)` and thickness `t` the pieces are the ceiling
`(x-t, y-t, w+2t, t)`, the walls `(x-t, y, t, floor_y-y)` and `(x+w, y, t, floor_y-y)` and the
floor slab `(x-t, floor_y, w+2t, y+h-floor_y+t)`. The inner faces (toward the room) are
exposed, so the ceiling drips stalactites into the room and the floor has the lit top, lip and
moss; the outer faces are outlined too.

**Exits are never covered.** An exit zone is a 32 px strip against the inside of an edge, so
the slabs never reach it; each exit also gets a doorway cut through the whole slab over its
span, so the opening reads as a gap in the wall, ceiling or floor rather than a painted-over
patch. The decoration next to a doorway stays out of the zone (a test checks every shape of the
list against every exit zone).

The camera clamps to the room, so in a room at least as large as the 960x540 view you see only
the floor top, the stalactites and whatever slab shows beyond a room edge that is shorter than
the view (room 1 is 460 px tall, so its ceiling and floor slabs show in the bands above and
below); a room smaller than the view, and the editor canvas, show the whole frame.

`Look` is the resolved look `build_room` draws from: the palette plus `speckle_density` and
`crack_density` (1 is the base rate, 0 draws none; the 256 and 16 caps still hold),
`boundary_thickness` and `draw_boundary`. A thickness that is not a positive finite number
gives no slabs and the room falls back to ground cover, exactly like `draw_boundary: false`.

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

`TerrainLook` keeps the last draw list and what it was built from (the room, its bounds,
floor, exits and solids and the look, or for `shapes` the solids, ground and palette) and
rebuilds only when one of them changes; `room_shapes` also says whether the boundary is in the
list. `scene.rs` draws it through the world camera and `terrain_draw.rs` skips shapes outside
the view. The list is drawn directly each frame; it is not baked into a texture, because a
texture would have to cover the whole room (rooms may be larger than the screen) and could
blur or seam under the follow camera. The room editor keeps drawing solids as plain rectangles.

Draw order in `scene.rs`: clear, background layers, the boundary and solids (one list; the old
flat floor strip and room outline only when the boundary is off), exits, items and coins,
enemies, the boss, the player; the HUD is drawn after, in screen space. All of it is in world
space through the camera, with no window-size pixel constants.

### Manual playtest checklist

Nothing here can be checked without a window. Run `cargo run`:

1. **Platform tops** (first room, warm palette): each platform has a tan lit top with a gently
   jagged lip, green moss tufts, a dark outline down the sides and under the bottom, short
   stalactites hanging below, and speckles and cracks inside. Walk along the tops and jump
   on and off: feet rest on the flat top line, the lip and moss poke up behind the player, and
   nothing looks sunk or floating.
2. **Wall sides**: walk and dash into a wall. The player stops at the wall's drawn edge (the
   outline is inside the collision rectangle), and the outline runs down both sides.
3. **Stacked and touching solids**: in the editor (F2, Terrain tool) draw two solids stacked,
   two side by side touching, and two overlapping; press Test room (F6). No outline or lit
   strip shows between them, the gradient reads as one block, and no edge is drawn twice. A
   solid standing on the floor has no bottom outline or stalactites at the floor line.
4. **Room load**: leave and re-enter a room, and press F5. The look is identical every time,
   with no reshuffling or flicker. Move one solid in the editor and save: only that solid's
   speckles and moss change.
5. **Window resize**: resize and maximize. The world letterboxes as before, the terrain scales
   with it, and no seams or shimmering lines show between bands or shapes. In a room larger
   than the screen, walk to each edge: nothing pops in or out at the screen border.
6. **Collision feels identical**: jump height, landing, dash distance and stopping at walls and
   ceilings are the same as on `main` (collision code is untouched).
7. **Editor view**: with F2 open, solids are still plain rectangles with their handles.
8. **Palette**: in `assets/rooms/first.ron` set `moss: false` or change `seed_offset`, press
   F5 and see the look change; set a color to `1.5` and F5 shows a load error naming the file
   and the color. A room with solids and no palette shows the default dark blue-grey look.
9. **Boundary**: in the first room look at the bands above and below the playable area: a
   ceiling slab with stalactites hanging into the room and a floor slab whose top has the lit
   strip, lip and moss, all in the room's palette. In the second room (800x360, smaller than the
   view) the frame shows on all four sides, with a gap in the left and right walls at each
   exit; the teal exit zones sit in those gaps and walking through them still changes room.
10. **Boundary and platforms**: a platform standing on the floor has no outline along its foot,
    and a platform touching a wall has no outline along the join.
11. **Speed**: no visible slowdown with several large solids and the boundary on screen.
