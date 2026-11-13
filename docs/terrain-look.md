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

`Look` is the resolved look `build_room` draws from (a preset plus the room's own choices, see
below): the palette plus `speckle_density` and `crack_density` (1 is the base rate, 0 draws
none; the 256 and 16 caps still hold), `boundary_thickness` and `draw_boundary`. A thickness that is not a positive finite number
gives no slabs and the room falls back to ground cover, exactly like `draw_boundary: false`.

### Determinism

Every solid is seeded from its room, its own position and the palette's `seed_offset`, and
each feature (speckles, cracks, lip, moss, stalactites) has its own random stream, keyed by
where along the edge it works. So adding, removing or reordering other solids, or switching
moss off, never changes how a solid looks; moving a solid gives it the look of its new spot.
The generator is `particles::Rng`; nothing reads the clock.

The draw list stays within 6 px above a solid, 12 px below it and never past its sides.

### Presets and the room's style

A room's look is a **preset** plus what the room changes about it.

`data/terrain_styles.ron` is the catalog of presets, built into the game. An
`assets/terrain_styles.ron` replaces it whole (the same rule as the other `data/*.ron` files); a
bad file stops the game from loading, and F5 keeps the old presets and names the file. Each
preset is a name and a complete look:

```ron
(
    presets: [
        (
            name: "cave",
            fill: (0.235, 0.165, 0.125),       // each color is (red, green, blue), 0 to 1
            edge: (0.075, 0.045, 0.035),       // outline
            highlight: (0.800, 0.580, 0.340),  // lit top strip and lip
            accent: (0.420, 0.620, 0.260),     // moss and some speckles
            moss: true,
            speckle_density: 1.5,              // 1 is the base rate, 0 to 10
            crack_density: 1.2,
            boundary_thickness: 64.0,          // pixels, more than 0 and at most 512
        ),
        // ...
    ],
)
```

Shipped presets: **stone** (the original dark blue-grey look, and the default), **mossy**,
**cave** (warm brown) and **ruins** (pale masonry, no moss, many cracks). Names must be unique
and not blank, there must be at least one preset, and every number must be finite and in range.

`layout.terrain_style` in a room file is optional, and so is every field of it:

```ron
terrain_style: (
    preset: "cave",          // a name from the catalog; "stone" when left out
    fill: (0.3, 0.2, 0.1),   // optional color overrides: fill, edge, highlight, accent
    moss: false,             // optional: switches the preset's moss on or off
    seed_offset: 7,          // reshuffles this room's speckles and moss
    draw_boundary: true,     // false: no slabs around the room; true when left out
),
```

The look is the preset, then each override the room names on top of it (a color the room names
beats the preset's), plus the room's `seed_offset` and `draw_boundary`. Density and boundary
thickness come from the preset only. A room that says nothing gets `stone` with the boundary
on, which is the look rooms had before presets existed. The default style is not written back
when the editor saves, and a style writes only what differs from it (a room that only picks a
preset saves as `terrain_style: (preset: "cave")`).

Room files from before presets spelled a palette as `fill`, `edge`, `highlight`, `accent`,
`seed_offset` and `moss` (bare values). Those fields are overrides now, over `stone`, whose
numbers are the old defaults, so an old room draws exactly what it drew before. `Some(..)` and
`None` are accepted as well as bare values.

Loading checks every room against the catalog. An unknown preset is an error naming the room
file and listing the presets the game has (`terrain_style: unknown preset "granite"; known
presets: stone, mossy, cave, ruins`); a color that is not finite or not between 0 and 1 is an
error naming the file and the color; unknown fields are errors too. The game never silently
draws another look for a room it could not resolve. The same checks run on F5, where the room's
style and the catalog are swapped in together or not at all.

Code: `src/terrain_styles.rs` (`TerrainPreset`, `TerrainStyles` with `load`, `validate` and
`resolve`, and `RoomStyle`, the type of `layout.terrain_style`); `src/game/terrain_link.rs`
(loads the catalog with the rooms and checks each room against it);
`TerrainStyles::resolve(&room.terrain_style)` gives the `Look`.

### Editing the look in the room editor

The room editor's **Room look** section (Room tools panel, under Room size) edits the open
room's `terrain_style`:

- **Preset**: a dropdown of the presets in `data/terrain_styles.ron` (or the assets override).
  Choosing another preset clears the room's color and moss overrides; the seed offset and the
  boundary switch stay. Choosing the one already chosen does nothing.
- **Fill, Edge, Highlight, Accent**: a color button each (click it for the picker) with a
  **Reset** that goes back to the preset's color; Reset is greyed out while the color is the
  preset's. A color picked equal to the preset's is no override.
- **Moss**: shows what the room will draw (the preset's, or the room's own choice).
- **Draw boundary**: the slabs around the room, a picture only.
- **Seed offset** (a drag box) and **Reroll**, which moves to the next seed and so reshuffles
  the speckles, cracks and moss.

Every control edits the draft (`RoomDraft::set_look_preset`, `set_look_color`,
`reset_look_color`, `set_look_moss`, `set_draw_boundary`, `set_seed_offset`, `reroll_seed` in
`src/editor_look.rs`, checked against the presets), marks the room unsaved, and is covered by
undo and redo. A choice, a checkbox, a typed value or a Reroll is one step; dragging in the
color picker or on the seed box is **one** step from the first change to the release. Opening
the picker is not an edit. Save writes only what differs from the default style, and **Test
room (F6)** plays the unsaved look.

The canvas shows the room with this look: the same draw list as the game
(`editor_look::LookPreview::shapes` calls `TerrainLook::room_shapes` with the same inputs and the
same room id, so the preview, the test room and the played room draw the same picture), drawn
through the canvas zoom and pan by `terrain_draw::draw_shapes_mapped`, and rebuilt only when the
look, the terrain, the bounds or the exits change. The room outline and the grid are drawn over
it; the selected solid gets its white outline and handles, and the Erase tool its red highlight.
In the arrival-spawn view the other room is drawn with its own style. A preset the catalog does
not have (only possible if the presets file changed under the editor) is shown in the panel and
the canvas draws the default look until another preset is chosen.

### Drawing and caching

`TerrainLook` keeps the last draw list and what it was built from (the room, its bounds,
floor, exits and solids and the look, or for `shapes` the solids, ground and palette) and
rebuilds only when one of them changes; `room_shapes` also says whether the boundary is in the
list. `scene.rs` draws it through the world camera and `terrain_draw.rs` skips shapes outside
the view. The list is drawn directly each frame; it is not baked into a texture, because a
texture would have to cover the whole room (rooms may be larger than the screen) and could
blur or seam under the follow camera. The room editor draws the same list through its canvas
transform (see above), with its own cache.

Draw order in `scene.rs`: clear, background layers, the boundary and solids (one list; the old
flat floor strip and room outline only when the boundary is off), exits, items and coins,
enemies, the boss, the player; the HUD is drawn after, in screen space. All of it is in world
space through the camera, with no window-size pixel constants.

### Manual playtest checklist

Nothing here can be checked without a window. Run `cargo run`:

1. **Platform tops** (first room, the `cave` preset): each platform has a tan lit top with a gently
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
7. **Editor view**: with F2 open, the solids and the boundary show the room's look (the same
   picture as in play), the room outline and grid are drawn over it, and a selected solid has
   its white outline and handles.
