# Features

One line per feature (160 characters at most): what; module(s); test file(s). Keep in sync with AGENTS.md. Detail: docs/features-detail.md.

## Player and world

- Walking, gravity, floor and wall collision; `src/player.rs`; `tests/gameplay.rs`
- Room terrain: solid platforms, walls, ceilings; head stops under solids; enemies swept against solids; `src/terrain.rs`; `tests/terrain.rs`
- Terrain hazards: touch hurts and throws the player away from the side touched, up from the top; `src/hazard.rs`; `tests/hazards.rs`, manual
- Hazard looks: Spikes, Lava or Thorns, facing any side, own color; spike rows along walls; `src/hazard_look.rs`; `src/hazard_look/tests.rs`, manual
- Terrain look (drawing only): procedural draw list for solids and boundary, presets, per-room style; `src/terrain_style.rs`; `tests/terrain_style.rs`, manual
- Reach envelope: jump height, longest gap with no ability, Dash, Double Jump, on the real physics; per-ability interval; `src/reach.rs`; `src/reach/tests.rs`
- Gates: mark what an entrance to an exit needs (Dash, Double Jump); editor panel; orphans; `src/gates.rs`; `src/dungeon_check/tests/double_jump.rs`
- Check dungeon: unreachable rooms, unopenable locks, unobtainable abilities, traps; editor button; `src/dungeon_check.rs`; `tests/shipped_content.rs`
- Dungeon map data: room `map_cell` (-64..64) or placement by exits; conflicts, one-way links, key numbers; `src/dungeon_map.rs`; `src/dungeon_map/tests.rs`
- Map discovery: rooms seen this run, cleared as a run starts or ends, kept by Save, none in a test room; `src/game/map_link.rs`; `src/game/map_tests.rs`
- Map screen (M, Esc closes): seen rooms named, next rooms empty, one-way arrows, key numbers; `src/hud_map.rs`, `src/dungeon_map/grid.rs`; `src/dungeon_map/grid_tests.rs`, manual
- Map screen rules: freezes the world, shut in editor, test room, pause menu, shop, result; `src/game/map_link.rs`; `src/game/map_screen_tests.rs`
- Map keybind: rebindable Map action (default M), older keybinds files keep M; `src/keybinds.rs`; `src/keybinds/map_tests.rs`
- Editor Map cell row: Auto or X/Y, shows the resolved cell, one undo step; `src/editor_panel/map_panel.rs`; `src/editor_panel/panel_tests/map.rs`
- Editor Dungeon map window: every room and draft, marks clashes, click opens; `src/editor_panel/map_window.rs`; `src/dungeon_map/editor_tests.rs`, manual
- Check dungeon: two rooms with one map cell, naming both; `src/dungeon_check.rs`; `tests/editor_map.rs`, `tests/shipped_content.rs`
- Reach preview: outlines of a jump with no ability and with each ability; reach line after a Player or F4 save; `src/reach/editor.rs`; `tests/reach_editing.rs`
- Fixed 120 Hz step, capped catch-up, edge input on the first tick only; same jump apex at 30/60/144 fps; `src/timestep.rs`; `tests/timestep.rs`
- Follow camera: smoothing, look-ahead, clamped to room bounds plus boundary walls; `src/camera.rs`; `tests/camera_bounds.rs`, manual
- Virtual 960x540 render target, letterboxed window scaling, HUD anchored to it; `src/camera.rs`, `src/hud.rs`; `src/camera.rs` unit tests, manual
- HUD in virtual space: counters, room name, boss bar with the boss's own name, F5 messages; `src/hud.rs`, `src/hud_layout.rs`; `tests/hud_layout.rs`, manual
- Health and mana bars: fixed length, anchored, `data/hud.ron` with F5 override, pure layout; `src/hud_bars.rs`; `src/hud_bars/tests.rs`, manual
- Damage feedback on the health bar: lost-segment burst, sine shake, loss carried by `PlayerDamaged`; `src/hud_damage.rs`; `src/hud_damage/tests.rs`, manual
- Variable jump height, coyote time, jump buffering; `src/player.rs`; `tests/jumping.rs`
- Player and enemy-knockback tunables in `data/player.ron` (older files load), validated, staged save, F5 reload; `src/tuning.rs`; `tests/tuning.rs`
- F4 live tuning window: a drag box with range per `player.ron` field, staged Save; `src/tuning_panel.rs`; `tests/tuning.rs`, headless panel tests, manual
- Directional melee swing, cooldown, one hit per enemy per swing; `src/combat.rs`, `src/game.rs`; `tests/combat.rs`
- Up and down nail strikes with pogo, per-direction hitboxes and knockback; `src/combat.rs`; `tests/nail_strikes.rs`, manual
- Gameplay event queue: `GameEvent`s emitted once each, drained by `take_events`; `src/events.rs`; `src/game/event_tests.rs`
- Hit-stop and screen shake from events (sine shake, no randomness); `data/feel.ron` with override; `src/feel.rs`; `src/game/feel_tests.rs`, manual
- Located events: `take_located_events` gives each event the world position it happened at; `src/events.rs`; `src/game/located_event_tests.rs`
- Particles: hit_spark, dust, dash_trail, death_burst from located events, seeded; `src/particles.rs`, `src/effects.rs`; `src/particles/tests.rs`, manual
- Post-process pass: vignette and soft glow under the HUD, data toggle; `src/postfx.rs`, `src/postfx_pass.rs`; `src/postfx/tests.rs`, manual
- Parallax background layers per room (procedural shapes, depth), drawn behind terrain; `src/background.rs`; `tests/background.rs`, manual
- Sound hooks: `data/audio.ron` maps events to sound names and volumes, missing files skipped; `src/sound.rs`; `src/sound/tests.rs`, manual
- Collectible Dash (facing, duration, cooldown, one air dash); `src/item.rs`, `src/player.rs`; `tests/items.rs`
- Collectible Double Jump: one air jump, refilled on landing; tuned in Player panel and F4; `src/player/air_jump.rs`; `src/player/air_jump_tests.rs`
- Player health, invincibility, respawn; `src/health.rs`; `tests/enemies.rs`
- Health and damage scale: values are ten times the old whole hit (`HIT_DAMAGE` 10, `MAX_HEALTH` 50), stored as `u16`; `src/health.rs`; `tests/damage_scale.rs`
- Player vitals: trailing health bar, regain on sword hits, mana from kills, hold-to-heal; `data/vitals.ron`; `src/vitals.rs`; `src/vitals/tests.rs`, manual
- Charms: `data/charms.ron` rules (mana-budget equip, effects layered on base stats); `src/charms.rs`; `src/charms/tests.rs`, `src/game/charm_tests.rs`, manual
- Charm bars: bars keep their length and show charm capacity as an extension or a locked end beside the base; `src/hud_bars.rs`; `src/hud_bars/tests.rs`, manual
- Inventory charm menu: owned charms with swatch, cost, effect; equip by keys or mouse; `src/charm_menu.rs`; `src/charm_menu/tests.rs`, manual
- Grounded item pickup, persistent session inventory; `src/item.rs`; `tests/items.rs`, `tests/rooms.rs`
- Taken items stay gone after an editor Save (per run; a moved item is new; not in test rooms); `src/game/world_items.rs`; `src/game/item_tests.rs`, manual
- Coin pickups, persistent session balance; `src/coin.rs`; `src/coin.rs` unit tests
- Inventory screen with paused gameplay; `src/game.rs`, `src/main.rs`; `src/game.rs` unit tests, manual
- Pause menu: Escape opens it, the world stands still, Unpause / Settings / Exit by keys and mouse; `src/pause_menu.rs`; `src/pause_menu/tests.rs`, manual
- Keybinds: Settings rebinds each action with a primary and alternate key, saved to a file; `src/keybinds.rs`; `src/keybinds/tests.rs`, manual
- Three rooms, directional exits, spawns, persistent contents; `src/room.rs`; `tests/rooms.rs`
- Keyboard controls, world rendering, melee visual, health bars; `src/main.rs`; manual
- F5 atomic reload of rooms and definitions with scrollable errors; `src/game.rs`, `src/room.rs`; `tests/reloading.rs`
- Assets folder from `HOLLOW_STEPS_ASSETS`, then next to the executable, then the source tree; `src/assets.rs`; `src/assets.rs` unit tests
- Startup load errors shown on screen (F5 retries) instead of a panic; `src/startup.rs`, `src/game.rs`; `tests/startup.rs`, manual
- Current room kept as a stored index, recomputed when a save rebuilds the room list; `src/game.rs`; `tests/startup.rs`

