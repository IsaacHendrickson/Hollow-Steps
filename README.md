# Hollow Steps

Hollow Steps is a small Rust 2D action-platformer prototype inspired by the
feel of exploration-focused games such as Hollow Knight. The first vertical
slice is intentionally narrow: one player can walk, jump, and land inside a
single bounded room.

## Run it

Install Rust, then run:

```text
cargo run
```

Useful checks while developing:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Current controls

- `A`/`D` or the left/right arrow keys: move
- `Space` or `Z`: jump

See [`AGENTS.md`](AGENTS.md) for the project conventions and the recommended
workflow for future changes.
