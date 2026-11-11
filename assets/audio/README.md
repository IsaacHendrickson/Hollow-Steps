# Sound files

No sound files ship with the game. The game runs without any of them: an event whose
file is missing is simply silent. Add your own files here. Use only sounds you made or
have the right to use.

Sound plays only in a build with audio: `cargo run --features audio`. On Linux that
needs the ALSA development package (`sudo apt-get install -y libasound2-dev`). Without
`--features audio` the game prints one line if it finds files here and plays nothing.

## Expected file names

Each name is `<sound>.wav` (tried first) or `<sound>.ogg`. The table is the default
mapping in `data/audio.ron`; copy that file to `assets/audio.ron` to change which event
plays which sound, or to change volumes. The copy replaces the default whole, and F5
reloads it together with the sound files.

| File | Plays when |
|---|---|
| `jump` | the player jumps |
| `land` | the player lands after a fall |
| `dash` | the player dashes |
| `swing` | the player swings (hit or miss) |
| `hit` | a swing hits an enemy or the boss |
| `enemy_die` | an enemy is defeated |
| `hurt` | the player takes damage |
| `coin` | a coin is collected |
| `pickup` | an item is picked up |
| `room_enter` | the player walks through an exit into another room |
| `boss_defeated` | the boss is defeated |

## Format

16-bit PCM WAV or Ogg Vorbis, mono or stereo. Anything else (MP3, FLAC, more than two
channels) is skipped with a line on the console saying why; it never stops the game.
Keep effects short; the same sound can start again before the last one has finished.