## Enemies and attacks

- Enemy patrol, chase, contact damage, health, knockback, death; `src/enemy.rs`; `tests/enemies.rs`
- Enemy dying state: stays `death_secs`, flashing and fading, no damage or hits, then removed; `src/enemy/death.rs`; `tests/enemy_death.rs`, manual
- Flying enemies: `movement` Ground or Flying (hover, bob, wing flap), no gravity or terrain; `src/enemy/movement.rs`, `src/enemy/flap.rs`; `tests/flying.rs`
- Hitbox and visual size apart: `size` is the hitbox, optional `visual_size` the drawn box; `src/visual_box.rs`; `tests/visual_size.rs`, manual
- RON enemy definitions and placements, sequential actions, conditional transitions, validation; `src/behavior.rs`; `tests/behavior.rs`, `tests/definitions.rs`
- Shared lunge, jump, slam, projectile fan, radial blast, warning and recovery actions; `src/enemy_attack/`, `src/behavior/`; `tests/attack_actions.rs`
- Lunge samples player direction once, suspends gravity, stops at walls, keeps authored trajectories; `src/enemy_attack/runtime.rs`; `tests/attack_actions.rs`
- Rect/circle attack boxes, immunity-gated damage and knockback; `src/enemy_attack/attack_box.rs`, `src/player.rs`; `tests/attack_actions.rs`
- One hit per tick: strongest of enemy contact/blast/attack and boss action/contact damage; `src/damage.rs`, `src/game.rs`; `src/damage.rs` unit tests
- Jump-to-player, hover, slam, random area blasts with arena clamping and editor parameters; `src/action_runtime/movement.rs`; `tests/arena_actions.rs`
- Warning tint strength (0.25) and 30% AttackBox outline; `src/enemy_attack/options.rs`, `src/body_draw.rs`; `tests/attack_actions.rs`
- Compile-time guard keeping hitbox/attack motion independent of body state; `src/enemy_attack/`; `tests/pose_geometry.rs`

