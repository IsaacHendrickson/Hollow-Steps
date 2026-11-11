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
