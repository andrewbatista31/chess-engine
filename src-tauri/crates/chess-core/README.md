# chess-core

Pure chess rules library: bitboard board representation, legal move generation,
FEN/PGN parsing and serialization, Zobrist hashing.

No I/O, no async, no engine, no GUI.

## Usage

```rust
use chess_core::prelude::*;

let pos = Position::starting();
for m in legal_moves(&pos) {
    println!("{:?}", m);
}
```

## Correctness

Move generation is validated against the standard perft suite (Chess Programming Wiki
positions 1-6). Run with:

```
cargo test -p chess-core --test perft_suite --release
```