## Bosses

- Arena exits stay locked while the boss lives and open on defeat, kept through respawn and room travel; `src/boss_special.rs`; `tests/boss_special.rs`
- Arena boss cycling Slam, Summon, Rings with summons, death coins, drawing; `src/boss_special.rs`; `tests/boss_special.rs`
- RON boss definitions and placements, cycle/seeded random selection, ordered actions; `src/boss_definition.rs`, `src/boss_special.rs`; `tests/boss_data.rs`
- Boss editor: placement, drag, delete, ordered attacks/actions, staged saves with renames; `src/boss_editor.rs`; `tests/boss_editor.rs`
- Boss body: optional `body` in the boss RON, hurt flash, warning tint, death fade, poses; `src/boss_special/body_link.rs`; `tests/boss_body.rs`, manual

## Bodies and poses

- RON humanoid/bird/animal body templates, palettes, scale, point maps, mirroring, box fallback; `src/body/template.rs`, `src/body_draw.rs`; `tests/bodies.rs`
- Damped point following, idle motion, per-point springs, F5 motion preservation; `src/body/`, `data/body_motion_defaults.ron`; `tests/body_motion.rs`
- Charger/Hopper/Shooter use distinct bodies, palettes, scales; `assets/entities/`; `tests/shipped_content.rs`, `tests/bodies.rs`
- Semantic pose assets and per-template point mapping; `src/pose.rs`; `tests/poses.rs`
- Attack phase pose commands with spring target cleanup; `src/behavior/action_info.rs`, `src/action_runtime/poses.rs`, `src/enemy_attack/runtime.rs`; `tests/poses.rs`
- Telegraph-derived critically damped pose springs, blend ratio, strike stiffness, shot recoil; `src/pose.rs`, `data/pose_defaults.ron`; `tests/poses.rs`
- Eight authored poses, attack bindings, timed hurt pose and white flash; `assets/poses/`, `src/body/instance.rs`; `tests/poses.rs`
- Player body: a hooded wanderer with idle, run, jump and attack poses; the hitbox is unchanged; `src/player_body.rs`; `tests/player_body.rs`, manual
- Pose catalog/binding validation shared by load and staged saves; F5 reloads poses; `src/pose.rs`; `tests/poses.rs`, `tests/reloading.rs`
- Fixture-only pose regression suite (semantics, lifecycle, facing, hurt timing, hot reload); `tests/poses.rs`, `tests/fixtures/`
- F3 body debug overlay (dots, lines, hitboxes, nearest-point labels); `src/body_debug.rs`, `src/body_draw.rs`; `tests/body_debug.rs`, manual
- F3 legend shows nearest body's current pose; `src/body_debug.rs`; `tests/body_debug.rs`, manual

## Room and definition editor

