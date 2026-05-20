# Chess Core Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `chess-core` Rust crate — a pure, well-tested chess rules library with bitboard representation, FEN/PGN, legal move generation, and verified correctness via perft.

**Architecture:** Cargo workspace rooted at the repo, with crates living under `src-tauri/crates/`. This plan only fills out `chess-core`; later plans add Tauri scaffolding alongside it. `chess-core` has zero engine, GUI, or I/O dependencies — it's a pure library.

**Tech Stack:** Rust 1.75+ (edition 2021), Cargo workspaces. Testing with built-in `#[test]`. `arrayvec` for stack-allocated `MoveList`. No async, no I/O.

---

## File Structure

```
chess-engine/
├── Cargo.toml                          ← workspace root
├── src-tauri/
│   └── crates/
│       └── chess-core/
│           ├── Cargo.toml
│           └── src/
│               ├── lib.rs              ← re-exports
│               ├── types.rs            ← Square, Color, Piece, File, Rank
│               ├── bitboard.rs         ← Bitboard type + ops
│               ├── position.rs         ← Position struct + accessors
│               ├── fen.rs              ← parse/serialize FEN
│               ├── moves.rs            ← Move type + flags
│               ├── attacks.rs          ← precomputed attack tables
│               ├── movegen.rs          ← legal move generation
│               ├── make_move.rs        ← make_move, unmake_move, MoveUndo
│               ├── zobrist.rs          ← Zobrist hashing
│               ├── outcome.rs          ← checkmate, stalemate, draws
│               ├── perft.rs            ← perft function for testing
│               └── pgn.rs              ← parse/serialize PGN
└── tests/
    └── (per-crate tests under src-tauri/crates/chess-core/tests/)
```

Each `.rs` file has one clear responsibility. Splitting `make_move.rs` out of `position.rs` keeps the Position type's accessors short and isolates the mutation machinery.

---

### Task 1: Workspace scaffold + chess-core crate

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `src-tauri/crates/chess-core/Cargo.toml`
- Create: `src-tauri/crates/chess-core/src/lib.rs`

- [ ] **Step 1: Create workspace root `Cargo.toml`**

```toml
[workspace]
resolver = "2"
members = ["src-tauri/crates/*"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
license = "MIT"

[workspace.dependencies]
arrayvec = "0.7"
```

- [ ] **Step 2: Create the `chess-core` crate manifest**

`src-tauri/crates/chess-core/Cargo.toml`:

```toml
[package]
name = "chess-core"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
arrayvec = { workspace = true }

[dev-dependencies]
```

- [ ] **Step 3: Create `lib.rs` with module stubs**

`src-tauri/crates/chess-core/src/lib.rs`:

```rust
//! chess-core: pure chess rules library.

pub mod types;
pub mod bitboard;
pub mod position;
pub mod fen;
pub mod moves;
pub mod attacks;
pub mod movegen;
pub mod make_move;
pub mod zobrist;
pub mod outcome;
pub mod perft;
pub mod pgn;
```

Create each referenced module as an empty file (`types.rs`, `bitboard.rs`, etc.) so the crate compiles.

- [ ] **Step 4: Verify the workspace builds**

Run: `cargo build`
Expected: `Compiling chess-core v0.1.0 ... Finished`

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src-tauri/crates/chess-core/
git commit -m "chore: scaffold cargo workspace and chess-core crate"
```

---

### Task 2: Core types (Square, Color, Piece, File, Rank)

**Files:**
- Modify: `src-tauri/crates/chess-core/src/types.rs`

- [ ] **Step 1: Write the failing test**

Append to `src-tauri/crates/chess-core/src/types.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_from_file_rank_and_back() {
        let sq = Square::new(File::E, Rank::Four);
        assert_eq!(sq.file(), File::E);
        assert_eq!(sq.rank(), Rank::Four);
        assert_eq!(sq.index(), 28); // (rank 4 = index 3) * 8 + (file E = index 4)
    }

    #[test]
    fn square_round_trip_for_all_64() {
        for i in 0..64u8 {
            let sq = Square::from_index(i).unwrap();
            assert_eq!(sq.index(), i);
        }
    }

    #[test]
    fn color_flip() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chess-core types::`
Expected: FAIL — `Square`, `File`, `Rank`, `Color` not defined.

- [ ] **Step 3: Implement the types**

Prepend to `src-tauri/crates/chess-core/src/types.rs`:

```rust
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color { White = 0, Black = 1 }

