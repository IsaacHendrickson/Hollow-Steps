# Features

- Shared lunge (direction sampled once at wind-up end), parabolic jump, impact slam, projectile fan with editable recovery, radial blast with editable recovery, warning feedback and recovery actions; Charger/Hopper/Shooter assignments preserve legacy behavior and boss rules: `src/enemy_attack.rs`, `src/behavior.rs`, `src/game.rs`, `src/body_draw.rs`, `data/action_defaults.ron`, `assets/behaviors/`; `tests/attack_actions.rs`, `tests/arena_actions.rs`, `tests/boss_data.rs`, `tests/definition_editor.rs`, `tests/reloading.rs`, `src/game.rs` immunity regression; manual warning/projectile/editor check.

- Shared rectangular/circular enemy attack boxes, independent body geometry and immunity-gated damage/knockback: `src/enemy_attack.rs`, `src/player.rs`; `tests/attack_actions.rs`.

- Session-only F3 body debug overlay in gameplay and the room editor, mirrored rest/current/target dots, target lines, gameplay hitboxes, and nearest-body point labels with fixed integer font sizes, zoom-independent generation, and a private font atlas using egui's bundled monospace font so overlay glyphs never resize/corrupt the HUD atlas; camera/filter state stays unchanged: `src/body_debug.rs`, `src/body_draw.rs`, `src/main.rs`; `tests/body_debug.rs`, `src/body_draw.rs` headless font test, plus manual F3/facing/editor/HUD check.

- Data-driven damped point following with 120/22 defaults and subtle humanoid/bird/animal idle motion, game-time pause, local-space target/snap API, per-point spring overrides, validation, F5 motion preservation, and shared-template editor controls/save round trips: `src/body.rs`, `src/enemy.rs`, `src/game.rs`, `src/definition_editor.rs`, `src/editor_panel.rs`, `data/body_motion_defaults.ron`, `assets/bodies/`; `tests/body_motion.rs`, `tests/bodies.rs`, `src/editor_panel.rs` headless tests, plus README manual check.

- RON humanoid/bird/animal body templates, per-enemy palettes/scale and runtime point maps, ordered anchored shapes/point-connected limbs, horizontal mirroring, unchanged box fallback and hitboxes, editor body controls and staged-save validation, and atomic F5 body reload: `src/body.rs`, `src/body_draw.rs`, `src/enemy.rs`, `src/definition_editor.rs`, `src/editor_panel.rs`, `assets/bodies/`; `src/body.rs` unit tests, `tests/bodies.rs`, `tests/shipped_rooms.rs`, plus manual visual/editor check.

- Boss entity picker and canvas placement/selection/drag/delete, egui boss properties and ordered attack/action editing with searchable action catalog, transactional validation and staged boss saves with rename propagation and runtime refresh: `src/boss_editor.rs`, `src/editor.rs`, `src/editor_panel/boss.rs`, `src/definition_editor.rs`, `src/game.rs`, `src/main.rs`; `tests/boss_editor.rs`, `tests/definition_editor.rs`, headless panel and canvas regression tests, plus manual editor check.

- Validated RON boss definitions and room placements, cycle/seeded random attack selection, ordered shared actions, preserved Ash Warden Slam/Summon/Rings behavior, and a fixture-only ground pound: `src/boss_definition.rs`, `src/boss_special.rs`, `src/behavior.rs`, `src/room.rs`, `assets/bosses/`; `tests/boss_data.rs`, `tests/boss_special.rs`, `src/game.rs` boss tests.

- Data-driven jump-to-player, hover, downward slam, and fused random area blasts for every enemy, arena/body clamping, blast damage through player immunity, generated labeled/validated editor parameters and data-backed insertion defaults: `src/behavior.rs`, `src/enemy.rs`, `src/game.rs`, `src/behavior_values.rs`, `src/editor_panel.rs`, `src/main.rs`, `data/action_defaults.ron`; `tests/arena_actions.rs`, `src/game.rs` blast test, existing catalog/save round-trip tests in `tests/definition_editor.rs`, plus README manual playtest.

- Validated RON behavior presets (Patrol, Chaser, Charger, Skittish, Stationary), searchable creation and optional entity assignment, shared-behavior usage/copy warning, and labeled numeric Key values with advanced rows: `src/preset.rs`, `src/behavior_values.rs`, `src/definition_editor.rs`, `src/editor_panel.rs`, `assets/presets/`; `tests/presets.rs`, `tests/shipped_rooms.rs`, `src/editor_panel.rs` headless tests, plus manual editor check.
- Independent blank-enemy creation, blank-behavior assignment with structured controls, and editable Skittish start/stop distances and speed persisted to gameplay: `src/definition_editor.rs`, `src/editor_panel.rs`, `src/behavior.rs`, `src/behavior_values.rs`; `tests/definition_editor.rs`, `tests/presets.rs`, `src/editor_panel.rs` headless tests.