- F2 paused room editor: placement, move, delete, entity list, spawn/exit editing, atomic RON saves, custom rooms; `src/editor.rs`; `tests/editor.rs`
- Entity/behavior side panel, state/link editing, shared-name updates, staged RON saves; `src/definition_editor/`; `tests/definition_editor.rs`
- Responsive scrollable panel, pinned Save, ordered action/transition rows, type catalogs, RON editing; `src/editor_panel.rs`; `tests/definition_editor.rs`
- Save conflict detection, full validation, LF writes, recoverable staging, repeat renames; `src/definition_editor/plan.rs`; `tests/definition_editor.rs`
- Searchable lists with stable selection, Unicode prefix-first matching, counts, management buttons; `src/search.rs`; `tests/definition_editor.rs`
- Patrol handles, exit/arrival deletion, item catalog, no-exit warnings, transactional saves, Windows-safe names; `src/editor.rs`; `tests/editor.rs`
- egui panel: collapsing sections, filtered dropdowns, tooltip truncation, solid scrollbars, canvas input capture; `src/editor_panel.rs`; headless tests, manual
- Editor canvas view: wheel zoom, middle-drag or Space+drag pan, Home reset, grid with snap; `src/editor_view.rs`; `tests/editor_view.rs`, manual
- Room size section (x, y, width, height, floor_y); a change leaving contents outside is refused; `src/room_size.rs`; `tests/room_size.rs`
- Room names: Room name box, 1-32 chars, unique ignoring case, New room asks first; `src/room_name.rs`; `tests/room_names.rs`, `tests/new_room.rs`, manual
- Room file names: only `room_<number>.ron` without leading zeros loads; a bad name errors with the fix; `src/editor_files.rs`; `tests/room_file_names.rs`
- Room order: custom rooms listed by id (room_2 before room_10), at start-up and after Save; `src/editor_files.rs`; `tests/room_order.rs`
- Door exits: Up in front of a door on the ground travels, "Up: enter" prompt; `src/game/world_doors.rs`, `src/door_exit.rs`; `src/game/door_tests.rs`, manual
- Doors in the editor: Add door (and link), drag, width and height; `src/editor_panel/door_panel.rs`; `src/door_exit/tests.rs`, `tests/door_exits.rs`, manual
- Abilities and keys as run state; Keys 1-8 open locked exits; editor Locked by; `src/exit_lock.rs`, `src/ability.rs`; `src/game/world_locks/tests.rs`, manual
- Linked exits: pick or create room B, connect an exit one-way or two-way, nothing written until Save; `src/exit_link.rs`; `tests/exit_link.rs`, manual
- Arrival editing in place: other rooms' arrivals into this room drawn and draggable; `src/editor_inbound.rs`; `tests/exit_arrival_editing.rs`, manual
- Unsaved-changes guard: closing the editor, Open room, New room ask Save / Discard / Cancel when dirty; `src/editor_guard.rs`; `tests/editor_guard.rs`, manual
- Room look in the editor: Room look section edits `layout.terrain_style` with a preview; `src/editor_look.rs`; `tests/editor_look.rs`, manual
- Room undo/redo: buttons and Ctrl+Z / Ctrl+Y over room edits, a drag is one step, 100 steps; `src/editor_history.rs`; `tests/editor_history.rs`, manual
- Test room: button and F6 play the unsaved draft from its spawn, F2 returns, nothing written; `src/game/test_room.rs`; `tests/test_room.rs`, manual
- Hazards in the editor: Add hazard, drag, size, damage, knockback, shape, facing, color, undo; `src/editor_hazard.rs`; `tests/hazards.rs`, manual
- Breakable walls: solid until the nail hits them N times (1-9), one hit per swing, no pogo; `src/puzzle.rs`; `src/game/puzzle_play_tests.rs`, manual
- Switches and barriers: the nail turns a numbered switch on for the run and its barrier opens; `src/puzzle.rs`; `src/game/world_puzzles/tests.rs`, manual
- Puzzle pieces in the editor: add, drag, size, hits, numbers, delete, undo, Test room, Save; `src/editor_puzzle.rs`; `src/game/puzzle_editor_tests.rs`, manual
- Editor tools: Select, Place, Terrain, Erase (V, P, T, E), grid-snapped solids with handles; `src/editor_terrain.rs`; `tests/editor_terrain.rs`, manual
- Behavior presets (Patrol, Chaser, Charger, Skittish, Stationary), searchable creation, usage warnings, Key values; `src/preset.rs`; `tests/presets.rs`
- Player stats in the editor: Player panel with every `player.ron` number, validated, staged Save; `src/player_panel.rs`; `tests/player_editor.rs`, manual
- Max mana guard: a vitals Save (Player panel, F4, Save all) below a saved charm's cost is refused; `src/player_editor.rs`; `tests/max_mana_guard.rs`, manual
- Player look in the editor: Look section edits `player_body.ron` with a spawn-point preview; `src/player_look_editor.rs`; `tests/player_look.rs`, manual
- Playing and undoing the edited player: Test room plays unsaved stats and look, own undo; `src/player_history.rs`; `tests/player_test_room.rs`, manual
- Charm editor panel: every charm with every `charms.ron` field, validated, staged Save; `src/charm_panel.rs`; `tests/charm_editor.rs`, manual
- Charm editing in the session: unsaved Charms in the guard, Test room, own undo/redo; `src/charm_history.rs`; `tests/charm_unsaved.rs`, manual
- Upgrade editor panel: every upgrade with every `upgrades.ron` field, validated, staged Save, max totals; `src/upgrade_panel.rs`; `tests/upgrade_editor.rs`, manual
- Upgrade editing in the session: unsaved Upgrades in the guard, Test room, own undo/redo; `src/snapshot_history.rs`; `tests/upgrade_test_room.rs`, manual
- Charms in rooms: `layout.charms`, placed in the editor, picked up once, owned for the run; `src/room_charms.rs`; `tests/world_charms.rs`, manual
- Player health, damage and mana in the editor: `max_health`, sword damage, vitals; `src/player_editor.rs`; `tests/player_stats.rs`, manual
- Blank-enemy creation and behavior assignment, editable Skittish distances/speed; `src/definition_editor/entities.rs`, `src/behavior/`; `tests/presets.rs`
- Catalog-backed pose dropdowns for attack phases in rows and Key values; `src/editor_panel/widgets.rs`; `tests/poses.rs`
- Body, pose, spring, hurt, blend and tint values all have data, validation, editor controls; `src/body/`, `src/pose.rs`, `data/`; `tests/poses.rs`

