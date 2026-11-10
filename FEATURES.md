# Features

One line per feature: what; module(s); test file(s). Keep in sync with AGENTS.md.

## Player and world

- Walking, gravity, floor and wall collision; `src/player.rs`; `tests/gameplay.rs`
- Room terrain: solid platforms, walls, ceilings; grounded on solid tops, dash stops at walls, no tunneling; enemy movement (lunge, jump, chase, knockback) is swept against solids; optional `terrain` in room layout RON; `src/terrain.rs`, `src/player.rs`, `src/enemy.rs`, `src/editor.rs`; `tests/terrain.rs`
- Fixed 120 Hz simulation step with capped catch-up; edge input reaches only the first tick of a frame, held input every tick; jump apex identical at 30/60/144 fps; `src/timestep.rs`, `src/main.rs`; `tests/timestep.rs`
- Follow camera: smoothing, horizontal look-ahead by facing, clamped to room bounds (centered when the room is smaller than the view), snap on room change/teleport; tunables in `data/camera.ron`; `src/camera.rs`, `src/main.rs`; `src/camera.rs` unit tests, manual
- Virtual 960x540 render target with letterboxed window scaling; HUD, inventory and reload panels anchored to the virtual screen; `src/camera.rs`, `src/hud.rs`, `src/main.rs`; `src/camera.rs` unit tests, manual
- Variable jump height, coyote time, jump buffering; `src/player.rs`; `tests/jumping.rs`
- Directional melee swing, cooldown, one hit per enemy per swing; `src/combat.rs`, `src/game.rs`; `tests/combat.rs`
- Collectible Dash (facing, duration, cooldown, one air dash); `src/item.rs`, `src/player.rs`; `tests/items.rs`
- Player health, invincibility, respawn; `src/health.rs`; `tests/enemies.rs`
- Grounded item pickup, persistent session inventory; `src/item.rs`; `tests/items.rs`, `tests/rooms.rs`
- Coin pickups, persistent session balance; `src/coin.rs`; `src/coin.rs` unit tests
- Inventory screen with paused gameplay; `src/game.rs`, `src/main.rs`; `src/game.rs` unit tests, manual
- Three rooms, directional exits, spawns, persistent contents; `src/room.rs`; `tests/rooms.rs`
- Keyboard controls, world rendering, melee visual, health bars; `src/main.rs`; manual
- F5 atomic reload of rooms and definitions with scrollable errors; `src/game.rs`, `src/room.rs`; `tests/reloading.rs`

## Enemies and attacks

- Enemy patrol, chase, contact damage, health, knockback, death; `src/enemy.rs`; `tests/enemies.rs`
- RON enemy definitions and room placements, sequential actions, conditional transitions, asset validation; `src/behavior.rs`; `tests/behavior.rs`, `tests/definitions.rs`
- Shared lunge, jump, slam, projectile fan, radial blast, warning and recovery actions; `src/enemy_attack.rs`, `src/behavior.rs`; `tests/attack_actions.rs`
- Lunge samples player direction once, suspends gravity, stops at walls, keeps authored trajectories; `src/enemy_attack.rs`; `tests/attack_actions.rs`
- Rect/circle attack boxes, immunity-gated damage and knockback; `src/enemy_attack.rs`, `src/player.rs`; `tests/attack_actions.rs`
- Jump-to-player, hover, slam, random area blasts with arena clamping and editor parameters; `src/behavior.rs`, `src/behavior_values.rs`; `tests/arena_actions.rs`
- Warning tint strength (0.25) and 30% AttackBox outline; `src/enemy_attack.rs`, `src/body_draw.rs`; `tests/attack_actions.rs`
- Compile-time guard keeping hitbox/attack motion independent of body state; `src/enemy_attack.rs`; `tests/pose_geometry.rs`

## Bosses

- Ash Warden timed attacks, melee damage, persistent death, locked arena exits; `src/boss.rs`; `tests/boss.rs`
- Arena boss cycling Slam, Summon, Rings with summons, death coins, drawing; `src/boss_special.rs`; `tests/boss_special.rs`
- RON boss definitions and placements, cycle/seeded random selection, ordered actions; `src/boss_definition.rs`, `src/boss_special.rs`; `tests/boss_data.rs`
- Boss editor: placement, drag, delete, ordered attacks/actions, staged saves with renames; `src/boss_editor.rs`, `src/editor_panel/boss.rs`; `tests/boss_editor.rs`

## Bodies and poses

