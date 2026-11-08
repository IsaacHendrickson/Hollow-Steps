# Features

- Walking, gravity, floor and wall collision: `src/player.rs`; `src/player.rs` unit tests, `tests/gameplay.rs`.
- Variable jump height, coyote time, and jump buffering: `src/player.rs`; `tests/jumping.rs`.
- Directional melee swing, short hitbox, cooldown, and one hit per enemy per swing: `src/combat.rs`, `src/player.rs`, `src/game.rs`; `tests/combat.rs`, `src/game.rs` unit tests.
- Enemy patrol, chase, contact damage, health, knockback, and death: `src/enemy.rs`, `src/game.rs`; `tests/enemies.rs`, `tests/combat.rs`, `src/game.rs` unit tests.
- Player health, invincibility, and respawn: `src/health.rs`, `src/player.rs`, `src/game.rs`; `tests/enemies.rs`, `src/game.rs` unit tests.
- Grounded item pickup and persistent session inventory: `src/item.rs`, `src/player.rs`, `src/game.rs`; `tests/items.rs`, `tests/rooms.rs`.
- Collectible Dash ability, facing, duration, cooldown, and one air dash per landing: `src/item.rs`, `src/player.rs`; `src/player.rs` dash tests, `tests/items.rs`, `src/game.rs` unit tests.
- Coin contact pickups and persistent session balance: `src/coin.rs`, `src/game.rs`, `src/room.rs`; `src/coin.rs` and `src/game.rs` unit tests.
- Inventory screen and paused gameplay: `src/game.rs`, `src/main.rs`; `src/game.rs` unit tests (screen needs manual playtest).
- RON room files, validated geometry and spawn/exit references, placed entities, directional travel (tested with LF and CRLF files), and persistent room contents: `assets/rooms/`, `src/room_data.rs`, `src/room.rs`, `src/game.rs`; `tests/room_data.rs`, `tests/rooms.rs`, `tests/boss.rs`, `src/room.rs` and `src/game.rs` unit tests.
- Reusable Ash Warden timed attacks, melee damage, persistent death, and locked arena exits: `src/boss.rs`, `src/room.rs`, `src/game.rs`; `tests/boss.rs`, `src/game.rs` boss tests.
- Playable arena boss gliding at a fixed speed while cycling Slam, Summon, and Rings every three seconds, melee damage, summoned enemies, death coins, and attack drawing: `src/boss_special.rs`, `src/game.rs`, `src/room.rs`, `src/main.rs`; `tests/boss_special.rs`, `src/game.rs` special tests, plus manual playtest.
- Keyboard controls, world rendering, melee visualization, and health bars: `src/main.rs`; gameplay tests above, plus manual playtest.
