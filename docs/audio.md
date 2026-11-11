## Sound hooks: design note

`GameEvent`s drive sound the same way they drive particles. `src/sound.rs` is pure: it
maps events to sound names (`data/audio.ron`, validated; an `assets/audio.ron` replaces it
whole), checks volumes and names, finds `assets/audio/<name>.wav` or `.ogg`, and turns a
batch of events into a list of sounds to start. `src/effects/sound_bank.rs` loads the
files and plays them with Macroquad. A cue whose file is missing plays nothing, so the
game runs with no sound files.

### Decision: audio is an opt-in cargo feature

Macroquad's `audio` feature is off in this project (Macroquad's own default), and without
it `macroquad::audio` is a stub that prints a warning on every play. Real playback needs
`macroquad/audio`, exposed here as the project feature `audio`:

    cargo run --features audio

It is not on by default because of what turning it on does (checked against Macroquad
0.4.16 and quad-snd 0.2.8):

- **Linux linking.** quad-snd links the system ALSA library (`-lasound`). `cargo check` and
  `cargo clippy --all-features` do not link and work without it, but `cargo build` and
  every test binary need `libasound2-dev` once the feature is on. The default build, the
  tests and CI need no Linux dev packages today (see `room-editor.md` and `ci.yml`), and
  this keeps that true.
- **Eleven extra crates** in `Cargo.lock` (quad-snd, quad-alsa-sys, audrey, hound, lewton,
  ogg, dasp_sample, dasp_frame, smallvec 0.6, maybe-uninit, audir-sles). All are
  Macroquad's own optional audio dependencies; none are new third-party authors.
- **No sound device.** On Linux the backend's audio thread panics when it cannot open a
  PCM device. The game keeps running (the thread dies, later plays print "Audio thread
  died"), but there is no sound.
- **Bad files.** The backend `unwrap`s and `assert`s on data it cannot decode (not
  WAV/Ogg, more than two channels), which would stop the game. The sound bank checks the
  file header first (`sound::check_format`) and loads under `catch_unwind`, so a bad file
  is skipped with a line on stderr instead.

Without the feature the sound bank is empty and plays nothing; if sound files exist it
prints one line saying the build has no audio.

### Making sound the default later

Change `Cargo.toml` to `default = ["audio"]` under `[features]`, and add
`sudo apt-get install -y libasound2-dev` before the build step of the Linux job in
`.github/workflows/ci.yml`. Update the "no Linux dev packages" statements in
`room-editor.md` and the CI comment in the same change.
