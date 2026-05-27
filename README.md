# chess-engine

A desktop chess application built with Tauri + Svelte, backed by a from-scratch
Rust chess engine (in progress).

## Status

- **Plan 1: chess-core** — complete. Bitboard rules library, FEN/PGN, perft-validated.
- **Plan 2: playable board** — complete. Tauri + Svelte UI; hot-seat play with undo/redo, FEN load, PGN save/load, game-over banner.
- **Plan 3: Stockfish integration** — complete. Engine picker per side (Human / Stockfish + Skill 0-20). Stockfish auto-plays. Always-on analysis with eval bar + top-3 candidate moves.
- **Plan 4+: DIY engine** — upcoming. Build a club-strength engine from scratch in Rust (perft → material → alpha-beta → quiescence → TT → ordering → eval → null-move).

## Run

```
npm install
npm run tauri dev
```

## Bundled engine

The app ships with Stockfish 17 (GPL-3.0) at `src-tauri/binaries/`. It's launched as a child process and communicated with via UCI over stdin/stdout. Skill Level 0 plays around 1100 Elo; Skill Level 20 is full strength.

## Limitations (Plan 3)

- Pawn promotion auto-defaults to Queen (no picker UI yet).
- History sidebar is read-only — clickable navigation to past positions is a planned follow-up.
- No DIY engine yet — Plan 4+.
- No engine-vs-engine mode yet — Plan 4+.
- No "hints off" / training mode toggle yet.

## Attribution

- Chess piece graphics: Cburnett (Wikimedia, CC-BY-SA 3.0). See `ATTRIBUTION.md`.
- Stockfish 17: GPL-3.0, https://stockfishchess.org/
