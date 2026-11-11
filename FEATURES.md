# Features

One line per feature: what; module(s); test file(s). Keep in sync with AGENTS.md.

## Player and world

- Walking, gravity, floor and wall collision; `src/player.rs`; `tests/gameplay.rs`
- Room terrain: solid platforms, walls, ceilings; grounded on solid tops, dash stops at walls, no tunneling; enemy movement (lunge, jump, chase, knockback) is swept against solids; optional `terrain` in room layout RON; `src/terrain.rs`, `src/player.rs`, `src/enemy.rs`, `src/editor.rs`; `tests/terrain.rs`
- Fixed 120 Hz simulation step with capped catch-up; edge input reaches only the first tick of a frame, held input every tick; jump apex identical at 30/60/144 fps; `src/timestep.rs`, `src/main.rs`; `tests/timestep.rs`
- Follow camera: smoothing, horizontal look-ahead by facing, clamped to room bounds (centered when the room is smaller than the view), snap on room change/teleport; tunables in `data/camera.ron`; `src/camera.rs`, `src/main.rs`; `src/camera.rs` unit tests, manual
- Virtual 960x540 render target with letterboxed window scaling; HUD, inventory and reload panels anchored to the virtual screen; `src/camera.rs`, `src/hud.rs`, `src/main.rs`; `src/camera.rs` unit tests, manual
- HUD in virtual-resolution space: health masks (full/empty) from `MAX_HEALTH`, coin counter and item count top left, room label and exit note top right, small controls hint bottom left, boss bar and caption showing the boss definition's own name (no hardcoded boss name); F5 reload messages kept; `src/hud.rs`, `src/hud_layout.rs`, `tests/hud_layout.rs`, `tests/boss_data.rs` (boss name), manual
- Variable jump height, coyote time, jump buffering; `src/player.rs`; `tests/jumping.rs`
- Player and enemy-knockback tunables (move/jump/gravity/dash/size, strike hitboxes and pogo speed, default enemy health, knockback) in `data/player.ron`, strict and fully validated; an `assets/player.ron` replaces it whole and a bad one is an error, never a fallback; F5 re-reads it without resetting the player; `src/tuning.rs`, `src/tuning_fields.rs`, `src/player.rs`, `src/enemy.rs`, `src/game.rs`; `tests/tuning.rs`, `src/tuning.rs` unit tests
- F4 live tuning window: a drag box with unit and range for every `player.ron` field (move, jump, gravity, dash, size, strike, default enemy health, knockback); changes apply on the next tick, typed out-of-range or non-numeric values are refused with a message, Save writes `assets/player.ron` through a staged write, unsaved note, list scrolls in a 540 px window; `src/tuning_panel.rs`, `src/tuning.rs`, `src/game.rs`, `src/main.rs`; headless panel tests, `tests/tuning.rs`, manual
- Directional melee swing, cooldown, one hit per enemy per swing; `src/combat.rs`, `src/game.rs`; `tests/combat.rs`
- Up and down nail strikes with pogo: X with Up held strikes up (Up with X is not a pickup), X with Down held in the air strikes down (forward on the floor), Up wins over Down; one hitbox size per direction and `pogo_speed` in the `strike` section of `data/player.ron` and the F4 window; a downward hit on an enemy or a live boss bounces the player up once per swing, never on the floor, and restores the air dash; up/down hits push enemies sideways away from the player; `src/combat.rs`, `src/player.rs`, `src/game.rs`, `src/tuning.rs`, `src/tuning_fields.rs`, `src/main.rs`; `tests/nail_strikes.rs`, `src/player/strike_tests.rs`, `src/game/strike_tests.rs`, manual
- Gameplay event queue: `GameEvent` (PlayerJumped, PlayerLanded, PlayerDashed, PlayerAttacked, EnemyHit, EnemyDied, PlayerDamaged, CoinCollected, ItemPickedUp, RoomEntered, BossDefeated) raised once at the moment each happens, in order, drained with `GameState::take_events`; a landing is reported only after a real fall; boss hits raise EnemyHit and the kill BossDefeated; nothing is raised while paused; `src/events.rs`, `src/player.rs`, `src/game.rs`; `src/events.rs`, `src/player/event_tests.rs`, `src/game/event_tests.rs` unit tests
- Hit-stop and screen shake: a swing that hits an enemy or the boss freezes enemies and the boss for `enemy_hit_secs`, losing health freezes them for `player_damaged_secs` (the player, camera and shake keep running; one freeze at a time, never summed); a sine shake that fades linearly shifts the world view (`Camera::view_rect_shaken`) without moving the camera; nothing ages while the inventory or editor is open; values in `data/feel.ron`, validated, with an optional whole-file `assets/feel.ron` override that F5 re-reads; `src/feel.rs`, `src/camera.rs`, `src/game.rs`, `src/main.rs`; `src/feel.rs`, `src/game/feel_tests.rs`, `src/camera.rs` unit tests, manual
- Located events: `GameState::take_located_events` returns each `GameEvent` with the world position it happened at (end of its tick): the player's feet for jump, landing, dash and swing, the player's center for damage, the enemy's or boss's center for hit, kill and defeat, none for coin, item and room events; `take_events` is unchanged; `src/events.rs`, `src/game.rs`; `src/events.rs`, `src/game/located_event_tests.rs` unit tests
- Particles: `hit_spark` (enemy hit, player damaged), `dust` (jump, landing), `dash_trail` (dash) and `death_burst` (enemy died, boss defeated) bursts spawned from located events and drawn in world space through the camera (so shake moves them), stepped by the fixed tick and frozen while the inventory is open; seeded deterministic generator (fixed seed, no clock), validated presets whose trigger events, counts, lifetimes, speeds, directions, spread, gravity, drag, size and colors are data in `data/particles.ron`, a live-particle cap, cleared on room change and test room; optional whole-file `assets/particles.ron` override that F5 re-reads (a bad file shows its error and keeps the defaults); `src/particles.rs`, `src/effects.rs`, `src/assets.rs`, `src/main.rs`; `src/particles/tests.rs`, `tests/effects.rs`, `src/game/located_event_tests.rs`, manual
- Sound hooks: `data/audio.ron` maps each `GameEvent` to a sound name and volume (validated: lowercase names that cannot leave the folder, volumes 0 to 1, one cue per event; an event with no cue is silent); the front end plays `assets/audio/<name>.wav` or `.ogg` if the file exists and plays nothing if it does not, so the game runs with no sound files; a file that is not WAV or Ogg, or that the backend cannot decode, is skipped with a stderr line instead of stopping the game; optional whole-file `assets/audio.ron` override and the sound files are re-read by F5; no sound files ship (`assets/audio/README.md` lists the names); sound plays only in a build with the opt-in `audio` cargo feature (`cargo run --features audio`; Linux needs `libasound2-dev`), otherwise one line is printed if files exist; `src/sound.rs`, `src/effects.rs`, `src/effects/sound_bank.rs`, `Cargo.toml`; `src/sound/tests.rs`, `tests/effects.rs`, manual (decision in `docs/audio.md`)
- Collectible Dash (facing, duration, cooldown, one air dash); `src/item.rs`, `src/player.rs`; `tests/items.rs`
- Player health, invincibility, respawn; `src/health.rs`; `tests/enemies.rs`
- Grounded item pickup, persistent session inventory; `src/item.rs`; `tests/items.rs`, `tests/rooms.rs`
- Coin pickups, persistent session balance; `src/coin.rs`; `src/coin.rs` unit tests
- Inventory screen with paused gameplay; `src/game.rs`, `src/main.rs`; `src/game.rs` unit tests, manual
- Three rooms, directional exits, spawns, persistent contents; `src/room.rs`; `tests/rooms.rs`
- Keyboard controls, world rendering, melee visual, health bars; `src/main.rs`; manual
- F5 atomic reload of rooms and definitions with scrollable errors; `src/game.rs`, `src/room.rs`; `tests/reloading.rs`
- Assets folder from `HOLLOW_STEPS_ASSETS`, then next to the executable, then the source tree; `src/assets.rs`; `src/assets.rs` unit tests
- Startup load errors shown on screen (F5 retries) instead of a panic, with the folder tried named; `src/game.rs`, `src/startup.rs`, `src/assets.rs`; `tests/startup.rs`, manual
- Current room kept as a stored index, recomputed when a save rebuilds the room list; `src/game.rs`; `tests/startup.rs`

