## Enemy body templates

Presentation is separate from collision and simulation. `assets/bodies/<name>.ron`
contains one strict body template, selected by the filename stem. Each template
has a `points` map (`"head": (rest: (x, y))`) and ordered `parts`. Positions and
sizes are fractions of the enemy's unchanged collision box, centered at `(0, 0)`,
with x right and y down. Enemy instances own unmirrored current positions,
velocities, and targets; drawing resolves positions and mirrors only the result.

Templates also accept motion data (omitted fields use the defaults in
`data/body_motion_defaults.ron`):

```ron
follow: (stiffness: 120.0, damping: 22.0),
follow_overrides: {"tail": (stiffness: 80.0, damping: 10.0)},
idle: [
    (point: "tail", amplitude: (0.015, 0.02), period_secs: 1.8, phase_offset_secs: 0.0),
],
```

The target defaults to rest plus the sum of idle offsets, each
`amplitude * sin(2*pi*(game_time + phase_offset_secs)/period_secs)`. Amplitudes
are local box fractions. `BodyInstance::set_target(name, position)` overrides
that target; `clear_target(name)` resumes rest + idle; `snap_to_targets()` places
all points at their targets and zeros velocities. Spawn snaps immediately,
including nonzero idle phases. No action poses are assigned.

Each step uses semi-implicit Euler: update velocity by
`(stiffness * (target - position) - damping * velocity) * dt`, then position by
the new velocity times `dt`. Only this spring delta is capped at 1/30 second;
idle time advances with the supplied game delta. Inventory, the editor, and
inactive rooms freeze motion. The 120/22 defaults settle without overshoot.
Lower damping gives tails and wings more overshoot. Stiffness must be finite and positive, damping
finite and nonnegative, and idle periods finite and positive. Amplitudes/phases
must be finite; idle and override names must exist in the point map.

Each part has `name`, `shape` (`Ellipse`, `Triangle`, `Rect`, or `Line`), `size:
(w, h)`, `rotation` in clockwise degrees, and `color_slot` (`primary`, `secondary`,
`accent`, or `eye`). Non-Line parts use `anchor: Some("head")` and `offset:
Some((x, y))`; Line parts use `from: Some("torso")` and `to: Some("hand_l")`
instead. Missing optional fields are omitted. Ellipses/rectangles are centered;
a triangle points right before rotation. Offsets use the box axes; rotation
turns the shape about its resolved center. A Line connects its two current
points, with thickness `min(w * box_width, h * box_height)`; its rotation turns
both endpoints about their midpoint (use zero to connect the points exactly).
Horizontal facing mirrors the complete result about the enemy center.

An enemy definition may include:

```ron
body: Some((
    template: "humanoid",
    primary: (0.55, 0.19, 0.30, 1.0),
    secondary: (0.27, 0.10, 0.23, 1.0),
    accent: (0.95, 0.57, 0.23, 1.0),
    eye: (1.0, 1.0, 0.75, 1.0),
    scale: 1.0,
)),
```

All four colors are normalized RGBA channels in `0..=1`. Scale defaults to one
and must be finite and positive; it scales points, offsets, and shapes about
the entity center. Omitting `body` or using `None` keeps the original maroon box.
The editor's **Body visual** section selects a template or Box and edits scale
and all four colors. New assignments copy `data/body_visual_defaults.ron`;
existing palettes are preserved when switching templates. The same section edits
shared template stiffness/damping, point overrides, and idle point/amplitude/
period/phase entries. Template geometry remains authored in RON. Loading and staged saves validate template names and
point references; unknown color slots are parse errors. F5 reloads templates
and enemy settings together, retaining the current game if validation fails.
Matching enemy placements retain surviving point positions, velocities, explicit
targets, and idle phase; new points snap to their targets and removed points disappear.
Reload matches name and patrol interval first, then moved placements by name and
occurrence, since placements have no persistent IDs.
Editor saves also reject body files changed since the editor opened.

Manual check: run the game, confirm chase (humanoid) and scittish (bird) have
visible bright eyes, watch the bird/beak mirror when it turns, and confirm an
enemy without a body still draws as a box. Edit a template point/part and an enemy palette/scale,
press F5, and confirm both refresh. In F2, edit an enemy's health and palette,
save, reopen, and confirm its body is retained. Hitboxes and health bars stay at
the original box size even if visual scale changes.
Watch breathing and opposite-phase wing motion; assign an animal body to see its
tail swish. Open inventory/F2 and confirm motion freezes, then resumes when closed.
Turn an enemy and confirm points flip immediately without sliding. Change an idle
amplitude or rest position and press F5: current points stay in place and ease to
the new targets. In Body visual, edit motion fields, save, reopen, and confirm
the values persist; an invalid period or spring value must block Save.