- Continuous integration: `.github/workflows/ci.yml`; formatting, tests, and Clippy on Linux and Windows for pull requests and pushes to `main`, with a 20-minute job limit and no unnecessary apt dependencies.
- Walking, gravity, floor and wall collision: `src/player.rs`; `src/player.rs` unit tests, `tests/gameplay.rs`.
- Variable jump height, coyote time, and jump buffering: `src/player.rs`; `tests/jumping.rs`.
- Directional melee swing, short hitbox, cooldown, and one hit per enemy per swing: `src/combat.rs`, `src/player.rs`, `src/game.rs`; `tests/combat.rs`, `src/game.rs` unit tests.
- Enemy patrol, chase, contact damage, health, knockback, and death: `src/enemy.rs`, `src/game.rs`; `tests/enemies.rs`, `tests/combat.rs`, `src/game.rs` unit tests.
- RON enemy definitions and room placements, sequential movement/wait/telegraph actions, conditional state transitions, and asset validation with portable error assertions: `src/behavior.rs`, `src/enemy.rs`, `src/room.rs`, `assets/`; `tests/behavior.rs`, `tests/definitions.rs`.
- F5 reloads all room placements and enemy definitions atomically, with wrapped, scrollable on-screen validation errors and retry: `src/game.rs`, `src/room.rs`, `src/main.rs`; `tests/reloading.rs`, plus manual playtest.
- Player health, invincibility, and respawn: `src/health.rs`, `src/player.rs`, `src/game.rs`; `tests/enemies.rs`, `src/game.rs` unit tests.
- Grounded item pickup and persistent session inventory: `src/item.rs`, `src/player.rs`, `src/game.rs`; `tests/items.rs`, `tests/rooms.rs`.
- Collectible Dash ability, facing, duration, cooldown, and one air dash per landing: `src/item.rs`, `src/player.rs`; `src/player.rs` dash tests, `tests/items.rs`, `src/game.rs` unit tests.
- Coin contact pickups and persistent session balance: `src/coin.rs`, `src/game.rs`, `src/room.rs`; `src/coin.rs` and `src/game.rs` unit tests.
- Inventory screen and paused gameplay: `src/game.rs`, `src/main.rs`; `src/game.rs` unit tests (screen needs manual playtest).
- Three rooms, directional exits, spawn handling, and persistent room contents: `src/room.rs`, `src/game.rs`; `tests/rooms.rs`, `tests/boss.rs`, `src/room.rs` and `src/game.rs` unit tests.
- Reusable Ash Warden timed attacks, melee damage, persistent death, and locked arena exits: `src/boss.rs`, `src/room.rs`, `src/game.rs`; `tests/boss.rs`, `src/game.rs` boss tests.
- Playable arena boss gliding at a fixed speed while cycling Slam, Summon, and Rings every three seconds, melee damage, summoned enemies, death coins, and attack drawing: `src/boss_special.rs`, `src/game.rs`, `src/room.rs`, `src/main.rs`; `tests/boss_special.rs`, `src/game.rs` special tests, plus manual playtest.
- Keyboard controls, world rendering, melee visualization, and health bars: `src/main.rs`; gameplay tests above, plus manual playtest.

- F2 paused room editor, mouse placement/movement/deletion, entity list, spawn/exit editing, validated atomic RON saves, and persistent custom rooms: `src/editor.rs`, `src/game.rs`, `src/main.rs`, `src/room.rs`; `tests/editor.rs`, plus manual playtest.

- Behavioral tests use fixed room/entity/behavior fixtures independently of editable game assets; one shipped-content validation smoke test: `tests/fixtures/`, `tests/support/mod.rs`, `tests/shipped_rooms.rs`, `src/game.rs`, `src/room.rs`; existing behavioral suites.

- Selected-enemy entity and behavior side panel, state/link editing, shared-name reference updates, and validated staged RON saves: `src/definition_editor.rs`, `src/editor.rs`, `src/main.rs`; `tests/definition_editor.rs`, plus manual playtest.

- Responsive scrollable editor with pinned Save, structured ordered action/transition rows, loader-generated type catalogs, inline numeric validation, and advanced RON editing: `src/behavior.rs`, `src/definition_editor.rs`, `src/main.rs`, `src/editor_panel.rs`; `tests/definition_editor.rs`, headless egui layout/input tests, plus manual resize/edit check.

- Definition save conflict detection with consistent native paths and portable file-identification assertions, full draft and all-room validation, content-only LF writes, recoverable staging/deletions, repeat renames, and pure entity/behavior operations: `src/definition_editor.rs`, `src/editor.rs`; `tests/definition_editor.rs`.

- Reusable searchable room/entity/behavior/action/condition/state lists with stable selection, Unicode case-insensitive prefix-first matching, counts, no-match rows, bounded labels, and entity/behavior management buttons with reference errors: `src/search.rs`, `src/editor.rs`, `src/definition_editor.rs`, `src/main.rs`; `src/search.rs` unit tests, `tests/definition_editor.rs`, plus manual editor check.

- Patrol endpoint handles with wall/body validation, exit/arrival deletion, enum item catalog, nonblocking no-exit warnings, existing-behavior selection, definition operation buttons with reference errors, stale property-error clearing, bounded mouse-wheel panel scrolling, transactional editor saves, and Windows-safe filenames: `src/editor.rs`, `src/definition_editor.rs`, `src/item.rs`, `src/game.rs`, `src/main.rs`; `tests/editor.rs`, `tests/definition_editor.rs`, `src/main.rs` UI state tests, plus manual editor check.

- egui editor panel with pinned Save, collapsing sections, filtered dropdowns with eight-row scroll viewports, counts/no-match rows, tooltip truncation, draggable solid scrollbars, and canvas input capture: `src/editor_panel.rs`, `src/main.rs`; `src/editor_panel.rs` headless tests, preserved `src/search.rs` tests, plus README manual checklist.
