## Atmosphere: design note

Two things give a room its mood: a post-process pass over the world picture (vignette and
soft glow) and background layers behind the terrain (parallax).

### Post-process: vignette and glow

`src/postfx.rs` is pure. It holds the settings (`data/postfx.ron`, validated; an
`assets/postfx.ron` replaces it whole and F5 re-reads it), the shader source, and the two
formulas the shader runs (`vignette_factor`, `glow_weight`) so tests can pin what the pass
does to a pixel without a window. `src/postfx_pass.rs` is the Macroquad side.

The world is drawn into the 960x540 render target as before. The pass then draws that
target into a second target of the same size through the material (one fragment shader:
glow, then vignette), and the HUD is drawn on top of the result, so the vignette never dims
the HUD. With `enabled: false`, or both strengths at 0, nothing extra is drawn and the frame
is exactly what it was without the pass. If the shader does not build on a machine, the
reason is shown like a failed reload and the picture goes out unprocessed.

| Setting | Meaning |
|---|---|
| `enabled` | the data toggle; `false` skips the pass |
| `vignette.strength` | 0 = no darkening, 1 = black corners |
| `vignette.radius` | distance from the middle (0) to the corners (1) where it starts |
| `vignette.softness` | width of the fade (0.05 to 2) |
| `glow.strength` | how much halo is added (0 to 2) |
| `glow.threshold` | brightness a pixel needs before it glows (0 to 0.95) |
| `glow.radius` | how far the halo reaches, in pixels of the 960x540 picture (0 to 24) |

The glow takes 24 samples (8 directions on 3 rings) around each pixel, so a large radius
with a high strength shows faint copies of thin bright shapes. Keep the radius small.

### Parallax background layers

`src/background.rs` is pure: the layer types, validation, and `parallax_offset`. The front end
(`src/scene.rs`) draws the layers first, right after clearing the screen, so everything else
(floor, terrain, exits, items, enemies, the player) is in front. They go through the same
world camera, so screen shake moves them with the world.

A room lists its layers in `layout`, back to front (the first is drawn first):

```ron
background: [
    (
        depth: 0.25,                 // 0 = fixed on the screen, 1 = moves with the world
        color: (30, 34, 60, 255),    // red, green, blue, alpha (0 to 255)
        shapes: [
            Rect(x: 90.0, y: 250.0, w: 34.0, h: 290.0),
            Circle(x: 760.0, y: 110.0, radius: 46.0),
            Triangle(a: (-60.0, 540.0), b: (150.0, 270.0), c: (360.0, 540.0)),
        ],
    ),
],
```

- Shapes are placed in room coordinates, as they appear when the camera rests against the
  room's top-left corner. Shapes may reach outside the room.
- As the camera travels right or down, a layer slides left or up by `depth` times that travel.
  The travel is clamped to what the camera can travel in this room, so a layer never slides
  past the room's edges.
- A room that is no wider (or taller) than the 960x540 screen gives the camera no travel on
  that axis, so its layers stay where they were placed. Parallax shows in larger rooms, e.g.
  after setting the width in the editor's Room size section.
- Limits: at most 8 layers per room and 64 shapes per layer, `depth` from 0 to 1, positive
  sizes, coordinates within 100000 pixels. A bad layer is a load error naming the room file
  and the layer ("background layer 1: ...").
- Old room files without `background` load as before. The editor keeps a room's layers when
  it saves, but has no controls for them yet: edit the room file and press F5 (a reload
  re-reads the layers of rooms that are already loaded).
