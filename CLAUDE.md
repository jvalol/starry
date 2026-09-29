# starry

A sliding tile puzzle of Van Gogh's Starry Night, in tiles with thickness. The
sixth game on `blitzkit` and the third in 3D. The engine owns the window,
rendering, input, and the meshes and textures the tiles are built from. The
dependency is the published crate, overridden by the engine checkout at
`../blitzkit` when built inside this project folder.

## Build and test

Requires Rust 1.87 or newer, the engine's MSRV.

```
cargo build
cargo test
cargo run
cargo clippy
cargo fmt
```

## How work happens here

Behavior changes are spec driven:

1. **Write the spec first.** Copy `specs/TEMPLATE.md` to `specs/NNNN-short-name.md`
   and fill it in. Say what the game does, not how the code does it.
2. **Make the acceptance criteria testable.** Each one names the test that proves
   it, or goes under "Verified by hand" when it needs a window.
3. **Write the tests, then the code.** `cargo test` passes before a commit.
4. **Update the spec when behavior changes.** A spec that disagrees with the game
   is a bug in the spec.

## Layout

- `src/main.rs` — nothing yet. The game is a spec and a rulebook so far.
- `src/board.rs` — the tiles, the gap, sliding, lifting, and what can be
  finished. None of it needs a window, which is why it came first.

## The parity rules

Two of them, and both are easy to get wrong by reasoning about them.

An arrangement can be finished by sliding when its inversion count plus the
gap's row counted from the bottom is odd. That is the four-wide rule and it does
not hold for other widths.

A lift leaves the board finishable exactly when the gap and the lifted tile are
an odd number of cells apart, counted along rows and columns. It does not depend
on the arrangement at all. A slide is the one-cell case, which is why sliding
can never strand anyone.

The first draft of spec 0001 said row distance decided the second rule. It does
not: row distance predicts 112 of the 240 pairs of cells, which is worse than a
coin. It was caught by computing all 240 rather than by thinking harder, and
`the_warning_reads_only_the_positions` is that computation kept as a test. Do
the same with the next parity claim.