## Enemies and attacks

- Enemy patrol, chase, contact damage, health, knockback, death; `src/enemy.rs`; `tests/enemies.rs`
- Enemy dying state: at zero health an enemy stays for `death_secs` (entity definition, default 0.4 s, 0 to 5 s, 0 removes it at once as before), flashing from the killing blow and fading out; while dying it deals no contact, blast or attack damage, takes no further hits, is no pogo target and runs no behavior; `EnemyDied` is raised once, at the killing blow, and removal raises nothing; hit-stop holds the fade; an optional `death_secs` in the entity RON and a Death time drag box in the editor's Entity section (kept across ordinary entity edits, clamped to the valid range); `src/enemy.rs`, `src/enemy/death.rs`, `src/enemy/definition.rs`, `src/game.rs`, `src/body_draw.rs`, `src/main.rs`, `src/definition_editor.rs`, `src/editor_panel/death_field.rs`; `tests/enemy_death.rs`, `src/game/death_tests.rs`, `src/enemy/death.rs` and `src/enemy/definition.rs` unit tests, `src/editor_panel/death_field.rs` panel tests, manual
- RON enemy definitions and room placements, sequential actions, conditional transitions, asset validation; `src/behavior.rs`, `src/behavior/`; `tests/behavior.rs`, `tests/definitions.rs`
- Shared lunge, jump, slam, projectile fan, radial blast, warning and recovery actions; `src/enemy_attack.rs`, `src/behavior/`, `src/action_runtime/attacks.rs`; `tests/attack_actions.rs`
- Lunge samples player direction once, suspends gravity, stops at walls, keeps authored trajectories; `src/enemy_attack.rs`; `tests/attack_actions.rs`
- Rect/circle attack boxes, immunity-gated damage and knockback; `src/enemy_attack.rs`, `src/player.rs`; `tests/attack_actions.rs`
- One hit per tick: strongest of enemy contact/blast/attack and boss action/contact damage; an attack hit keeps its knockback only when it is the strongest; `src/damage.rs`, `src/game.rs`; `src/damage.rs` unit tests, `src/game.rs` blast and attack tests
- Jump-to-player, hover, slam, random area blasts with arena clamping and editor parameters; `src/behavior/`, `src/action_runtime/movement.rs`, `src/action_runtime/blasts.rs`, `src/behavior_values.rs`; `tests/arena_actions.rs`
- Warning tint strength (0.25) and 30% AttackBox outline; `src/enemy_attack.rs`, `src/body_draw.rs`; `tests/attack_actions.rs`
- Compile-time guard keeping hitbox/attack motion independent of body state; `src/enemy_attack.rs`; `tests/pose_geometry.rs`

