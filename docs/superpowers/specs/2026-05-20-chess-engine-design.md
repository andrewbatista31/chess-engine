# Chess Engine — Design

**Date:** 2026-05-20
**Status:** Approved (design phase)

## Goal

A standalone desktop chess application where the user can:

- Play against themselves (free play, both sides controlled locally)
- Play against an engine (DIY or Stockfish, either side)
- Watch engine-vs-engine games (DIY vs Stockfish)
- Set up arbitrary positions via FEN or board editor
- See live analysis: top-N candidate moves, principal variations, evaluation bar
- Save and load games as PGN; undo/redo moves

A secondary goal is to *build* a club-strength (~1600-2000 Elo) chess engine from scratch in Rust, with Stockfish integrated as a reference for comparison.

## Stack

- **Backend:** Rust (engine, rules, Tauri commands)
- **Frontend:** Svelte + TypeScript
- **App shell:** Tauri (native desktop, cross-platform; Windows is the primary target)
- **External engine:** Stockfish (bundled binary, communicated with via UCI)

## High-level architecture

```
chess-engine/                     ← Tauri project root
├── src-tauri/                    ← Rust backend
│   ├── crates/
│   │   ├── chess-core/           ← board, moves, FEN/PGN, rules (pure logic, no I/O)
│   │   ├── chess-engine-diy/     ← DIY engine: search + eval
│   │   ├── chess-engine-uci/     ← Stockfish adapter (UCI subprocess)
│   │   └── chess-engine-api/     ← Engine trait + shared types
│   ├── src/main.rs               ← Tauri commands, wires engines to frontend
│   └── stockfish/                ← bundled stockfish.exe
└── src/                          ← Svelte frontend
    ├── lib/board/                ← board rendering, drag-drop
    ├── lib/analysis/             ← eval bar, top-N moves panel
    ├── lib/gamestate/            ← move history, PGN, undo/redo store
    └── routes/                   ← play, setup, engine-vs-engine views
```

**Module boundaries:**

- `chess-core` knows nothing about engines, GUIs, or async — pure rules and data. Independently testable.
- `chess-engine-api` defines the `Engine` trait. Both engine crates implement it.
- The Tauri layer is thin: receives commands from the frontend, dispatches to the right `Box<dyn Engine>`, returns results / streams analysis events.
- Frontend never knows which engine it's talking to.

**Crate split rationale:** keeps `chess-core` independently testable (the perf-critical chess core needs aggressive unit tests + benchmarks), and makes it trivial to publish the DIY engine as a standalone UCI binary later.

## Core data model (`chess-core`)

Board representation: **bitboards** (`u64` per piece type × color). Chosen over a mailbox array because club-level move-gen and eval performance depend on it.

```rust
pub struct Position {
    bitboards: [u64; 12],   // 6 piece types × 2 colors
    side_to_move: Color,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u8,
    fullmove_number: u16,
    zobrist_hash: u64,
}

pub struct Move(u16);  // packed: from(6) | to(6) | flags(4)

impl Position {
    pub fn from_fen(s: &str) -> Result<Self>;
    pub fn to_fen(&self) -> String;
    pub fn legal_moves(&self) -> MoveList;     // ArrayVec-backed, no alloc
    pub fn make_move(&mut self, m: Move) -> MoveUndo;
    pub fn unmake_move(&mut self, undo: MoveUndo);
    pub fn is_check(&self) -> bool;
    pub fn outcome(&self) -> Option<Outcome>;
}

pub mod pgn { /* parse + serialize */ }
pub mod fen { /* parse + serialize */ }
```

**Notable choices:**

- `make_move` / `unmake_move` rather than cloning positions — essential for fast search.
- `MoveUndo` stores everything needed to revert (captured piece, prior castling rights, prior en-passant, prior halfmove clock, prior zobrist). Small struct on the stack.
- Zobrist hashing baked in from the start; the transposition table needs it.
- `MoveList` is stack-allocated (`ArrayVec<Move, 256>`) — no heap allocs in the hot path.

## Engine API (`chess-engine-api`)

```rust
pub struct SearchLimits {
    pub max_depth: Option<u8>,
    pub max_time: Option<Duration>,
    pub max_nodes: Option<u64>,
}

#[derive(Clone)]
pub struct AnalysisInfo {
    pub depth: u8,
    pub score: Score,           // centipawns or Mate(n)
    pub pv: Vec<Move>,           // principal variation
    pub multipv_index: u8,
    pub nodes: u64,
    pub nps: u64,
    pub time: Duration,
}

#[async_trait]
pub trait Engine: Send + Sync {
    fn name(&self) -> &str;

    async fn analyze(
        &self,
        position: Position,
        limits: SearchLimits,
        multipv: u8,
    ) -> mpsc::Receiver<AnalysisInfo>;

    async fn stop(&self);
}
```

One trait, two implementations. The frontend sees a uniform interface regardless of which engine is active.

## DIY engine (`chess-engine-diy`)

Layered architecture; each layer added and tested independently:

```
┌─────────────────────────────────────────────────┐
│  Iterative deepening driver                      │  ← time management
├─────────────────────────────────────────────────┤
│  Alpha-beta search with PVS                      │
│  + transposition table probe/store               │
│  + move ordering (TT move, MVV-LVA, killers)     │
│  + null-move pruning                             │
├─────────────────────────────────────────────────┤
│  Quiescence search (captures-only)               │
├─────────────────────────────────────────────────┤
│  Evaluation                                      │
│  • material                                      │
│  • piece-square tables (tapered: mg → eg)        │
│  • mobility                                      │
│  • basic king safety (pawn shield)               │
│  • basic pawn structure (doubled, isolated)      │
└─────────────────────────────────────────────────┘
```

**Key data structures:**

- `TranspositionTable` — open-addressed, fixed size (default 64 MB), entries store `{zobrist, depth, score, score_bound, best_move, age}`.
- `KillerMoves[2][MAX_PLY]` and `HistoryHeuristic[12][64]` for move ordering.
- `SearchContext` holds TT, killers, history, stop flag, node counter — passed by `&mut` through search.

**Concurrency:** one search runs on a Tokio blocking task (search is CPU-bound). It sends `AnalysisInfo` events through an `mpsc::Sender` after each completed iterative-deepening iteration. `stop()` flips an `AtomicBool` the search checks every N nodes.

**Build order** (each step a working engine on its own):

1. Perft (move-gen correctness)
2. Material-only eval + plain minimax
3. Alpha-beta + iterative deepening
4. Quiescence search
5. Transposition table + Zobrist
6. Move ordering (TT move → captures by MVV-LVA → killers → history)
7. PSTs + mobility + king safety + pawn structure
8. Null-move pruning

## Stockfish adapter (`chess-engine-uci`)

Spawns `stockfish.exe` as a child process. Communicates via stdin/stdout using the UCI protocol.

**Initialization:** send `uci` → wait for `uciok` → send `isready` → wait for `readyok`.

**`analyze()` translates the trait call into:**

```
position fen <fen>
setoption name MultiPV value <n>
go depth <d> movetime <ms>
```

A reader task parses each `info depth ... score cp ... pv ...` line into an `AnalysisInfo` and forwards it through the mpsc channel. `stop()` sends `stop` over stdin. One Stockfish process per engine instance; lifecycle owned by the adapter struct, killed on drop.

Stockfish binary is bundled in `src-tauri/stockfish/` and shipped as a Tauri resource.

## GUI structure (Svelte)

**Three routes:**

- `/play` — board + move history sidebar + analysis panel
- `/setup` — board in edit mode + FEN input, "start from here" button
- `/engine-vs-engine` — board + both engines' analysis panels side-by-side

**Core stores (`src/lib/gamestate/`):**

- `gameStore` — current `Position`, move history (`Vec<Move>`), undo/redo cursor
- `analysisStore` — last N `AnalysisInfo` events from the active engine, keyed by multipv index
- `enginesStore` — which engines are loaded, which is "white player", which is "black player" (each can be `Human | DIY | Stockfish`)

**Board component (`src/lib/board/`):**

- SVG-based 8×8 grid
- Drag-drop with legal-move highlighting (legal moves fetched via Tauri command calling `position.legal_moves()`)
- Setup mode: drag pieces from a tray; right-click to clear; FEN textbox round-trips with the board

**Analysis panel:**

- Eval bar (vertical, left of board) — maps centipawn score to 0-100% via `2/(1+exp(-cp/400)) - 1` (Lichess-style)
- Top-N candidate moves list — each row: SAN move, score, depth, PV preview
- Updates live as `AnalysisInfo` events stream in

**Tauri ↔ frontend communication:**

- Commands (request/response): `legal_moves`, `make_move`, `load_fen`, `load_pgn`, `save_pgn`
- Events (server-push): `analysis_info`, keyed by a `search_id` so old searches' events get filtered out by the frontend

## Testing strategy

| Layer | Approach |
|---|---|
| `chess-core` move gen | **Perft** — standard test suite (positions 1-6 from Chess Programming Wiki). Must pass exactly before search work begins. |
| `chess-core` FEN/PGN | Round-trip property tests (parse → serialize → parse equals original). |
| DIY engine search | "Mate in N" test positions — engine must find the mate at depth N. Plus a small bench suite for perf regressions. |
| DIY engine eval | Symmetry test (eval of mirrored position equals `-eval` of original) — catches sign bugs. |
| UCI adapter | Unit-tested with a fake child process (`Stdio::piped()` + scripted UCI replies). |
| Engine trait conformance | Same suite runs against both `DiyEngine` and `StockfishEngine` — both find the obvious tactics. |
| Frontend | Playwright e2e for the golden path: load app → make a move → engine analysis appears → undo works. |

## Out of scope for v1

- Opening books / endgame tablebases (DIY engine plays from-scratch openings)
- Time controls / clocks (analysis-driven, not timed games)
- Online play / multiplayer
- Opening explorer, game database, training puzzles
- Engine strength beyond ~2000 Elo (no late-move reductions, futility pruning, complex eval terms)

## Risks & open questions

- **Stockfish binary distribution.** Need to confirm Tauri resource bundling handles the executable correctly on Windows. Fallback: download on first run.
- **Perft performance.** A naive bitboard implementation may not hit the perft speeds needed to make club-level depth practical. Magic bitboards (for sliding pieces) likely needed; deferred until benchmarks show it's the bottleneck.
- **Async story for in-process engine.** Tokio + a `mpsc::Receiver<AnalysisInfo>` needs to translate cleanly to Tauri events. May need a small bridge layer.