impl Color {
    pub fn flip(self) -> Color {
        match self { Color::White => Color::Black, Color::Black => Color::White }
    }
    pub fn index(self) -> usize { self as usize }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind { Pawn = 0, Knight = 1, Bishop = 2, Rook = 3, Queen = 4, King = 5 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece { pub color: Color, pub kind: PieceKind }

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum File { A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7 }

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rank { One=0, Two=1, Three=2, Four=3, Five=4, Six=5, Seven=6, Eight=7 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(u8); // 0..64, index = rank * 8 + file

impl Square {
    pub fn new(file: File, rank: Rank) -> Self {
        Self((rank as u8) * 8 + file as u8)
    }
    pub fn from_index(i: u8) -> Option<Self> {
        if i < 64 { Some(Self(i)) } else { None }
    }
    pub fn index(self) -> u8 { self.0 }
    pub fn file(self) -> File {
        // SAFETY: 0..64 guarantees file 0..8
        unsafe { std::mem::transmute(self.0 % 8) }
    }
    pub fn rank(self) -> Rank {
        unsafe { std::mem::transmute(self.0 / 8) }
    }
}

impl File {
    pub fn from_index(i: u8) -> Option<Self> {
        if i < 8 { Some(unsafe { std::mem::transmute(i) }) } else { None }
    }
}
impl Rank {
    pub fn from_index(i: u8) -> Option<Self> {
        if i < 8 { Some(unsafe { std::mem::transmute(i) }) } else { None }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chess-core types::`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/types.rs
git commit -m "feat(chess-core): add Square, Color, Piece, File, Rank types"
```

---

### Task 3: Bitboard type + operations

**Files:**
- Modify: `src-tauri/crates/chess-core/src/bitboard.rs`

- [ ] **Step 1: Write the failing tests**

In `src-tauri/crates/chess-core/src/bitboard.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{File, Rank, Square};

    #[test]
    fn empty_and_full() {
        assert_eq!(Bitboard::EMPTY.0, 0);
        assert_eq!(Bitboard::FULL.0, u64::MAX);
    }

    #[test]
    fn set_and_get_square() {
        let bb = Bitboard::from_square(Square::new(File::E, Rank::Four));
        assert!(bb.contains(Square::new(File::E, Rank::Four)));
        assert!(!bb.contains(Square::new(File::A, Rank::One)));
        assert_eq!(bb.count(), 1);
    }

    #[test]
    fn pop_lsb_iterates_all_squares() {
        let mut bb = Bitboard(0b1011);
        let mut squares = vec![];
        while let Some(sq) = bb.pop_lsb() {
            squares.push(sq.index());
        }
        assert_eq!(squares, vec![0, 1, 3]);
    }

    #[test]
    fn bitwise_ops() {
        let a = Bitboard(0b1100);
        let b = Bitboard(0b1010);
        assert_eq!((a & b).0, 0b1000);
        assert_eq!((a | b).0, 0b1110);
        assert_eq!((a ^ b).0, 0b0110);
        assert_eq!((!a).0, !0b1100u64);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core bitboard::`
Expected: FAIL — `Bitboard` not defined.

- [ ] **Step 3: Implement Bitboard**

Prepend to `src-tauri/crates/chess-core/src/bitboard.rs`:

```rust
use crate::types::Square;
use std::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL:  Bitboard = Bitboard(u64::MAX);

    pub fn from_square(sq: Square) -> Self { Bitboard(1u64 << sq.index()) }
    pub fn contains(self, sq: Square) -> bool { self.0 & (1u64 << sq.index()) != 0 }
    pub fn count(self) -> u32 { self.0.count_ones() }
    pub fn is_empty(self) -> bool { self.0 == 0 }

    /// Pop and return the least significant bit's square, or None if empty.
    pub fn pop_lsb(&mut self) -> Option<Square> {
        if self.0 == 0 { return None; }
        let idx = self.0.trailing_zeros() as u8;
        self.0 &= self.0 - 1;
        Square::from_index(idx)
    }
}

impl BitAnd for Bitboard { type Output = Self; fn bitand(self, r: Self) -> Self { Bitboard(self.0 & r.0) } }
impl BitOr  for Bitboard { type Output = Self; fn bitor (self, r: Self) -> Self { Bitboard(self.0 | r.0) } }
impl BitXor for Bitboard { type Output = Self; fn bitxor(self, r: Self) -> Self { Bitboard(self.0 ^ r.0) } }
impl Not    for Bitboard { type Output = Self; fn not(self) -> Self { Bitboard(!self.0) } }
impl Shl<u32> for Bitboard { type Output = Self; fn shl(self, s: u32) -> Self { Bitboard(self.0 << s) } }
impl Shr<u32> for Bitboard { type Output = Self; fn shr(self, s: u32) -> Self { Bitboard(self.0 >> s) } }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chess-core bitboard::`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/bitboard.rs
git commit -m "feat(chess-core): add Bitboard type with set/pop/bitwise ops"
```

---

### Task 4: Position struct + initial position

**Files:**
- Modify: `src-tauri/crates/chess-core/src/position.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, File, PieceKind, Rank, Square};

    #[test]
    fn starting_position_has_32_pieces() {
        let pos = Position::starting();
        assert_eq!(pos.all_pieces().count(), 32);
        assert_eq!(pos.side_to_move, Color::White);
    }

    #[test]
    fn starting_position_white_king_on_e1() {
        let pos = Position::starting();
        let sq = Square::new(File::E, Rank::One);
        let piece = pos.piece_at(sq).expect("king must exist");
        assert_eq!(piece.color, Color::White);
        assert_eq!(piece.kind, PieceKind::King);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core position::`
Expected: FAIL — `Position` not defined.

- [ ] **Step 3: Implement Position**

`src-tauri/crates/chess-core/src/position.rs`:

```rust
use crate::bitboard::Bitboard;
use crate::types::{Color, Piece, PieceKind, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CastlingRights {
    pub white_king_side: bool,
    pub white_queen_side: bool,
    pub black_king_side: bool,
    pub black_queen_side: bool,
}

impl CastlingRights {
    pub const ALL: Self = CastlingRights {
        white_king_side: true, white_queen_side: true,
        black_king_side: true, black_queen_side: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// Indexed by [color][piece_kind].
    pub bitboards: [[Bitboard; 6]; 2],
    pub side_to_move: Color,
    pub castling: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmove_clock: u8,
    pub fullmove_number: u16,
    pub zobrist_hash: u64, // populated by Task 14
}

impl Position {
    pub fn empty() -> Self {
        Self {
            bitboards: [[Bitboard::EMPTY; 6]; 2],
            side_to_move: Color::White,
            castling: CastlingRights::default(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            zobrist_hash: 0,
        }
    }

    pub fn starting() -> Self {
        // Use the standard starting FEN once Task 5 implements parsing.
        // Until then, build by hand:
        let mut p = Self::empty();
        use crate::types::{File, PieceKind::*, Rank};
        let back_rank = [Rook, Knight, Bishop, Queen, King, Bishop, Knight, Rook];
        for (i, kind) in back_rank.iter().enumerate() {
            let file = File::from_index(i as u8).unwrap();
            p.set_piece(Square::new(file, Rank::One),   Piece { color: Color::White, kind: *kind });
            p.set_piece(Square::new(file, Rank::Eight), Piece { color: Color::Black, kind: *kind });
            p.set_piece(Square::new(file, Rank::Two),   Piece { color: Color::White, kind: Pawn });
            p.set_piece(Square::new(file, Rank::Seven), Piece { color: Color::Black, kind: Pawn });
        }
        p.castling = CastlingRights::ALL;
        p
    }

    pub fn set_piece(&mut self, sq: Square, piece: Piece) {
        self.bitboards[piece.color.index()][piece.kind as usize] =
            self.bitboards[piece.color.index()][piece.kind as usize] | Bitboard::from_square(sq);
    }

    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        for c in [Color::White, Color::Black] {
            for k in 0..6 {
                if self.bitboards[c.index()][k].contains(sq) {
                    let kind = match k {
                        0 => PieceKind::Pawn, 1 => PieceKind::Knight, 2 => PieceKind::Bishop,
                        3 => PieceKind::Rook, 4 => PieceKind::Queen, 5 => PieceKind::King,
                        _ => unreachable!(),
                    };
                    return Some(Piece { color: c, kind });
                }
            }
        }
        None
    }

    pub fn occupied_by(&self, color: Color) -> Bitboard {
        let bbs = &self.bitboards[color.index()];
        bbs[0] | bbs[1] | bbs[2] | bbs[3] | bbs[4] | bbs[5]
    }

    pub fn all_pieces(&self) -> Bitboard {
        self.occupied_by(Color::White) | self.occupied_by(Color::Black)
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p chess-core position::`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/position.rs
git commit -m "feat(chess-core): add Position struct + starting position"
```

---

### Task 5: FEN parser

**Files:**
- Modify: `src-tauri/crates/chess-core/src/fen.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;
    use crate::types::Color;

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn parses_starting_fen() {
        let pos = parse_fen(START_FEN).unwrap();
        assert_eq!(pos.side_to_move, Color::White);
        assert_eq!(pos.all_pieces().count(), 32);
        assert!(pos.castling.white_king_side);
        assert!(pos.castling.black_queen_side);
        assert_eq!(pos.halfmove_clock, 0);
        assert_eq!(pos.fullmove_number, 1);
    }

    #[test]
    fn parses_kiwipete() {
        // Standard perft position 2 (the "kiwipete" position).
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let pos = parse_fen(fen).unwrap();
        assert_eq!(pos.all_pieces().count(), 32);
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse_fen("not a fen").is_err());
        assert!(parse_fen("8/8/8/8/8/8/8/8 w - - 0 1").is_ok()); // empty board OK
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core fen::`
Expected: FAIL — `parse_fen` not defined.

- [ ] **Step 3: Implement the parser**

```rust
use crate::position::{CastlingRights, Position};
use crate::types::{Color, File, Piece, PieceKind, Rank, Square};

#[derive(Debug)]
pub enum FenError {
    BadField(&'static str),
    BadChar(char),
    WrongFieldCount,
}

pub fn parse_fen(s: &str) -> Result<Position, FenError> {
    let mut fields = s.split_ascii_whitespace();
    let board    = fields.next().ok_or(FenError::WrongFieldCount)?;
    let stm      = fields.next().ok_or(FenError::WrongFieldCount)?;
    let castling = fields.next().ok_or(FenError::WrongFieldCount)?;
    let ep       = fields.next().ok_or(FenError::WrongFieldCount)?;
    let hmc      = fields.next().ok_or(FenError::WrongFieldCount)?;
    let fmn      = fields.next().ok_or(FenError::WrongFieldCount)?;

    let mut pos = Position::empty();

    // Board: ranks 8..1, files A..H.
    let mut rank = 7i32;
    for row in board.split('/') {
        if rank < 0 { return Err(FenError::BadField("board")); }
        let mut file = 0i32;
        for ch in row.chars() {
            if ch.is_ascii_digit() {
                file += ch.to_digit(10).unwrap() as i32;
            } else {
                if file >= 8 { return Err(FenError::BadField("board")); }
                let (color, kind) = match ch {
                    'P' => (Color::White, PieceKind::Pawn),
                    'N' => (Color::White, PieceKind::Knight),
                    'B' => (Color::White, PieceKind::Bishop),
                    'R' => (Color::White, PieceKind::Rook),
                    'Q' => (Color::White, PieceKind::Queen),
                    'K' => (Color::White, PieceKind::King),
                    'p' => (Color::Black, PieceKind::Pawn),
                    'n' => (Color::Black, PieceKind::Knight),
                    'b' => (Color::Black, PieceKind::Bishop),
                    'r' => (Color::Black, PieceKind::Rook),
                    'q' => (Color::Black, PieceKind::Queen),
                    'k' => (Color::Black, PieceKind::King),
                    c => return Err(FenError::BadChar(c)),
                };
                let sq = Square::new(
                    File::from_index(file as u8).ok_or(FenError::BadField("board"))?,
                    Rank::from_index(rank as u8).ok_or(FenError::BadField("board"))?,
                );
                pos.set_piece(sq, Piece { color, kind });
                file += 1;
            }
        }
        if file != 8 { return Err(FenError::BadField("board")); }
        rank -= 1;
    }
    if rank != -1 { return Err(FenError::BadField("board")); }

    pos.side_to_move = match stm {
        "w" => Color::White,
        "b" => Color::Black,
        _ => return Err(FenError::BadField("side_to_move")),
    };

    pos.castling = CastlingRights::default();
    if castling != "-" {
        for ch in castling.chars() {
            match ch {
                'K' => pos.castling.white_king_side  = true,
                'Q' => pos.castling.white_queen_side = true,
                'k' => pos.castling.black_king_side  = true,
                'q' => pos.castling.black_queen_side = true,
                _ => return Err(FenError::BadField("castling")),
            }
        }
    }

    pos.en_passant = if ep == "-" { None } else { Some(parse_square(ep)?) };
    pos.halfmove_clock  = hmc.parse().map_err(|_| FenError::BadField("halfmove_clock"))?;
    pos.fullmove_number = fmn.parse().map_err(|_| FenError::BadField("fullmove_number"))?;

    Ok(pos)
}

fn parse_square(s: &str) -> Result<Square, FenError> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 { return Err(FenError::BadField("square")); }
    let file = File::from_index(bytes[0].wrapping_sub(b'a')).ok_or(FenError::BadField("square"))?;
    let rank = Rank::from_index(bytes[1].wrapping_sub(b'1')).ok_or(FenError::BadField("square"))?;
    Ok(Square::new(file, rank))
}
```

- [ ] **Step 4: Update `Position::starting` to use FEN**

In `position.rs`, replace the manual loop in `starting()`:

```rust
pub fn starting() -> Self {
    crate::fen::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
        .expect("starting FEN is valid")
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p chess-core`
Expected: all previous tests still pass, plus 3 new FEN tests pass.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/chess-core/src/
git commit -m "feat(chess-core): FEN parser + reuse it for Position::starting"
```

---

### Task 6: FEN serializer + round-trip property test

**Files:**
- Modify: `src-tauri/crates/chess-core/src/fen.rs`
- Modify: `src-tauri/crates/chess-core/src/position.rs` (add `to_fen` shim)

- [ ] **Step 1: Write the failing test**

Append to `fen.rs` tests:

```rust
#[test]
fn fen_round_trip() {
    let fens = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
    ];
    for fen in fens {
        let pos = parse_fen(fen).unwrap();
        assert_eq!(serialize_fen(&pos), fen, "round trip failed for {fen}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chess-core fen::tests::fen_round_trip`
Expected: FAIL — `serialize_fen` not defined.

- [ ] **Step 3: Implement the serializer**

Append to `fen.rs`:

```rust
pub fn serialize_fen(pos: &Position) -> String {
    let mut out = String::with_capacity(80);

    for rank_idx in (0..8u8).rev() {
        let mut empty = 0;
        for file_idx in 0..8u8 {
            let sq = Square::new(
                File::from_index(file_idx).unwrap(),
                Rank::from_index(rank_idx).unwrap(),
            );
            match pos.piece_at(sq) {
                None => empty += 1,
                Some(p) => {
                    if empty > 0 {
                        out.push(char::from_digit(empty, 10).unwrap());
                        empty = 0;
                    }
                    out.push(piece_char(p));
                }
            }
        }
        if empty > 0 { out.push(char::from_digit(empty, 10).unwrap()); }
        if rank_idx > 0 { out.push('/'); }
    }

    out.push(' ');
    out.push(if pos.side_to_move == Color::White { 'w' } else { 'b' });

    out.push(' ');
    let c = pos.castling;
    if !(c.white_king_side || c.white_queen_side || c.black_king_side || c.black_queen_side) {
        out.push('-');
    } else {
        if c.white_king_side  { out.push('K'); }
        if c.white_queen_side { out.push('Q'); }
        if c.black_king_side  { out.push('k'); }
        if c.black_queen_side { out.push('q'); }
    }

    out.push(' ');
    match pos.en_passant {
        None => out.push('-'),
        Some(sq) => out.push_str(&square_to_str(sq)),
    }

    out.push(' ');
    out.push_str(&pos.halfmove_clock.to_string());
    out.push(' ');
    out.push_str(&pos.fullmove_number.to_string());

    out
}

fn piece_char(p: Piece) -> char {
    let c = match p.kind {
        PieceKind::Pawn=>'p', PieceKind::Knight=>'n', PieceKind::Bishop=>'b',
        PieceKind::Rook=>'r', PieceKind::Queen=>'q',  PieceKind::King=>'k',
    };
    if p.color == Color::White { c.to_ascii_uppercase() } else { c }
}

fn square_to_str(sq: Square) -> String {
    let mut s = String::with_capacity(2);
    s.push((b'a' + sq.file() as u8) as char);
    s.push((b'1' + sq.rank() as u8) as char);
    s
}
```

- [ ] **Step 4: Add `to_fen` convenience on `Position`**

In `position.rs`:

```rust
impl Position {
    pub fn to_fen(&self) -> String { crate::fen::serialize_fen(self) }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test -p chess-core`
Expected: all passing.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/crates/chess-core/src/
git commit -m "feat(chess-core): FEN serializer + round-trip tests"
```

---

### Task 7: Move type + flags

**Files:**
- Modify: `src-tauri/crates/chess-core/src/moves.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{File, Rank, Square};

    #[test]
    fn pack_and_unpack_quiet_move() {
        let from = Square::new(File::E, Rank::Two);
        let to   = Square::new(File::E, Rank::Four);
        let m = Move::new(from, to, MoveFlag::Quiet);
        assert_eq!(m.from(), from);
        assert_eq!(m.to(), to);
        assert_eq!(m.flag(), MoveFlag::Quiet);
    }

    #[test]
    fn promotion_flags_distinct() {
        let from = Square::new(File::A, Rank::Seven);
        let to   = Square::new(File::A, Rank::Eight);
        let mq = Move::new(from, to, MoveFlag::PromoQueen);
        let mn = Move::new(from, to, MoveFlag::PromoKnight);
        assert_ne!(mq.0, mn.0);
        assert_eq!(mq.flag(), MoveFlag::PromoQueen);
        assert_eq!(mn.flag(), MoveFlag::PromoKnight);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core moves::`
Expected: FAIL.

- [ ] **Step 3: Implement Move**

```rust
use crate::types::Square;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag {
    Quiet           = 0,
    DoublePawnPush  = 1,
    KingCastle      = 2,
    QueenCastle     = 3,
    Capture         = 4,
    EnPassant       = 5,
    PromoKnight     = 8,
    PromoBishop     = 9,
    PromoRook       = 10,
    PromoQueen      = 11,
    PromoCaptureN   = 12,
    PromoCaptureB   = 13,
    PromoCaptureR   = 14,
    PromoCaptureQ   = 15,
}

impl MoveFlag {
    pub fn is_capture(self) -> bool {
        matches!(self,
            MoveFlag::Capture | MoveFlag::EnPassant |
            MoveFlag::PromoCaptureN | MoveFlag::PromoCaptureB |
            MoveFlag::PromoCaptureR | MoveFlag::PromoCaptureQ
        )
    }
    pub fn is_promotion(self) -> bool { (self as u8) & 0b1000 != 0 }
}

/// Packed move: bits 0-5 = from, 6-11 = to, 12-15 = flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(pub u16);

impl Move {
    pub fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        Move(
            (from.index() as u16) |
            ((to.index() as u16) << 6) |
            ((flag as u16) << 12)
        )
    }
    pub fn from(self) -> Square { Square::from_index((self.0 & 0x3F) as u8).unwrap() }
    pub fn to  (self) -> Square { Square::from_index(((self.0 >> 6) & 0x3F) as u8).unwrap() }
    pub fn flag(self) -> MoveFlag {
        // SAFETY: flag values 0..16 are all defined
        unsafe { std::mem::transmute(((self.0 >> 12) & 0xF) as u8) }
    }
}

pub type MoveList = arrayvec::ArrayVec<Move, 256>;
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core moves::`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/moves.rs
git commit -m "feat(chess-core): Move type with packed encoding"
```

---

### Task 8: Precomputed attack tables — non-sliders

**Files:**
- Modify: `src-tauri/crates/chess-core/src/attacks.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitboard::Bitboard;
    use crate::types::{Color, File, Rank, Square};

    #[test]
    fn knight_attacks_from_d4_hits_8_squares() {
        let d4 = Square::new(File::D, Rank::Four);
        assert_eq!(knight_attacks(d4).count(), 8);
    }

    #[test]
    fn knight_attacks_from_a1_hits_2_squares() {
        let a1 = Square::new(File::A, Rank::One);
        assert_eq!(knight_attacks(a1).count(), 2);
    }

    #[test]
    fn king_attacks_from_e4_hits_8_squares() {
        let e4 = Square::new(File::E, Rank::Four);
        assert_eq!(king_attacks(e4).count(), 8);
    }

    #[test]
    fn white_pawn_attacks_from_e2_hits_d3_and_f3() {
        let e2 = Square::new(File::E, Rank::Two);
        let attacks = pawn_attacks(Color::White, e2);
        assert!(attacks.contains(Square::new(File::D, Rank::Three)));
        assert!(attacks.contains(Square::new(File::F, Rank::Three)));
        assert_eq!(attacks.count(), 2);
    }

    #[test]
    fn pawn_attack_at_corner() {
        let a2 = Square::new(File::A, Rank::Two);
        let attacks = pawn_attacks(Color::White, a2);
        assert_eq!(attacks.count(), 1);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core attacks::`
Expected: FAIL.

- [ ] **Step 3: Implement attack tables**

```rust
use crate::bitboard::Bitboard;
use crate::types::{Color, Square};

const FILE_A: u64 = 0x0101010101010101;
const FILE_H: u64 = 0x8080808080808080;
const FILE_AB: u64 = FILE_A | (FILE_A << 1);
const FILE_GH: u64 = FILE_H | (FILE_H >> 1);

pub fn knight_attacks(sq: Square) -> Bitboard {
    let b = 1u64 << sq.index();
    let attacks =
        ((b << 17) & !FILE_A)
      | ((b << 15) & !FILE_H)
      | ((b << 10) & !FILE_AB)
      | ((b <<  6) & !FILE_GH)
      | ((b >> 17) & !FILE_H)
      | ((b >> 15) & !FILE_A)
      | ((b >> 10) & !FILE_GH)
      | ((b >>  6) & !FILE_AB);
    Bitboard(attacks)
}

pub fn king_attacks(sq: Square) -> Bitboard {
    let b = 1u64 << sq.index();
    let attacks =
        ((b << 8))
      | ((b >> 8))
      | ((b << 1) & !FILE_A)
      | ((b >> 1) & !FILE_H)
      | ((b << 9) & !FILE_A)
      | ((b << 7) & !FILE_H)
      | ((b >> 7) & !FILE_A)
      | ((b >> 9) & !FILE_H);
    Bitboard(attacks)
}

pub fn pawn_attacks(color: Color, sq: Square) -> Bitboard {
    let b = 1u64 << sq.index();
    let attacks = match color {
        Color::White => ((b << 9) & !FILE_A) | ((b << 7) & !FILE_H),
        Color::Black => ((b >> 7) & !FILE_A) | ((b >> 9) & !FILE_H),
    };
    Bitboard(attacks)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core attacks::`
Expected: 5 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/attacks.rs
git commit -m "feat(chess-core): knight, king, pawn attack generation"
```

---

### Task 9: Sliding-piece attack generation (ray-based)

**Files:**
- Modify: `src-tauri/crates/chess-core/src/attacks.rs`

Magic bitboards are the standard optimization, but a ray-based implementation is much simpler and correct. We'll start with this; profiling can drive an upgrade later (see spec "Risks & open questions").

- [ ] **Step 1: Write the failing tests**

Append to `attacks.rs` tests:

```rust
#[test]
fn rook_on_a1_empty_board_sees_a_file_and_rank_1() {
    let a1 = Square::new(File::A, Rank::One);
    let attacks = rook_attacks(a1, Bitboard::EMPTY);
    // 7 squares up file A + 7 squares along rank 1 = 14
    assert_eq!(attacks.count(), 14);
}

#[test]
fn rook_attack_blocked_by_own_piece_includes_blocker() {
    let a1 = Square::new(File::A, Rank::One);
    let blocker = Bitboard::from_square(Square::new(File::A, Rank::Four));
    let attacks = rook_attacks(a1, blocker);
    // Up file A: a2, a3, a4 (3). Along rank 1: 7 squares. Total 10.
    assert_eq!(attacks.count(), 10);
    assert!(attacks.contains(Square::new(File::A, Rank::Four)));
    assert!(!attacks.contains(Square::new(File::A, Rank::Five)));
}

#[test]
fn bishop_on_d4_empty_board_hits_13_squares() {
    let d4 = Square::new(File::D, Rank::Four);
    assert_eq!(bishop_attacks(d4, Bitboard::EMPTY).count(), 13);
}

#[test]
fn queen_combines_rook_and_bishop() {
    let d4 = Square::new(File::D, Rank::Four);
    let combined = (rook_attacks(d4, Bitboard::EMPTY) | bishop_attacks(d4, Bitboard::EMPTY)).count();
    assert_eq!(queen_attacks(d4, Bitboard::EMPTY).count(), combined);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core attacks::tests::rook`
Expected: FAIL.

- [ ] **Step 3: Implement sliding attacks**

Append to `attacks.rs`:

```rust
const ROOK_DIRS:   [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
const BISHOP_DIRS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

fn ray_attacks(sq: Square, occ: Bitboard, dirs: &[(i32, i32)]) -> Bitboard {
    let mut bb = 0u64;
    let f0 = (sq.index() % 8) as i32;
    let r0 = (sq.index() / 8) as i32;
    for &(df, dr) in dirs {
        let (mut f, mut r) = (f0 + df, r0 + dr);
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let idx = (r * 8 + f) as u8;
            let bit = 1u64 << idx;
            bb |= bit;
            if occ.0 & bit != 0 { break; }
            f += df; r += dr;
        }
    }
    Bitboard(bb)
}

pub fn rook_attacks  (sq: Square, occ: Bitboard) -> Bitboard { ray_attacks(sq, occ, &ROOK_DIRS) }
pub fn bishop_attacks(sq: Square, occ: Bitboard) -> Bitboard { ray_attacks(sq, occ, &BISHOP_DIRS) }
pub fn queen_attacks (sq: Square, occ: Bitboard) -> Bitboard {
    rook_attacks(sq, occ) | bishop_attacks(sq, occ)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core attacks::`
Expected: 9 total passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/attacks.rs
git commit -m "feat(chess-core): sliding-piece attacks (rook/bishop/queen, ray-based)"
```

---

### Task 10: Pseudo-legal move generation

**Files:**
- Modify: `src-tauri/crates/chess-core/src/movegen.rs`

"Pseudo-legal" = legal except possibly leaving own king in check. Task 12 adds legality filtering. Special moves (castling, en passant, promotions) handled in Task 11.

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn starting_position_has_20_pseudo_legal_moves() {
        let pos = Position::starting();
        let moves = pseudo_legal_moves(&pos);
        assert_eq!(moves.len(), 20);
        // 16 pawn moves (8 single + 8 double) + 4 knight moves
    }

    #[test]
    fn black_to_move_starting_position_has_20() {
        let mut pos = Position::starting();
        pos.side_to_move = crate::types::Color::Black;
        assert_eq!(pseudo_legal_moves(&pos).len(), 20);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core movegen::`
Expected: FAIL.

- [ ] **Step 3: Implement pseudo-legal generation**

```rust
use crate::attacks::*;
use crate::bitboard::Bitboard;
use crate::moves::{Move, MoveFlag, MoveList};
use crate::position::Position;
use crate::types::{Color, PieceKind, Square};

pub fn pseudo_legal_moves(pos: &Position) -> MoveList {
    let mut out = MoveList::new();
    let us   = pos.side_to_move;
    let them = us.flip();
    let our_pieces   = pos.occupied_by(us);
    let their_pieces = pos.occupied_by(them);
    let occ = our_pieces | their_pieces;

    gen_pawn_moves(pos, us, occ, their_pieces, &mut out);
    gen_piece_moves(pos, us, PieceKind::Knight, our_pieces, their_pieces, occ, &mut out, knight_attacks_wrap);
    gen_piece_moves(pos, us, PieceKind::Bishop, our_pieces, their_pieces, occ, &mut out, |sq, o| bishop_attacks(sq, o));
    gen_piece_moves(pos, us, PieceKind::Rook,   our_pieces, their_pieces, occ, &mut out, |sq, o| rook_attacks(sq, o));
    gen_piece_moves(pos, us, PieceKind::Queen,  our_pieces, their_pieces, occ, &mut out, |sq, o| queen_attacks(sq, o));
    gen_piece_moves(pos, us, PieceKind::King,   our_pieces, their_pieces, occ, &mut out, king_attacks_wrap);

    out
}

fn knight_attacks_wrap(sq: Square, _o: Bitboard) -> Bitboard { knight_attacks(sq) }
fn king_attacks_wrap  (sq: Square, _o: Bitboard) -> Bitboard { king_attacks(sq) }

fn gen_piece_moves<F>(
    pos: &Position,
    us: Color,
    kind: PieceKind,
    our_pieces: Bitboard,
    their_pieces: Bitboard,
    occ: Bitboard,
    out: &mut MoveList,
    attacks_fn: F,
) where F: Fn(Square, Bitboard) -> Bitboard {
    let mut bb = pos.bitboards[us.index()][kind as usize];
    while let Some(from) = bb.pop_lsb() {
        let mut targets = attacks_fn(from, occ) & !our_pieces;
        while let Some(to) = targets.pop_lsb() {
            let flag = if their_pieces.contains(to) { MoveFlag::Capture } else { MoveFlag::Quiet };
            out.push(Move::new(from, to, flag));
        }
    }
}

fn gen_pawn_moves(pos: &Position, us: Color, occ: Bitboard, their_pieces: Bitboard, out: &mut MoveList) {
    use crate::types::{File, Rank};
    let mut pawns = pos.bitboards[us.index()][PieceKind::Pawn as usize];
    let (push_dir, double_rank, promo_rank) = match us {
        Color::White => ( 1i32, Rank::Two,   Rank::Eight),
        Color::Black => (-1i32, Rank::Seven, Rank::One),
    };

    while let Some(from) = pawns.pop_lsb() {
        let r = from.rank() as i32;
        let f = from.file() as i32;

        // single push
        let nr = r + push_dir;
        if (0..8).contains(&nr) {
            let to = Square::new(File::from_index(f as u8).unwrap(), Rank::from_index(nr as u8).unwrap());
            if !occ.contains(to) {
                if to.rank() == promo_rank {
                    for promo in [MoveFlag::PromoQueen, MoveFlag::PromoRook,
                                  MoveFlag::PromoBishop, MoveFlag::PromoKnight] {
                        out.push(Move::new(from, to, promo));
                    }
                } else {
                    out.push(Move::new(from, to, MoveFlag::Quiet));
                    // double push
                    if from.rank() == double_rank {
                        let nr2 = r + 2 * push_dir;
                        let to2 = Square::new(File::from_index(f as u8).unwrap(), Rank::from_index(nr2 as u8).unwrap());
                        if !occ.contains(to2) {
                            out.push(Move::new(from, to2, MoveFlag::DoublePawnPush));
                        }
                    }
                }
            }
        }

        // captures (diagonals)
        for df in [-1, 1] {
            let nf = f + df;
            if !(0..8).contains(&nf) || !(0..8).contains(&nr) { continue; }
            let to = Square::new(File::from_index(nf as u8).unwrap(), Rank::from_index(nr as u8).unwrap());
            if their_pieces.contains(to) {
                if to.rank() == promo_rank {
                    for promo in [MoveFlag::PromoCaptureQ, MoveFlag::PromoCaptureR,
                                  MoveFlag::PromoCaptureB, MoveFlag::PromoCaptureN] {
                        out.push(Move::new(from, to, promo));
                    }
                } else {
                    out.push(Move::new(from, to, MoveFlag::Capture));
                }
            }
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core movegen::`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/movegen.rs
git commit -m "feat(chess-core): pseudo-legal move generation (no castling/EP yet)"
```

---

### Task 11: Castling and en passant in move generation

**Files:**
- Modify: `src-tauri/crates/chess-core/src/movegen.rs`

- [ ] **Step 1: Write the failing tests**

Append to `movegen.rs` tests:

```rust
#[test]
fn castling_when_clear() {
    // White king e1, white rooks a1/h1, otherwise empty, all castling rights.
    let fen = "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1";
    let pos = crate::fen::parse_fen(fen).unwrap();
    let moves = pseudo_legal_moves(&pos);
    let castles: Vec<_> = moves.iter()
        .filter(|m| matches!(m.flag(), MoveFlag::KingCastle | MoveFlag::QueenCastle))
        .collect();
    assert_eq!(castles.len(), 2);
}

#[test]
fn en_passant_square_produces_capture() {
    // Black just played d7-d5; white e5 pawn can take en passant on d6.
    let fen = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
    let pos = crate::fen::parse_fen(fen).unwrap();
    let ep_moves: Vec<_> = pseudo_legal_moves(&pos).iter()
        .filter(|m| m.flag() == MoveFlag::EnPassant)
        .collect();
    assert_eq!(ep_moves.len(), 1);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core movegen::tests::castling`
Expected: FAIL.

- [ ] **Step 3: Add castling and en-passant generation**

Add to `gen_pawn_moves` (right before the closing brace of the while loop, after diagonal captures):

```rust
        // en passant
        if let Some(ep_sq) = pos.en_passant {
            let ep_f = ep_sq.file() as i32;
            let ep_r = ep_sq.rank() as i32;
            if ep_r == nr && (ep_f - f).abs() == 1 {
                out.push(Move::new(from, ep_sq, MoveFlag::EnPassant));
            }
        }
```

Add a new function in `movegen.rs`:

```rust
fn gen_castling(pos: &Position, us: Color, occ: Bitboard, out: &mut MoveList) {
    use crate::types::{File, Rank};
    let (ks_right, qs_right, rank) = match us {
        Color::White => (pos.castling.white_king_side, pos.castling.white_queen_side, Rank::One),
        Color::Black => (pos.castling.black_king_side, pos.castling.black_queen_side, Rank::Eight),
    };
    let e = Square::new(File::E, rank);
    if ks_right {
        let f = Square::new(File::F, rank);
        let g = Square::new(File::G, rank);
        if !occ.contains(f) && !occ.contains(g) {
            out.push(Move::new(e, g, MoveFlag::KingCastle));
        }
    }
    if qs_right {
        let b = Square::new(File::B, rank);
        let c = Square::new(File::C, rank);
        let d = Square::new(File::D, rank);
        if !occ.contains(b) && !occ.contains(c) && !occ.contains(d) {
            out.push(Move::new(e, c, MoveFlag::QueenCastle));
        }
    }
}
```

Call `gen_castling(pos, us, occ, &mut out);` at the bottom of `pseudo_legal_moves`.

> Note: castling legality through-check is enforced by the legality filter in Task 12.

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core movegen::`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/movegen.rs
git commit -m "feat(chess-core): castling and en-passant move generation"
```

---

### Task 12: make_move / unmake_move + MoveUndo

**Files:**
- Modify: `src-tauri/crates/chess-core/src/make_move.rs`
- Modify: `src-tauri/crates/chess-core/src/position.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;
    use crate::moves::{Move, MoveFlag};
    use crate::types::{File, Rank, Square};

    #[test]
    fn make_then_unmake_returns_to_original() {
        let original = crate::position::Position::starting();
        let mut p = original.clone();
        let e2 = Square::new(File::E, Rank::Two);
        let e4 = Square::new(File::E, Rank::Four);
        let m = Move::new(e2, e4, MoveFlag::DoublePawnPush);
        let undo = make_move(&mut p, m);
        assert_ne!(p, original);
        unmake_move(&mut p, m, undo);
        assert_eq!(p, original);
    }

    #[test]
    fn ep_capture_removes_correct_pawn() {
        let pos = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let mut p = pos.clone();
        let e5 = Square::new(File::E, Rank::Five);
        let d6 = Square::new(File::D, Rank::Six);
        let d5 = Square::new(File::D, Rank::Five);
        let _ = make_move(&mut p, Move::new(e5, d6, MoveFlag::EnPassant));
        assert!(p.piece_at(d5).is_none(), "captured pawn should be gone");
        assert!(p.piece_at(d6).is_some(), "moving pawn now on d6");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core make_move::`
Expected: FAIL.

- [ ] **Step 3: Implement make/unmake**

```rust
use crate::bitboard::Bitboard;
use crate::moves::{Move, MoveFlag};
use crate::position::{CastlingRights, Position};
use crate::types::{Color, File, Piece, PieceKind, Rank, Square};

#[derive(Debug, Clone, Copy)]
pub struct MoveUndo {
    pub captured: Option<Piece>,
    pub prior_castling: CastlingRights,
    pub prior_en_passant: Option<Square>,
    pub prior_halfmove_clock: u8,
    pub prior_zobrist_hash: u64,
}

pub fn make_move(pos: &mut Position, m: Move) -> MoveUndo {
    let undo = MoveUndo {
        captured: pos.piece_at(m.to()),
        prior_castling: pos.castling,
        prior_en_passant: pos.en_passant,
        prior_halfmove_clock: pos.halfmove_clock,
        prior_zobrist_hash: pos.zobrist_hash,
    };
    let us = pos.side_to_move;
    let them = us.flip();
    let mover = pos.piece_at(m.from()).expect("piece on from square");

    // Remove mover from `from`.
    clear_square(pos, m.from(), mover);

    // Handle capture (standard, not EP).
    if let Some(cap) = undo.captured {
        clear_square(pos, m.to(), cap);
    }

    // Handle special moves.
    match m.flag() {
        MoveFlag::EnPassant => {
            let cap_rank = match us { Color::White => Rank::Five, Color::Black => Rank::Four };
            let cap_sq = Square::new(m.to().file(), cap_rank);
            let cap_piece = Piece { color: them, kind: PieceKind::Pawn };
            clear_square(pos, cap_sq, cap_piece);
            // Note: captured pawn isn't in `undo.captured` (which read from m.to()).
            // unmake_move handles EP separately.
        }
        MoveFlag::KingCastle => {
            let rank = m.from().rank();
            let rook_from = Square::new(File::H, rank);
            let rook_to   = Square::new(File::F, rank);
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, rook_from, rook);
            set_square  (pos, rook_to,   rook);
        }
        MoveFlag::QueenCastle => {
            let rank = m.from().rank();
            let rook_from = Square::new(File::A, rank);
            let rook_to   = Square::new(File::D, rank);
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, rook_from, rook);
            set_square  (pos, rook_to,   rook);
        }
        _ => {}
    }

    // Place mover on `to`, possibly with promotion.
    let placed = match m.flag() {
        MoveFlag::PromoKnight | MoveFlag::PromoCaptureN => Piece { color: us, kind: PieceKind::Knight },
        MoveFlag::PromoBishop | MoveFlag::PromoCaptureB => Piece { color: us, kind: PieceKind::Bishop },
        MoveFlag::PromoRook   | MoveFlag::PromoCaptureR => Piece { color: us, kind: PieceKind::Rook },
        MoveFlag::PromoQueen  | MoveFlag::PromoCaptureQ => Piece { color: us, kind: PieceKind::Queen },
        _ => mover,
    };
    set_square(pos, m.to(), placed);

    // Update castling rights if king or rook moved/was captured.
    update_castling_rights(pos, m, mover);

    // Update en-passant square.
    pos.en_passant = if m.flag() == MoveFlag::DoublePawnPush {
        let r = match us { Color::White => Rank::Three, Color::Black => Rank::Six };
        Some(Square::new(m.from().file(), r))
    } else { None };

    // Halfmove clock: reset on pawn move or capture, else increment.
    let is_capture = m.flag().is_capture() || undo.captured.is_some();
    if mover.kind == PieceKind::Pawn || is_capture {
        pos.halfmove_clock = 0;
    } else {
        pos.halfmove_clock = pos.halfmove_clock.saturating_add(1);
    }

    if us == Color::Black { pos.fullmove_number += 1; }
    pos.side_to_move = them;

    // Zobrist updates are done in Task 14 after Zobrist is introduced.
    undo
}

pub fn unmake_move(pos: &mut Position, m: Move, undo: MoveUndo) {
    let them = pos.side_to_move;
    let us = them.flip();

    if us == Color::Black { pos.fullmove_number -= 1; }
    pos.side_to_move = us;

    // Determine the piece currently on `to` (this is the mover, possibly promoted).
    let placed = pos.piece_at(m.to()).expect("piece on to-square after move");
    clear_square(pos, m.to(), placed);

    // Determine original mover (pre-promotion if promotion).
    let mover = if m.flag().is_promotion() {
        Piece { color: us, kind: PieceKind::Pawn }
    } else { placed };
    set_square(pos, m.from(), mover);

    // Restore captures.
    match m.flag() {
        MoveFlag::EnPassant => {
            let cap_rank = match us { Color::White => Rank::Five, Color::Black => Rank::Four };
            let cap_sq = Square::new(m.to().file(), cap_rank);
            set_square(pos, cap_sq, Piece { color: them, kind: PieceKind::Pawn });
        }
        MoveFlag::KingCastle => {
            let rank = m.from().rank();
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, Square::new(File::F, rank), rook);
            set_square  (pos, Square::new(File::H, rank), rook);
        }
        MoveFlag::QueenCastle => {
            let rank = m.from().rank();
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, Square::new(File::D, rank), rook);
            set_square  (pos, Square::new(File::A, rank), rook);
        }
        _ => {
            if let Some(cap) = undo.captured {
                set_square(pos, m.to(), cap);
            }
        }
    }

    pos.castling        = undo.prior_castling;
    pos.en_passant      = undo.prior_en_passant;
    pos.halfmove_clock  = undo.prior_halfmove_clock;
    pos.zobrist_hash    = undo.prior_zobrist_hash;
}

fn set_square(pos: &mut Position, sq: Square, p: Piece) {
    pos.bitboards[p.color.index()][p.kind as usize] =
        pos.bitboards[p.color.index()][p.kind as usize] | Bitboard::from_square(sq);
}
fn clear_square(pos: &mut Position, sq: Square, p: Piece) {
    pos.bitboards[p.color.index()][p.kind as usize] =
        pos.bitboards[p.color.index()][p.kind as usize] & !Bitboard::from_square(sq);
}

fn update_castling_rights(pos: &mut Position, m: Move, mover: Piece) {
    use crate::types::PieceKind::*;
    let r = m.from().rank();
    if mover.kind == King {
        match mover.color {
            Color::White => { pos.castling.white_king_side = false; pos.castling.white_queen_side = false; }
            Color::Black => { pos.castling.black_king_side = false; pos.castling.black_queen_side = false; }
        }
    }
    if mover.kind == Rook {
        let a_rank = Square::new(File::A, r);
        let h_rank = Square::new(File::H, r);
        if m.from() == a_rank {
            match mover.color {
                Color::White => pos.castling.white_queen_side = false,
                Color::Black => pos.castling.black_queen_side = false,
            }
        } else if m.from() == h_rank {
            match mover.color {
                Color::White => pos.castling.white_king_side = false,
                Color::Black => pos.castling.black_king_side = false,
            }
        }
    }
    // Rook captured on its original square -> lose corresponding right.
    let a1 = Square::new(File::A, Rank::One);
    let h1 = Square::new(File::H, Rank::One);
    let a8 = Square::new(File::A, Rank::Eight);
    let h8 = Square::new(File::H, Rank::Eight);
    if m.to() == a1 { pos.castling.white_queen_side = false; }
    if m.to() == h1 { pos.castling.white_king_side  = false; }
    if m.to() == a8 { pos.castling.black_queen_side = false; }
    if m.to() == h8 { pos.castling.black_king_side  = false; }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/
git commit -m "feat(chess-core): make_move / unmake_move with MoveUndo"
```

---

### Task 13: Legality filter (square_attacked + filter_legal)

**Files:**
- Modify: `src-tauri/crates/chess-core/src/movegen.rs`

- [ ] **Step 1: Write the failing tests**

Append to `movegen.rs` tests:

```rust
#[test]
fn starting_position_has_20_legal_moves() {
    let pos = Position::starting();
    assert_eq!(legal_moves(&pos).len(), 20);
}

#[test]
fn pinned_piece_cannot_move() {
    // White king on e1, white knight on e2, black rook on e8. Knight is pinned.
    let fen = "4r3/8/8/8/8/8/4N3/4K3 w - - 0 1";
    let pos = crate::fen::parse_fen(fen).unwrap();
    let legal = legal_moves(&pos);
    let knight_moves: Vec<_> = legal.iter()
        .filter(|m| m.from() == Square::new(File::E, Rank::Two))
        .collect();
    assert_eq!(knight_moves.len(), 0);
}

#[test]
fn cannot_castle_through_check() {
    // White king e1, white rook h1, black rook on f8 attacking f1.
    let fen = "5r2/8/8/8/8/8/8/4K2R w K - 0 1";
    let pos = crate::fen::parse_fen(fen).unwrap();
    let castles: Vec<_> = legal_moves(&pos).iter()
        .filter(|m| m.flag() == MoveFlag::KingCastle)
        .collect();
    assert_eq!(castles.len(), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core movegen::tests::starting_position_has_20_legal`
Expected: FAIL.

- [ ] **Step 3: Implement attack detection and legal filtering**

Add to `movegen.rs`:

```rust
use crate::make_move::{make_move as do_make, unmake_move as do_unmake};

pub fn square_attacked(pos: &Position, sq: Square, by: Color) -> bool {
    let occ = pos.all_pieces();
    let bbs = &pos.bitboards[by.index()];

    if (pawn_attacks(by.flip(), sq) & bbs[PieceKind::Pawn as usize]).0 != 0 { return true; }
    if (knight_attacks(sq)         & bbs[PieceKind::Knight as usize]).0 != 0 { return true; }
    if (king_attacks(sq)           & bbs[PieceKind::King as usize]).0 != 0 { return true; }
    let bishops_queens = bbs[PieceKind::Bishop as usize] | bbs[PieceKind::Queen as usize];
    if (bishop_attacks(sq, occ) & bishops_queens).0 != 0 { return true; }
    let rooks_queens = bbs[PieceKind::Rook as usize] | bbs[PieceKind::Queen as usize];
    if (rook_attacks(sq, occ) & rooks_queens).0 != 0 { return true; }
    false
}

pub fn is_in_check(pos: &Position) -> bool {
    let us = pos.side_to_move;
    let king_bb = pos.bitboards[us.index()][PieceKind::King as usize];
    if king_bb.0 == 0 { return false; }
    let king_sq = Square::from_index(king_bb.0.trailing_zeros() as u8).unwrap();
    square_attacked(pos, king_sq, us.flip())
}

pub fn legal_moves(pos: &Position) -> MoveList {
    let mut out = MoveList::new();
    let pseudo = pseudo_legal_moves(pos);
    let mut probe = pos.clone();
    let us = pos.side_to_move;

    for m in pseudo {
        // For castling, also check the king doesn't pass through check.
        if m.flag() == MoveFlag::KingCastle || m.flag() == MoveFlag::QueenCastle {
            let rank = m.from().rank();
            let through_file = if m.flag() == MoveFlag::KingCastle { File::F } else { File::D };
            let through = Square::new(through_file, rank);
            if square_attacked(&probe, m.from(), us.flip())
                || square_attacked(&probe, through, us.flip()) {
                continue;
            }
        }
        let undo = do_make(&mut probe, m);
        // After the move, our king must not be under attack from the *new* side-to-move.
        let king_bb = probe.bitboards[us.index()][PieceKind::King as usize];
        let ok = if king_bb.0 == 0 {
            true
        } else {
            let king_sq = Square::from_index(king_bb.0.trailing_zeros() as u8).unwrap();
            !square_attacked(&probe, king_sq, us.flip())
        };
        do_unmake(&mut probe, m, undo);
        if ok { out.push(m); }
    }
    out
}
```

Re-export from `lib.rs` (already done since we use `pub mod movegen`).

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/movegen.rs
git commit -m "feat(chess-core): legal move filter (pins, checks, castling through check)"
```

---

### Task 14: Zobrist hashing

**Files:**
- Modify: `src-tauri/crates/chess-core/src/zobrist.rs`
- Modify: `src-tauri/crates/chess-core/src/position.rs` (compute on construction)
- Modify: `src-tauri/crates/chess-core/src/make_move.rs` (incremental update)

- [ ] **Step 1: Write the failing test**

In `zobrist.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;
    use crate::moves::{Move, MoveFlag};
    use crate::types::{File, Rank, Square};
    use crate::make_move::{make_move, unmake_move};

    #[test]
    fn equal_positions_have_equal_hash() {
        let a = crate::position::Position::starting();
        let b = parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(a.zobrist_hash, b.zobrist_hash);
        assert_ne!(a.zobrist_hash, 0);
    }

    #[test]
    fn incremental_update_matches_full_recompute() {
        let mut p = crate::position::Position::starting();
        let m = Move::new(Square::new(File::E, Rank::Two), Square::new(File::E, Rank::Four), MoveFlag::DoublePawnPush);
        let undo = make_move(&mut p, m);
        let recomputed = compute_zobrist(&p);
        assert_eq!(p.zobrist_hash, recomputed);
        unmake_move(&mut p, m, undo);
        assert_eq!(p.zobrist_hash, compute_zobrist(&p));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chess-core zobrist::`
Expected: FAIL — `compute_zobrist` not defined.

- [ ] **Step 3: Implement Zobrist tables and full-recompute**

```rust
use crate::position::Position;
use crate::types::{Color, Square};

pub struct ZobristKeys {
    pub pieces: [[[u64; 64]; 6]; 2], // [color][piece_kind][square]
    pub side_to_move: u64,
    pub castling: [u64; 16],         // index = bitmask of 4 castling rights
    pub en_passant_file: [u64; 8],
}

// Deterministic PRNG (xorshift64) seeded for reproducible hashes across runs.
const fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13; x ^= x >> 7; x ^= x << 17; x
}

pub static KEYS: ZobristKeys = build_keys();

const fn build_keys() -> ZobristKeys {
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut pieces = [[[0u64; 64]; 6]; 2];
    let mut c = 0;
    while c < 2 {
        let mut k = 0;
        while k < 6 {
            let mut s = 0;
            while s < 64 {
                seed = xorshift64(seed);
                pieces[c][k][s] = seed;
                s += 1;
            }
            k += 1;
        }
        c += 1;
    }
    seed = xorshift64(seed);
    let side_to_move = seed;
    let mut castling = [0u64; 16];
    let mut i = 0;
    while i < 16 {
        seed = xorshift64(seed);
        castling[i] = seed;
        i += 1;
    }
    let mut en_passant_file = [0u64; 8];
    let mut f = 0;
    while f < 8 {
        seed = xorshift64(seed);
        en_passant_file[f] = seed;
        f += 1;
    }
    ZobristKeys { pieces, side_to_move, castling, en_passant_file }
}

pub fn castling_index(c: crate::position::CastlingRights) -> usize {
    (c.white_king_side  as usize)
  | ((c.white_queen_side as usize) << 1)
  | ((c.black_king_side  as usize) << 2)
  | ((c.black_queen_side as usize) << 3)
}

pub fn compute_zobrist(pos: &Position) -> u64 {
    let mut h: u64 = 0;
    for c in [Color::White, Color::Black] {
        for k in 0..6 {
            let mut bb = pos.bitboards[c.index()][k];
            while let Some(sq) = bb.pop_lsb() {
                h ^= KEYS.pieces[c.index()][k][sq.index() as usize];
            }
        }
    }
    if pos.side_to_move == Color::Black { h ^= KEYS.side_to_move; }
    h ^= KEYS.castling[castling_index(pos.castling)];
    if let Some(ep) = pos.en_passant {
        h ^= KEYS.en_passant_file[ep.file() as usize];
    }
    h
}
```

- [ ] **Step 4: Populate hash on construction**

In `fen.rs`, at the end of `parse_fen`, before the final `Ok(pos)`:

```rust
pos.zobrist_hash = crate::zobrist::compute_zobrist(&pos);
```

In `position.rs::empty()`, no change needed (hash is correctly 0 with no pieces — but the side-to-move key is white-default, so for completeness call compute too if you want; leaving 0 for empty is fine since no engine uses an empty board).

- [ ] **Step 5: Update Zobrist incrementally in make_move**

For simplicity in v1, **fully recompute** at the end of `make_move`:

In `make_move.rs::make_move`, just before returning `undo`, add:

```rust
pos.zobrist_hash = crate::zobrist::compute_zobrist(pos);
```

`unmake_move` already restores `prior_zobrist_hash`. A truly incremental update is an optimization for a later plan — flagged as a TODO in code:

```rust
// TODO(perf): incremental zobrist update — recomputing each move is ~30% of search time.
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p chess-core`
Expected: all passing.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/crates/chess-core/src/
git commit -m "feat(chess-core): zobrist hashing (full recompute for v1)"
```

---

### Task 15: Outcome detection

**Files:**
- Modify: `src-tauri/crates/chess-core/src/outcome.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;

    #[test]
    fn fools_mate_is_checkmate() {
        // 1. f3 e5 2. g4 Qh4#
        let pos = parse_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3").unwrap();
        assert_eq!(detect_outcome(&pos), Some(Outcome::Checkmate));
    }

    #[test]
    fn stalemate_detected() {
        // Classic stalemate: black to move, no legal moves, not in check.
        let pos = parse_fen("k7/2Q5/1K6/8/8/8/8/8 b - - 0 1").unwrap();
        assert_eq!(detect_outcome(&pos), Some(Outcome::Stalemate));
    }

    #[test]
    fn fifty_move_rule() {
        let mut pos = crate::position::Position::starting();
        pos.halfmove_clock = 100;
        assert_eq!(detect_outcome(&pos), Some(Outcome::FiftyMoveRule));
    }

    #[test]
    fn starting_position_is_ongoing() {
        let pos = crate::position::Position::starting();
        assert_eq!(detect_outcome(&pos), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core outcome::`
Expected: FAIL.

- [ ] **Step 3: Implement outcome detection**

```rust
use crate::movegen::{is_in_check, legal_moves};
use crate::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Checkmate,
    Stalemate,
    FiftyMoveRule,
    InsufficientMaterial,
    // Threefold repetition is detected at the game level, not from a single Position.
}

pub fn detect_outcome(pos: &Position) -> Option<Outcome> {
    if pos.halfmove_clock >= 100 { return Some(Outcome::FiftyMoveRule); }
    if has_insufficient_material(pos) { return Some(Outcome::InsufficientMaterial); }
    if legal_moves(pos).is_empty() {
        return Some(if is_in_check(pos) { Outcome::Checkmate } else { Outcome::Stalemate });
    }
    None
}

fn has_insufficient_material(pos: &Position) -> bool {
    use crate::types::{Color, PieceKind};
    // Count pieces excluding kings.
    let mut total_minor = 0u32;
    let mut total_other = 0u32;
    for c in [Color::White, Color::Black] {
        for k in 0..6 {
            let n = pos.bitboards[c.index()][k].count();
            match k {
                k if k == PieceKind::King as usize => {}
                k if k == PieceKind::Knight as usize || k == PieceKind::Bishop as usize => { total_minor += n; }
                _ => { total_other += n; }
            }
        }
    }
    total_other == 0 && total_minor <= 1
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core outcome::`
Expected: 4 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/outcome.rs
git commit -m "feat(chess-core): outcome detection (mate/stalemate/50-move/insufficient material)"
```

---

### Task 16: Perft infrastructure

**Files:**
- Modify: `src-tauri/crates/chess-core/src/perft.rs`

Perft (performance test) counts the number of legal positions at depth N. It's the gold standard for move-gen correctness — every chess engine validates against published perft numbers.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn perft_starting_depth_1_is_20() {
        let pos = Position::starting();
        assert_eq!(perft(&pos, 1), 20);
    }

    #[test]
    fn perft_starting_depth_2_is_400() {
        let pos = Position::starting();
        assert_eq!(perft(&pos, 2), 400);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core perft::`
Expected: FAIL.

- [ ] **Step 3: Implement perft**

```rust
use crate::make_move::{make_move, unmake_move};
use crate::movegen::legal_moves;
use crate::position::Position;

pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves = legal_moves(pos);
    if depth == 1 { return moves.len() as u64; }
    let mut probe = pos.clone();
    let mut nodes = 0u64;
    for m in moves {
        let undo = make_move(&mut probe, m);
        nodes += perft(&probe, depth - 1);
        unmake_move(&mut probe, m, undo);
    }
    nodes
}

/// Divides nodes by root move — for debugging when totals disagree with the reference.
pub fn perft_divide(pos: &Position, depth: u32) -> Vec<(crate::moves::Move, u64)> {
    let mut out = vec![];
    let mut probe = pos.clone();
    for m in legal_moves(pos) {
        let undo = make_move(&mut probe, m);
        let n = if depth <= 1 { 1 } else { perft(&probe, depth - 1) };
        unmake_move(&mut probe, m, undo);
        out.push((m, n));
    }
    out
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core perft::`
Expected: 2 passed (these are fast; deeper perft comes next task).

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/perft.rs
git commit -m "feat(chess-core): perft + perft_divide"
```

---

### Task 17: Perft test suite against published reference positions

**Files:**
- Create: `src-tauri/crates/chess-core/tests/perft_suite.rs`

These are integration tests — slow but they're the make-or-break correctness check for the entire move generator. They use the canonical positions from the Chess Programming Wiki.

- [ ] **Step 1: Write the test file**

`src-tauri/crates/chess-core/tests/perft_suite.rs`:

```rust
//! Perft suite — the make-or-break correctness check for move generation.
//!
//! Reference: https://www.chessprogramming.org/Perft_Results

use chess_core::fen::parse_fen;
use chess_core::perft::perft;

fn run(fen: &str, depth_results: &[(u32, u64)]) {
    let pos = parse_fen(fen).expect("valid fen");
    for &(d, expected) in depth_results {
        let actual = perft(&pos, d);
        assert_eq!(
            actual, expected,
            "perft({d}) mismatch for {fen}: expected {expected}, got {actual}"
        );
    }
}

#[test]
fn perft_position_1_starting() {
    run(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &[(1, 20), (2, 400), (3, 8902), (4, 197281)],
    );
}

#[test]
fn perft_position_2_kiwipete() {
    run(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[(1, 48), (2, 2039), (3, 97862)],
    );
}

#[test]
fn perft_position_3_endgame() {
    run(
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[(1, 14), (2, 191), (3, 2812), (4, 43238)],
    );
}

#[test]
fn perft_position_4() {
    run(
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[(1, 6), (2, 264), (3, 9467)],
    );
}

#[test]
fn perft_position_5() {
    run(
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[(1, 44), (2, 1486), (3, 62379)],
    );
}

#[test]
fn perft_position_6() {
    run(
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        &[(1, 46), (2, 2079), (3, 89890)],
    );
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p chess-core --test perft_suite --release`
Expected: 6 passed. (Use `--release` because some go to depth 4.)

> **Important:** if any perft test fails, *do not proceed* to later tasks. Use `perft_divide` to find which root move's subtree disagrees with the reference, then descend into that subtree. A failing perft test means a move-gen bug — every later layer will be unsound on top of it.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/crates/chess-core/tests/perft_suite.rs
git commit -m "test(chess-core): perft suite against 6 standard reference positions"
```

---

### Task 18: PGN parser (SAN move list)

**Files:**
- Modify: `src-tauri/crates/chess-core/src/pgn.rs`

We'll handle a pragmatic subset: tag pairs, SAN move list, result. No comments, NAGs, or variations in v1 (could be added later).

- [ ] **Step 1: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const SCHOLARS_MATE: &str = r#"[Event "Test"]
[Site "?"]
[Date "2026.05.20"]
[White "A"]
[Black "B"]
[Result "1-0"]

1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6?? 4. Qxf7# 1-0
"#;

    #[test]
    fn parses_scholars_mate_to_completion() {
        let game = parse_pgn(SCHOLARS_MATE).unwrap();
        assert_eq!(game.tags.get("White").map(|s| s.as_str()), Some("A"));
        assert_eq!(game.result, "1-0");
        assert_eq!(game.moves.len(), 7);
        // Final position should be checkmate.
        assert_eq!(
            crate::outcome::detect_outcome(&game.final_position),
            Some(crate::outcome::Outcome::Checkmate)
        );
    }

    #[test]
    fn rejects_illegal_san() {
        let bad = "[Result \"*\"]\n\n1. e9 *\n";
        assert!(parse_pgn(bad).is_err());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p chess-core pgn::`
Expected: FAIL.

- [ ] **Step 3: Implement the parser**

```rust
use std::collections::HashMap;

use crate::make_move::make_move;
use crate::moves::{Move, MoveFlag};
use crate::movegen::legal_moves;
use crate::position::Position;
use crate::types::{File, PieceKind, Rank, Square};

#[derive(Debug)]
pub struct Game {
    pub tags: HashMap<String, String>,
    pub moves: Vec<Move>,
    pub result: String,
    pub final_position: Position,
}

#[derive(Debug)]
pub enum PgnError {
    Malformed,
    IllegalSan(String),
    AmbiguousSan(String),
}

pub fn parse_pgn(input: &str) -> Result<Game, PgnError> {
    let mut tags = HashMap::new();
    let mut body = String::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Some(rest) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // [Name "Value"]
            let mut parts = rest.splitn(2, ' ');
            let name = parts.next().ok_or(PgnError::Malformed)?.to_string();
            let value = parts.next().ok_or(PgnError::Malformed)?;
            let value = value.trim_matches('"');
            tags.insert(name, value.to_string());
        } else {
            body.push_str(line);
            body.push(' ');
        }
    }

    let mut pos = Position::starting();
    let mut moves = vec![];
    let mut result = "*".to_string();

    for token in body.split_ascii_whitespace() {
        // Skip move numbers like "1." or "12...".
        if token.chars().next().map_or(false, |c| c.is_ascii_digit())
            && token.contains('.') {
            continue;
        }
        if matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*") {
            result = token.to_string();
            break;
        }
        let m = parse_san(&pos, token)?;
        make_move(&mut pos, m);
        moves.push(m);
    }

    Ok(Game { tags, moves, result, final_position: pos })
}

fn parse_san(pos: &Position, san: &str) -> Result<Move, PgnError> {
    // Strip + and # and ! ? annotations.
    let san = san.trim_end_matches(|c: char| matches!(c, '+' | '#' | '!' | '?'));

    // Castling.
    if san == "O-O" || san == "0-0" {
        return find_move(pos, |m| m.flag() == MoveFlag::KingCastle, san);
    }
    if san == "O-O-O" || san == "0-0-0" {
        return find_move(pos, |m| m.flag() == MoveFlag::QueenCastle, san);
    }

    // Parse promotion suffix: e.g. "e8=Q" or "exd8=Q".
    let (san_core, promo) = if let Some(idx) = san.rfind('=') {
        let promo_char = san.as_bytes().get(idx + 1).copied().ok_or(PgnError::Malformed)?;
        (&san[..idx], Some(promo_char))
    } else { (san, None) };

    let bytes = san_core.as_bytes();
    if bytes.is_empty() { return Err(PgnError::Malformed); }

    // Determine piece (capital letter at start except for pawn).
    let (piece_kind, mut i) = match bytes[0] {
        b'N' => (PieceKind::Knight, 1),
        b'B' => (PieceKind::Bishop, 1),
        b'R' => (PieceKind::Rook,   1),
        b'Q' => (PieceKind::Queen,  1),
        b'K' => (PieceKind::King,   1),
        b'a'..=b'h' => (PieceKind::Pawn, 0),
        _ => return Err(PgnError::Malformed),
    };

    // Optional disambiguation: file letter, rank digit, or both. Capture marker 'x'.
    let mut disamb_file: Option<u8> = None;
    let mut disamb_rank: Option<u8> = None;
    let mut _is_capture = false;

    // The destination is the LAST 2 chars (file letter + rank digit).
    if bytes.len() - i < 2 { return Err(PgnError::Malformed); }
    let dest_start = bytes.len() - 2;
    while i < dest_start {
        match bytes[i] {
            b'x' => { _is_capture = true; }
            f @ b'a'..=b'h' => { disamb_file = Some(f - b'a'); }
            r @ b'1'..=b'8' => { disamb_rank = Some(r - b'1'); }
            _ => return Err(PgnError::Malformed),
        }
        i += 1;
    }
    let to = parse_sq_bytes(&bytes[dest_start..]).ok_or(PgnError::Malformed)?;

    // Find a legal move matching criteria.
    find_move(pos, |m| {
        if m.to() != to { return false; }
        if !matches_piece(pos, m, piece_kind) { return false; }
        if let Some(df) = disamb_file { if m.from().file() as u8 != df { return false; } }
        if let Some(dr) = disamb_rank { if m.from().rank() as u8 != dr { return false; } }
        if let Some(p) = promo {
            let want = match p { b'N' => PieceKind::Knight, b'B' => PieceKind::Bishop,
                                  b'R' => PieceKind::Rook,   b'Q' => PieceKind::Queen,
                                  _ => return false };
            return promo_kind(m.flag()) == Some(want);
        } else if m.flag().is_promotion() {
            return false;
        }
        true
    }, san)
}

fn matches_piece(pos: &Position, m: Move, kind: PieceKind) -> bool {
    pos.piece_at(m.from()).map_or(false, |p| p.kind == kind)
}

fn promo_kind(flag: MoveFlag) -> Option<PieceKind> {
    Some(match flag {
        MoveFlag::PromoKnight | MoveFlag::PromoCaptureN => PieceKind::Knight,
        MoveFlag::PromoBishop | MoveFlag::PromoCaptureB => PieceKind::Bishop,
        MoveFlag::PromoRook   | MoveFlag::PromoCaptureR => PieceKind::Rook,
        MoveFlag::PromoQueen  | MoveFlag::PromoCaptureQ => PieceKind::Queen,
        _ => return None,
    })
}

fn find_move<F: Fn(Move) -> bool>(pos: &Position, predicate: F, san: &str) -> Result<Move, PgnError> {
    let candidates: Vec<Move> = legal_moves(pos).into_iter().filter(|m| predicate(*m)).collect();
    match candidates.len() {
        0 => Err(PgnError::IllegalSan(san.to_string())),
        1 => Ok(candidates[0]),
        _ => Err(PgnError::AmbiguousSan(san.to_string())),
    }
}

fn parse_sq_bytes(bytes: &[u8]) -> Option<Square> {
    if bytes.len() != 2 { return None; }
    let f = File::from_index(bytes[0].wrapping_sub(b'a'))?;
    let r = Rank::from_index(bytes[1].wrapping_sub(b'1'))?;
    Some(Square::new(f, r))
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core pgn::`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/pgn.rs
git commit -m "feat(chess-core): PGN parser (tags + SAN move list + result)"
```

---

### Task 19: PGN serializer (SAN output + round-trip)

**Files:**
- Modify: `src-tauri/crates/chess-core/src/pgn.rs`

- [ ] **Step 1: Write the failing tests**

Append to `pgn.rs` tests:

```rust
#[test]
fn serialize_then_parse_round_trip() {
    let game = parse_pgn(SCHOLARS_MATE).unwrap();
    let written = serialize_pgn(&game);
    let reparsed = parse_pgn(&written).unwrap();
    assert_eq!(reparsed.moves, game.moves);
    assert_eq!(reparsed.result, game.result);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p chess-core pgn::tests::serialize`
Expected: FAIL.

- [ ] **Step 3: Implement the serializer**

Append to `pgn.rs`:

```rust
pub fn serialize_pgn(game: &Game) -> String {
    let mut out = String::new();
    let tag_order = ["Event", "Site", "Date", "Round", "White", "Black", "Result"];
    for &name in &tag_order {
        if let Some(v) = game.tags.get(name) {
            out.push_str(&format!("[{name} \"{v}\"]\n"));
        }
    }
    for (k, v) in &game.tags {
        if !tag_order.contains(&k.as_str()) {
            out.push_str(&format!("[{k} \"{v}\"]\n"));
        }
    }
    out.push('\n');

    let mut pos = Position::starting();
    for (i, &m) in game.moves.iter().enumerate() {
        if i % 2 == 0 {
            out.push_str(&format!("{}. ", i / 2 + 1));
        }
        out.push_str(&move_to_san(&pos, m));
        out.push(' ');
        make_move(&mut pos, m);
    }
    out.push_str(&game.result);
    out.push('\n');
    out
}

fn move_to_san(pos: &Position, m: Move) -> String {
    if m.flag() == MoveFlag::KingCastle  { return base_with_suffix(pos, m, "O-O".into()); }
    if m.flag() == MoveFlag::QueenCastle { return base_with_suffix(pos, m, "O-O-O".into()); }

    let p = pos.piece_at(m.from()).expect("piece on from");
    let mut s = String::new();
    let is_capture = m.flag().is_capture();
    let dest = sq_str(m.to());

    if p.kind == PieceKind::Pawn {
        if is_capture {
            s.push(file_char(m.from().file()));
            s.push('x');
        }
        s.push_str(&dest);
    } else {
        s.push(piece_letter(p.kind));
        // Disambiguation: check if other same-kind pieces also can reach `to`.
        let competitors: Vec<Move> = legal_moves(pos).into_iter()
            .filter(|cm| cm.to() == m.to()
                     && cm.from() != m.from()
                     && pos.piece_at(cm.from()).map_or(false, |q| q.kind == p.kind))
            .collect();
        if !competitors.is_empty() {
            let same_file = competitors.iter().any(|c| c.from().file() == m.from().file());
            let same_rank = competitors.iter().any(|c| c.from().rank() == m.from().rank());
            if !same_file       { s.push(file_char(m.from().file())); }
            else if !same_rank  { s.push(rank_char(m.from().rank())); }
            else { s.push(file_char(m.from().file())); s.push(rank_char(m.from().rank())); }
        }
        if is_capture { s.push('x'); }
        s.push_str(&dest);
    }

    if let Some(promo) = promo_kind(m.flag()) {
        s.push('=');
        s.push(piece_letter(promo));
    }

    base_with_suffix(pos, m, s)
}

fn base_with_suffix(pos: &Position, m: Move, mut s: String) -> String {
    let mut probe = pos.clone();
    let _ = make_move(&mut probe, m);
    if crate::movegen::is_in_check(&probe) {
        if legal_moves(&probe).is_empty() { s.push('#'); }
        else { s.push('+'); }
    }
    s
}

fn piece_letter(k: PieceKind) -> char {
    match k {
        PieceKind::Knight => 'N', PieceKind::Bishop => 'B',
        PieceKind::Rook   => 'R', PieceKind::Queen  => 'Q',
        PieceKind::King   => 'K', PieceKind::Pawn   => 'P',
    }
}
fn file_char(f: File) -> char { (b'a' + f as u8) as char }
fn rank_char(r: Rank) -> char { (b'1' + r as u8) as char }
fn sq_str(sq: Square) -> String {
    let mut s = String::with_capacity(2);
    s.push(file_char(sq.file()));
    s.push(rank_char(sq.rank()));
    s
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test -p chess-core`
Expected: all passing.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/crates/chess-core/src/pgn.rs
git commit -m "feat(chess-core): PGN serializer + round-trip test"
```

---

### Task 20: Public re-exports + crate README

**Files:**
- Modify: `src-tauri/crates/chess-core/src/lib.rs`
- Create: `src-tauri/crates/chess-core/README.md`

- [ ] **Step 1: Add prelude / re-exports**

Replace `lib.rs`:

```rust
//! chess-core: pure chess rules library.
//!
//! Public surface intended for downstream crates lives in the prelude.

pub mod attacks;
pub mod bitboard;
pub mod fen;
pub mod make_move;
pub mod movegen;
pub mod moves;
pub mod outcome;
pub mod perft;
pub mod pgn;
pub mod position;
pub mod types;
pub mod zobrist;

pub mod prelude {
    pub use crate::bitboard::Bitboard;
    pub use crate::fen::{parse_fen, serialize_fen, FenError};
    pub use crate::make_move::{make_move, unmake_move, MoveUndo};
    pub use crate::movegen::{is_in_check, legal_moves, pseudo_legal_moves, square_attacked};
    pub use crate::moves::{Move, MoveFlag, MoveList};
    pub use crate::outcome::{detect_outcome, Outcome};
    pub use crate::perft::{perft, perft_divide};
    pub use crate::pgn::{parse_pgn, serialize_pgn, Game, PgnError};
    pub use crate::position::{CastlingRights, Position};
    pub use crate::types::{Color, File, Piece, PieceKind, Rank, Square};
}
```

- [ ] **Step 2: Write the crate README**

`src-tauri/crates/chess-core/README.md`:

```markdown
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
```

- [ ] **Step 3: Verify everything still builds and tests pass**

Run: `cargo test -p chess-core`
Expected: all unit tests passing.
Run: `cargo test -p chess-core --test perft_suite --release`
Expected: 6 passed.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/crates/chess-core/
git commit -m "docs(chess-core): prelude re-exports + crate README"
```

---

## Done state

When this plan completes:

- `cargo test -p chess-core` passes (~50+ unit tests).
- `cargo test -p chess-core --test perft_suite --release` passes — proves move generation correctness against 6 canonical positions through depths 3-4.
- The `chess-core` crate exposes a small, focused public API via `chess_core::prelude`: `Position`, `Move`, `legal_moves`, `make_move`/`unmake_move`, `parse_fen`/`serialize_fen`, `parse_pgn`/`serialize_pgn`, `perft`, `detect_outcome`.
- No engine, no GUI, no Tauri yet — those land in Plan 2.

## Known optimizations deferred

These are flagged in code as `TODO(perf)` and addressed in later plans only if profiling shows them as bottlenecks:

- **Magic bitboards** for sliding piece attacks (currently ray-based loop).
- **Incremental Zobrist updates** in `make_move` (currently full recompute).
- **Staged move generation** (currently generates all moves upfront).

These don't affect correctness, only speed.