8. **Presets**: in `assets/rooms/first.ron` change `preset` to `stone`, `mossy` and `ruins` in
   turn, pressing F5 each time: the colors, moss, speckle and crack density and the thickness
   of the boundary change with it. Put `fill: (1.0, 0.0, 0.0)` under it and the fill turns red
   while the rest stays the preset's. Set `preset: "granite"` and F5 shows a load error naming
   the file and the known presets (the game keeps the old look); set a color to `1.5` and F5
   names the color. A room with solids and no style shows the default dark blue-grey look
   with the boundary.
9. **Boundary**: in the first room look at the bands above and below the playable area: a
   ceiling slab with stalactites hanging into the room and a floor slab whose top has the lit
   strip, lip and moss, all in the room's palette. In the second room (800x360, smaller than the
   view) the frame shows on all four sides, with a gap in the left and right walls at each
   exit; the teal exit zones sit in those gaps and walking through them still changes room.
10. **Boundary and platforms**: a platform standing on the floor has no outline along its foot,
    and a platform touching a wall has no outline along the join.
11. **Speed**: no visible slowdown with several large solids and the boundary on screen.
12. **Room edges**: look at all four edges of a room (in the editor, zoomed out, and in play):
    ceiling, left and right walls and floor each show the frame, seams between the slabs have
    no outline, and nothing is drawn over the playable area.
13. **Exits**: every exit still shows its doorway in the frame and the teal exit zone sits in
    the gap; walk through each one in play and in the editor's Test room: it is usable.
14. **Room look in the editor** (F2, Room look section): switch through every preset and watch
    the canvas change at once; drag in a color's picker and watch it follow; Reset puts the
    preset's color back; toggle Moss and Draw boundary; drag the Seed offset box and press
    Reroll. Opening a picker and closing it again changes nothing and does not mark the room
    unsaved.
15. **Undo of the look**: after changing a color (one drag), the preset and the seed, Ctrl+Z
    steps back one change at a time: a whole color drag comes back in one step. Ctrl+Y redoes.
16. **Zoom and pan**: zoom and pan the canvas with the look showing: the picture stays glued to
    the room, the speckles do not crawl and no seams show.
17. **Test room**: press F6: the room has the same look (colors, moss, speckles, boundary) as
    the canvas, including unsaved edits; F2 returns to the editor with the same look.
18. **Save and reload**: save, press F5 or restart: the room keeps its preset, colors, seed and
    boundary choice. A room left at the default look writes no `terrain_style`.
19. **Window resize** in the editor: the canvas refits and the look is not clipped oddly.
20. **Collision identical**: the boundary is only a picture; the player collides with the same
    walls, floor and solids as before.