- RON humanoid/bird/animal body templates, palettes, scale, point maps, mirroring, box fallback; `src/body.rs`, `src/body_draw.rs`; `tests/bodies.rs`
- Damped point following, idle motion, per-point springs, F5 motion preservation; `src/body.rs`, `data/body_motion_defaults.ron`; `tests/body_motion.rs`
- Charger/Hopper/Shooter use distinct bodies, palettes, scales; `assets/entities/`; `tests/shipped_rooms.rs`, `tests/bodies.rs`
- Semantic pose assets and per-template point mapping; `src/pose.rs`; `tests/poses.rs`
- Attack phase pose commands with spring target cleanup; `src/behavior.rs`, `src/enemy_attack.rs`; `tests/poses.rs`
- Telegraph-derived critically damped pose springs, blend ratio, strike stiffness, shot recoil; `src/pose.rs`, `data/pose_defaults.ron`; `tests/poses.rs`
- Eight authored poses, attack bindings, timed hurt pose and white flash; `assets/poses/`, `src/body.rs`; `tests/poses.rs`
- Pose catalog/binding validation shared by load and staged saves; F5 reloads poses; `src/pose.rs`, `src/definition_editor.rs`; `tests/poses.rs`, `tests/reloading.rs`
- Fixture-only pose regression suite (semantics, lifecycle, facing, hurt timing, hot reload); `tests/poses.rs`, `tests/fixtures/`
- F3 body debug overlay (dots, lines, hitboxes, nearest-point labels, private font atlas); `src/body_debug.rs`, `src/body_draw.rs`; `tests/body_debug.rs`, manual
- F3 legend shows nearest body's current pose; `src/body_debug.rs`; `tests/body_debug.rs`, manual

## Room and definition editor

- F2 paused room editor: placement, move, delete, entity list, spawn/exit editing, atomic RON saves, custom rooms; `src/editor.rs`; `tests/editor.rs`
- Entity/behavior side panel, state/link editing, shared-name updates, staged RON saves; `src/definition_editor.rs`; `tests/definition_editor.rs`
- Responsive scrollable panel, pinned Save, ordered action/transition rows, type catalogs, advanced RON editing; `src/editor_panel.rs`; `tests/definition_editor.rs`
- Save conflict detection, full validation, LF writes, recoverable staging, repeat renames; `src/definition_editor.rs`; `tests/definition_editor.rs`
- Searchable lists with stable selection, Unicode prefix-first matching, counts, management buttons; `src/search.rs`; `tests/definition_editor.rs`
- Patrol handles, exit/arrival deletion, item catalog, no-exit warnings, transactional saves, Windows-safe names; `src/editor.rs`, `src/item.rs`; `tests/editor.rs`
- egui panel: collapsing sections, filtered dropdowns, tooltip truncation, solid scrollbars, canvas input capture; `src/editor_panel.rs`; headless tests, manual
- Editor canvas view: wheel zoom about the cursor, middle-drag/Space+drag pan, Home/Reset view, clamped zoom, grid overlay with snap size (default 8 px, snaps place/drag), footer sized from the window; `src/editor_view.rs`, `src/main.rs`, `src/editor_panel/canvas_tools.rs`; `tests/editor_view.rs`, headless panel tests, manual
- Room size section (x, y, width, height, floor_y): changes that would leave the spawn, items, exits, terrain, enemies or boss outside are refused; `src/room_size.rs`, `src/editor_panel/canvas_tools.rs`; `tests/room_size.rs`
- Unsaved-changes guard: closing the editor (F2), Open room and New room ask Save / Discard / Cancel when the session is dirty and act at once when clean; a failed save keeps the dialog open with the error; `src/editor_guard.rs`, `src/editor_panel/unsaved_dialog.rs`; `tests/editor_guard.rs`, headless dialog tests, manual
- Behavior presets (Patrol, Chaser, Charger, Skittish, Stationary), searchable creation, usage warnings, Key values; `src/preset.rs`, `src/behavior_values.rs`; `tests/presets.rs`
- Blank-enemy creation and behavior assignment, editable Skittish distances/speed; `src/definition_editor.rs`, `src/behavior.rs`; `tests/presets.rs`
- Catalog-backed pose dropdowns for attack phases in rows and Key values; `src/editor_panel.rs`, `src/behavior_values.rs`; `tests/poses.rs`
- Body, pose, spring, hurt, blend and tint values all have data, validation, editor controls; `src/body.rs`, `src/pose.rs`, `data/`; `tests/poses.rs`

## Tests and CI

- Fixture-based behavioral tests plus one shipped-content smoke test; `tests/fixtures/`, `tests/support/mod.rs`; `tests/shipped_rooms.rs`
- CI: fmt, test, Clippy on Linux and Windows for PRs and `main`; `.github/workflows/ci.yml`