## Hub and runs

- Profile: coins, tier and clears kept in `profile.ron` (config folder), staged save, bad file kept as `.bak`; `src/profile.rs`; `src/profile/tests.rs`
- Run rules: `data/run.ron` (hub, start and final boss room, coin share), outcomes, banking, saturating; `src/run.rs`; `src/run/tests.rs`
- Run loop: hub exit starts a run; death, final boss or Abandon ends it; frozen result, Enter returns; `src/game/run_link.rs`; `src/game/run_tests.rs`
- Pause menu in a run: Abandon run with confirm (Keep playing default, Esc back); `src/pause_menu.rs`; `src/pause_menu/run_tests.rs`, `src/game/run_pause_tests.rs`
- Result screen and counters: text, layout, HUD and inventory captions; `src/run/screen.rs`, `src/hud_run.rs`; `src/run/screen/tests.rs`, manual
- Shipped hub and shop (`assets/run.ron`); F4 Run (debug): Coins, tier, Reset; `src/tuning_panel/run.rs`; `run/tests.rs`, `tests/shipped_content.rs`
- Run loop edges: profile save and bad file, test-room sandbox, editor moves, no hub; `src/game/run_link.rs`; `src/game/run_profile_tests.rs`
- Permanent upgrades: levels bought and saved in the profile, layered on stats; `src/upgrades.rs`; `src/upgrades/tests.rs`, `src/game/upgrade_tests.rs`
- Upgrade shop: Up at the hub stall opens it, buy levels with banked coins; `src/shop_menu.rs`; `src/shop_menu/tests.rs`, `src/game/shop_tests.rs`
- Difficulty rules: per-tier percents in `data/difficulty.ron`, cap, `scale(tier)`; `src/difficulty.rs`; `src/difficulty/tests.rs`, `tests/shipped_content.rs`
- Tier on a run: enemy/boss health scaled at run start and kept on F5/Save; `src/game/tier_link.rs`, `src/enemy/tier.rs`; `src/game/tier_tests.rs`
- Tier damage: `DamageResolver::gather_scaled` scales enemy and boss hits (contact, actions, attacks, blasts); `src/damage/tier.rs`; `src/damage/tier/tests.rs`
- Tier coins: a run's coins scaled before banking, result line "Tier N bonus: +X%"; `src/run/bonus.rs`; `src/run/bonus/tests.rs`, `src/game/tier_tests.rs`
- Min tier on enemy/boss placements: out of play below it, kept by editor and Save; `src/game/tier_link.rs`; `tests/min_tier.rs`, `src/game/min_tier_tests.rs`
- Editor Min tier field (0-99), one undo step; `src/editor_min_tier.rs`, `src/editor_panel/min_tier_field.rs`; `editor_panel/min_tier_field/tests.rs`
- F4 Difficulty section: six numbers, read-out, Save writes difficulty.ron; `src/tuning_panel/difficulty.rs`; `difficulty/tests.rs`, `difficulty/draw_tests.rs`
- Rules set in F4 apply at the next run only, saved staged; HUD "Tier N"; `src/game/tier_link.rs`, `src/run/screen.rs`; `src/game/difficulty_tests.rs`

