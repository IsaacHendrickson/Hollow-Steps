# Variety: the run seed

Every run has a **seed**, one `u64` that makes a run's random choices repeatable. The seed is
the first thing that makes one run differ from the next without any file being edited. Optional
enemy placements are the first thing that uses it.

## The seed

- **Where it comes from.** `SeedSource` (`src/seed.rs`) hands out one seed per run: it walks a
  SplitMix64 sequence from a start value. `main.rs` reads the system clock **once**, at
  start-up, and gives the game a source started from it (`GameState::use_seed_source`); nothing
  else reads a clock. A game that is never given a source starts from a fixed constant, so no
  test is random.
- **When a run takes it.** `start_run` takes the next seed and the run keeps it for its life,
  through the result screen. `GameState::run_seed()` is `Some` during a run and on the result
  screen, and `None` in the hub, in a game with no hub and in a **test room** (a sandbox takes no
  seed and uses none up). `last_run_seed()` is the seed of the last run started, even after it is
  over (F4's label uses it). A run that cannot start (a room file is broken) takes no seed.
- **Fixing it.** `set_fixed_seed(Some(seed))`: every run then starts with exactly that seed, and
  the source does not move, so turning it off goes on with the sequence it was in. A run already
  going keeps the seed it started with.
- **Not saved.** The seed is not in `profile.ron`: it describes one run, and a fresh launch
  starts from the clock again.

## The streams

One seed feeds many uses, and no use may disturb another. `stream(seed, label, key)` returns a
`particles::Rng` (SplitMix64) for one use:

- `label` says what the stream is for (`"enemies"`), `key` says which one (a room and a group).
- The label is hashed with FNV-1a, then the seed, the hash and the key are mixed through
  SplitMix steps. The same three values always give the same stream; a different seed, label
  or key gives a different one, and for one seed and label two keys never start in the same state.
- The first numbers of three streams are pinned in `src/seed/tests.rs`: if the mixing changes,
  the test fails loudly, because every run's enemies would change with it.

## A seed as text

`format_seed` writes 16 lowercase hex digits with leading zeros (`00000000000000ab`).
`parse_seed` reads 1 to 16 hex digits in either case with an optional `0x`; an empty text, a
bare `0x`, a character that is not a hex digit (a space too) and more than 16 digits are each
refused with a message that says why.