## Bosses

- Arena exits stay locked while the boss lives and open on defeat, which persists through respawn and room travel; `src/room.rs`, `src/boss_special.rs`; `tests/boss_special.rs`, `src/game.rs` unit tests
- Arena boss cycling Slam, Summon, Rings with summons, death coins, drawing; `src/boss_special.rs`; `tests/boss_special.rs`
- RON boss definitions and placements, cycle/seeded random selection, ordered actions; `src/boss_definition.rs`, `src/boss_special.rs`; `tests/boss_data.rs`
- Boss editor: placement, drag, delete, ordered attacks/actions, staged saves with renames; `src/boss_editor.rs`, `src/editor_panel/boss.rs`; `tests/boss_editor.rs`

## Bodies and poses

- RON humanoid/bird/animal body templates, palettes, scale, point maps, mirroring, box fallback; `src/body.rs`, `src/body_draw.rs`; `tests/bodies.rs`
- Damped point following, idle motion, per-point springs, F5 motion preservation; `src/body.rs`, `data/body_motion_defaults.ron`; `tests/body_motion.rs`
- Charger/Hopper/Shooter use distinct bodies, palettes, scales; `assets/entities/`; `tests/shipped_rooms.rs`, `tests/bodies.rs`
- Semantic pose assets and per-template point mapping; `src/pose.rs`; `tests/poses.rs`
- Attack phase pose commands with spring target cleanup; `src/behavior/action_info.rs`, `src/action_runtime/poses.rs`, `src/enemy_attack.rs`; `tests/poses.rs`
- Telegraph-derived critically damped pose springs, blend ratio, strike stiffness, shot recoil; `src/pose.rs`, `data/pose_defaults.ron`; `tests/poses.rs`
- Eight authored poses, attack bindings, timed hurt pose and white flash; `assets/poses/`, `src/body.rs`; `tests/poses.rs`
- Player body: a hooded wanderer (`assets/bodies/wanderer.ron`, original design) with idle, run, jump and attack poses (`assets/poses/wanderer_*.ron`) chosen by grounded/speed/swing state; look and run threshold in `data/player_body.ron` (an `assets/player_body.ron` replaces it whole), no template keeps the blue box, the hitbox is unchanged; `draw_body` shared with enemies; `src/player_body.rs`, `src/body_draw.rs`, `src/main.rs`; `tests/player_body.rs`, `tests/shipped_rooms.rs`, manual
- Pose catalog/binding validation shared by load and staged saves; F5 reloads poses; `src/pose.rs`, `src/definition_editor.rs`; `tests/poses.rs`, `tests/reloading.rs`
- Fixture-only pose regression suite (semantics, lifecycle, facing, hurt timing, hot reload); `tests/poses.rs`, `tests/fixtures/`
- F3 body debug overlay (dots, lines, hitboxes, nearest-point labels, private font atlas); `src/body_debug.rs`, `src/body_draw.rs`; `tests/body_debug.rs`, manual
- F3 legend shows nearest body's current pose; `src/body_debug.rs`; `tests/body_debug.rs`, manual

## Room and definition editor