## Run seed and variety

- Run seed: `SeedSource` (SplitMix64, clock at start-up), `stream(seed, label, key)`, 16-hex `format_seed`/`parse_seed`; `src/seed.rs`; `src/seed/tests.rs`
- A run takes a seed at start (can be fixed), kept to the result screen, none in hub or test room, not saved; `src/game/seed_link.rs`; `src/game/seed_tests.rs`
- Result screen's last line `Seed: <16 hex>`, after the tier line, dimmer; `src/run/seed_line.rs`; `src/run/seed_line/tests.rs`
- F4 Run (debug) Seed row: Fixed seed box, hex text box (bad text refused, old seed kept), seed label; `src/tuning_panel/run/seed.rs`; `run/seed/*tests.rs`
- Optional placements: enemy `group` (1-9, one member stays) and `chance` (0-100, default 100); a run's seed decides; `src/variety.rs`; `src/variety/tests.rs`
- Only a run's rooms are thinned, min tier first: not the hub, test room, loaders or Save; F5 keeps the picks; `src/game/tier_link.rs`; `game/variety_tests.rs`
- Editor Group (None box, 1-9 drag) and Chance % rows under Min tier, one undo step each; `src/editor_panel/variety_fields.rs`; `src/editor_variety.rs`
- Player is left-right symmetric, proved on the real `Player` (run, jump, air jump, dash, swing) against a mirror; `tests/flip.rs`
- A room seen in a mirror: layout, patrols, boss, exits, gates, map cell; arrivals into flipped rooms; `src/mirror.rs`; `src/mirror/tests/`
- `variety.ron` `flip_percent` (0-100; shipped 50): chance a run flips; `src/variety/flip.rs`; `src/variety/flip/tests.rs`; `tests/shipped_content.rs`
- A run decides its flip at start from its seed; F4 Flip next run: Auto/Never/Always; `src/game/flip_link.rs`; `src/tuning_panel/run/flip.rs`
- A flipped run builds every room but the hub from mirrored documents; plain loaders and the editor unchanged; `src/mirror/source.rs`; `src/game/world_load.rs`
- Flipped run: hub exit arrives at the mirrored spot, F5 keeps the flip, Continue is unflipped; `src/game/flip_link.rs`; `flip_link/play_tests.rs`
- F2 and Open room refuse in a flipped run, saying how out; hub, unflipped runs and the test room are unaffected; `src/game/flip_link.rs`

## Dungeon pieces

- A room can be a dungeon piece (Start, Middle, Boss; weight 1-10) with up to 8 pickup slots (Key, Ability, Any); `src/piece.rs`; `piece/tests.rs`
- Slots must be finite, inside, off solids and apart; Room and `from_room` carry mark and slots, the mirror flips slots; `src/piece.rs`; `src/mirror.rs`
- Dungeon piece section: This room is a piece box, Role, Weight 1-10; one undo step each; `src/editor_panel/piece_panel.rs`; `piece_panel/tests.rs`
- Add pickup slot, drag, Kind dropdown, Delete; a dashed teal ring with K, A or ? marks it; `src/editor_piece.rs`; `src/editor_visuals/slot_mark.rs`
- Mark and slots survive Save and reload; unmarking keeps the slots; the Test room shows no slot as a pickup; the unsaved guard asks; `tests/pieces_editing.rs`

## Tests and CI

- Fixture-based tests; only `tests/shipped_content.rs` reads `assets/` (load and validate); `tests/fixtures/`, `tests/support/mod.rs`
- CI: fmt, test, Clippy on Linux and Windows for PRs and `main`; `.github/workflows/ci.yml`
- MIT `LICENSE` and a minimum Rust version (`rust-version` in `Cargo.toml`, named in the README); no test
