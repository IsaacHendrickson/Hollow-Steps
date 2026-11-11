## Boss architecture

Boss definitions are strict RON files in `assets/bosses/`. Rooms reference the
definition's `name`, just like enemy placements:

```ron
(enemies: [], boss: Some((definition: "Ash Warden", position: (560.0, 436.0))))
```

Omitting `boss` leaves the room without a boss. Positions are body centers and
must fit within the room. A definition contains all encounter settings:

```ron
(
    name: "Ash Warden", health: 1000, size: (48.0, 48.0), contact_damage: 0,
    selection: cycle, pause_seconds: 3.0,
    attacks: [
        (name: "Slam", actions: [
            glide_to_player(speed: 1000.0, above_floor: 180.0),
            telegraph(seconds: 0.5),
            glide_to_floor(speed: 1000.0),
            damage_target(radius: 56.0, damage: 10),
        ]),
    ],
)
```

`size` is the boss's hitbox (contact damage, being hit). An optional `visual_size: Some((w, h))` (each
side 1 to 2000 px, left out of saved files when unset) is the size of the box it is drawn in; the two
boxes share a horizontal center and a bottom edge, so a taller picture rises above the hitbox.

`selection` is `cycle` (file order, wrapping) or `random` (uniform choices,
repeats allowed, with an instance-local seed for reproducible tests). The pause
is the minimum interval between attack starts, including attack time; an
unfinished sequence delays the next start. The first attack starts after that
interval. Selection and timers belong to each boss, so inactive rooms pause.

Attacks have unique nonempty names and nonempty ordered action lists using the
shared action library. `wait(seconds)` and `telegraph(seconds)` consume time;
instant actions run in order at a boundary, and unused time passes to the next
action. Boss action parameters (pixels, seconds, pixels/second) are:

| Action | Parameters and completion |
| --- | --- |
| `glide_to_player` | `speed`, `above_floor`: set a nonblocking glide to the captured player x and floor minus height. Height 0 preserves the current y. |
| `glide_to_floor` | `speed`: set a nonblocking glide to the captured x and floor minus half the boss height. |
| `damage_target` | `radius`, `damage`: immediately damage a player within the captured x radius, even if still gliding. |
| `summon` | `inset`: spawn two stationary patrollers this far from the arena's sides. |
| `spawn_rings` | `count`: place this many random arena rings immediately. |
| `explode_rings` | `radius`, `damage`: explode pending rings and damage a player inside any ring. |
| `jump_to` (alias of shared `jump_to_player`) | `dx`, `dy`, `speed`: glide to the captured player position plus offsets, clamped to the arena; complete on arrival. Use `dy: -200.0` for 200 pixels above the player. |
| `hover` | `seconds`: hold position for the duration. |
| `slam_down` | `speed`: descend to the floor; complete on landing. |
| `spawn_shockwave` | `radius`, `damage`: immediately damage a player within the horizontal radius of the boss. |
| `area_blast` | Shared `count`, `radius`, `fuse_time`, `damage`: create arena circles, wait for the fuse, then explode them. |

The ground-pound sequence (`jump_to` 200 above the player, `hover` 0.5 seconds,
`slam_down` at speed 100, `spawn_shockwave`) exists only in a test fixture.
Boss-only effects and nonblocking glides are rejected in enemy behaviors and
are not added to the enemy editor's action picker. The shared jump, hover, slam,
and area blast actions keep their existing enemy behavior and controls.
Existing enemy horizontal movement actions are rejected in
boss attacks. Loading rejects unknown/missing fields or action types, duplicate
boss/attack names, zero health, invalid size or placement, missing references,
negative/nonfinite values (signed finite jump offsets are allowed), zero boss
movement speeds, noninteger damage outside
0..65535, and ring counts outside 1..256. Errors identify the file, boss, attack,
action index, and invalid parameter. F5 validates bosses before applying a
reload; existing encounter progress remains intact. Room saves preserve boss
placements; this task adds no boss editor controls.

The playable arena uses `src/boss_special.rs`: a boss (health from its definition) cycles Slam,
Summon, and Rings roughly every three seconds. Move out of the red slam zone
and pending red rings; orange rings have already exploded. Summoned enemies
use normal enemy combat. Each melee swing deals one hit (10) of boss damage, and defeat
unlocks the exit and drops twenty collectible coins after a brief effect.
Inventory and inactive rooms pause the encounter. The game imports each enemy
and coin spawn once into the room's existing lists for combat and collection.

`Room::exits_locked()` derives its answer from the room's `boss_special`
(locked while it exists and is not defeated) instead of storing a second lock
flag, and `exit_at` enforces the lock for every exit. Boss health and defeat
persist through respawn and room travel, and inactive rooms and the inventory
pause freeze the encounter. The earlier fixed-attack `src/boss.rs` (an Ash
Warden with a hardcoded rectangle sweep) was never placed in a real room and
has been removed; `boss_special` is the only boss system.