- F2 paused room editor: placement, move, delete, entity list, spawn/exit editing, atomic RON saves, custom rooms; `src/editor.rs`; `tests/editor.rs`
- Entity/behavior side panel, state/link editing, shared-name updates, staged RON saves; `src/definition_editor.rs`; `tests/definition_editor.rs`
- Responsive scrollable panel, pinned Save, ordered action/transition rows, type catalogs, advanced RON editing; `src/editor_panel.rs`, `src/editor_panel/entity.rs`, `src/editor_panel/states.rs`; `tests/definition_editor.rs`
- Save conflict detection, full validation, LF writes, recoverable staging, repeat renames; `src/definition_editor.rs`; `tests/definition_editor.rs`
- Searchable lists with stable selection, Unicode prefix-first matching, counts, management buttons; `src/search.rs`; `tests/definition_editor.rs`
- Patrol handles, exit/arrival deletion, item catalog, no-exit warnings, transactional saves, Windows-safe names; `src/editor.rs`, `src/item.rs`; `tests/editor.rs`
- egui panel: collapsing sections, filtered dropdowns, tooltip truncation, solid scrollbars, canvas input capture; `src/editor_panel.rs`, `src/editor_panel/widgets.rs`; headless tests (`src/editor_panel/panel_tests/`), manual
- Editor canvas view: wheel zoom about the cursor, middle-drag/Space+drag pan, Home/Reset view, clamped zoom, grid overlay with snap size (default 8 px, snaps place/drag), footer sized from the window; `src/editor_view.rs`, `src/main.rs`, `src/editor_panel/canvas_tools.rs`; `tests/editor_view.rs`, headless panel tests, manual
- Room size section (x, y, width, height, floor_y): changes that would leave the spawn, items, exits, terrain, enemies or boss outside are refused; `src/room_size.rs`, `src/editor_panel/canvas_tools.rs`; `tests/room_size.rs`
- Unsaved-changes guard: closing the editor (F2), Open room and New room ask Save / Discard / Cancel when the session is dirty and act at once when clean; a failed save keeps the dialog open with the error; `src/editor_guard.rs`, `src/editor_panel/unsaved_dialog.rs`; `tests/editor_guard.rs`, headless dialog tests, manual
- Room undo/redo: Undo and Redo buttons and Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z (canvas focused, no text box focused) over place, move, resize, delete, terrain, exit and room-size changes; a drag is one step, 100 steps kept, a new edit clears redo, opening or creating a room clears history; entity, behavior and boss definition edits are not covered (stated in the buttons' tooltip); `src/editor_history.rs`, `src/editor_panel/history_panel.rs`; `tests/editor_history.rs`, headless panel tests, manual
- Test room: the Test room button and F6 play the unsaved draft (boss included) from its spawn with the editor parked; exits are inert, F5 and Open room wait; F2 returns to the same session with its draft, dirty flag, message and undo history; the built room stands in for the saved one and the player and counters are restored, nothing is written; an invalid draft stays in the editor with the reason in its message; `src/game/test_room.rs`, `src/game.rs`, `src/hud.rs`, `src/main.rs`, `src/editor_panel.rs`; `tests/test_room.rs`, headless panel test, manual
- Editor tools: a Tools section (Select, Place, Terrain, Erase; keys V, P, T, E) replaces the Place and Select / drag buttons; Terrain drags out a grid-snapped solid, Select moves and resizes solids by eight handles, Erase and right-click and Delete remove solids or markers; every edit is validated and refused over the spawn, items, exits or outside the room; solids save to `layout.terrain` and undo/redo covers them (a drag is one step); `src/editor_terrain.rs`, `src/editor_tools.rs`, `src/terrain_canvas.rs`, `src/editor_panel/tool_panel.rs`; `tests/editor_terrain.rs`, headless panel tests, manual
- Behavior presets (Patrol, Chaser, Charger, Skittish, Stationary), searchable creation, usage warnings, Key values; `src/preset.rs`, `src/behavior_values.rs`; `tests/presets.rs`
- Blank-enemy creation and behavior assignment, editable Skittish distances/speed; `src/definition_editor.rs`, `src/behavior/`; `tests/presets.rs`
- Catalog-backed pose dropdowns for attack phases in rows and Key values; `src/editor_panel/widgets.rs`, `src/editor_panel/states.rs`, `src/editor_panel/key_values.rs`, `src/behavior_values.rs`; `tests/poses.rs`
- Body, pose, spring, hurt, blend and tint values all have data, validation, editor controls; `src/body.rs`, `src/pose.rs`, `data/`; `tests/poses.rs`

## Tests and CI

- Fixture-based behavioral tests plus one shipped-content smoke test; `tests/fixtures/`, `tests/support/mod.rs`; `tests/shipped_rooms.rs`
- CI: fmt, test, Clippy on Linux and Windows for PRs and `main`; `.github/workflows/ci.yml`
