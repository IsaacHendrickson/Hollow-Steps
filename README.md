# Hollow Steps

Hollow Steps is a small Rust 2D action-platformer prototype inspired by the
feel of exploration-focused games such as Hollow Knight. The first vertical
slice is intentionally narrow: one player can walk, jump, and travel between
three bounded rooms, including a boss arena.

## Run it

Install stable Rust. On Ubuntu/Debian, install Macroquad's X11, OpenGL, and
ALSA development dependencies first:

```sh
sudo apt-get update
sudo apt-get install -y pkg-config libx11-dev libxi-dev libgl1-mesa-dev libasound2-dev
```

Then run:

```text
cargo run
```

Sound is optional: no sound files ship, and the game runs without them. To hear
sounds, add files named in `assets/audio/README.md` and run
`cargo run --features audio` (see `docs/audio.md` for why audio is opt-in).

The game finds its `assets` folder in this order: the `HOLLOW_STEPS_ASSETS`
environment variable, an `assets` folder next to the executable, then the
source tree it was built from. So a built binary can be copied with its
`assets` folder to another machine. If the assets fail to load, the game shows
the error on screen; fix the files and press F5 to retry.

Useful checks while developing:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

GitHub Actions runs these checks on `ubuntu-latest` and `windows-latest` for
every pull request and push to `main`. The Linux job installs the packages
above; Windows needs no extra system libraries.

## Current controls

- `A`/`D` or the left/right arrow keys: move
- `Space` or `Z`: jump; hold to rise higher, release for a shorter jump.
  Jump input is buffered for 0.12 seconds before landing; coyote time allows
  a jump for 0.1 seconds after leaving support.
- `X`: swing in the last movement direction, including while airborne.
  The hitbox extends 38 pixels for 0.12 seconds with a 0.35-second cooldown.
  Each swing hits each enemy once; enemies take three hits to defeat (health and
  damage are in tenths of a hit: a sword hit is 10, the player has 50) and
  are knocked back on impact. Their health bars show remaining health.
- `Up` + `X`: strike upward. `Down` + `X` while airborne: strike downward (on
  the floor it is an ordinary forward swing). A downward strike that hits an
  enemy or the boss bounces you up (once per swing) and restores your air dash.
  Up and down hits push enemies sideways, away from you. Box sizes and the
  bounce speed are the `strike` values in `data/player.ron` (F4 window).
- `Shift`: after collecting Dash, dash horizontally in the facing direction at
  800 units/second for 0.2 seconds. Dash starts have a 0.5 second cooldown;
  only one air dash is available until landing.
- `C`: hold on the ground to heal. After a short delay mana turns into health (30 mana a
  second, one health each); let go and the health is added a moment later. Letting go during
  the delay costs nothing. While healing you walk at half speed and cannot jump, dash or
  attack. Damage cancels the heal (the health not yet added is lost, the mana spent is not
  refunded) and you must let go and press again; walking off a ledge ends it at once with the
  health already bought. Mana (up to 100) comes from sword kills of regular enemies, 10 each,
  never bosses. A sword hit on an enemy or boss gives back half of the health you lost
  recently, the part the trailing bar still remembers (the bar itself comes in a later task).
  All the numbers are in `data/vitals.ron` (F4 window).
- `I`: open/close the inventory screen to see coins and collected items.
  Gameplay pauses while it is open; `Esc` also closes it.
- `F2`: open/close the room editor (gameplay pauses).
- `F4`: open/close the player tuning window (gameplay keeps running). Drag or type values for
  move, jump, dash, size, strike boxes, pogo and enemy knockback, then the vitals (mana, health
  trail, healing); they apply at once. `Save` writes `player.ron` in the
  assets folder, which replaces the shipped `data/player.ron` defaults (delete it to go back),
  and `vitals.ron` too when a vitals value changed (same for `data/vitals.ron`).
- `F5`: validate `player.ron`, `vitals.ron` (your health, mana and any heal in progress are kept), room files and their enemy, body template, behavior, and boss definitions, including
  while the inventory is open. Errors appear on screen; fix the files and
  press `F5` again to retry. Use the mouse wheel to scroll long errors.
- Touch a gold coin to collect it automatically, including while airborne.
  Each coin adds one to the balance and stays collected through room travel
  and respawn for the current game session.
- `Up` (without `X`): pick up a nearby item while standing on the floor. The Amber Shard
  and Dash items highlight and show a prompt when in reach; collected items leave
  the world and stay in the player's inventory for the current game session.
- Walk into the right exit of the first room to enter the second room; its
  left exit leads back. The second room's right exit leads to the Ash Warden.
  Its arena exit stays red and locked until the boss is defeated.

## Reference docs

Design and reference notes live in `docs/` so everyday tasks do not have to
read them. Open only the one you need:

- [Enemy body templates and poses](docs/enemy-bodies.md)
- [Boss architecture](docs/boss-architecture.md)
- [Room design](docs/room-design.md)
- [Gameplay ownership](docs/gameplay-ownership.md)
- [Data-driven enemies](docs/data-driven-enemies.md)
- [Room editor design, behavior presets, and editor manual checks](docs/room-editor.md)

Project rules and the module list are in [AGENTS.md](AGENTS.md); the feature
list is in [FEATURES.md](FEATURES.md).
