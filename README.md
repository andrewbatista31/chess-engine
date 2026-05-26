# chess-engine

A desktop chess application built with Tauri + Svelte, backed by a from-scratch
Rust chess engine (in progress).

## Status

- **Plan 1: chess-core** — complete. Bitboard rules library, FEN/PGN, perft-validated.
- **Plan 2: playable board** — complete. Tauri + Svelte UI; hot-seat play with undo/redo, FEN load, PGN save/load, game-over banner.
- **Plan 3+: engines** — upcoming. Stockfish UCI integration, then DIY engine.

## Run

```
npm install
npm run tauri dev
```

## Limitations (Plan 2)

- Pawn promotion auto-defaults to Queen (no picker UI yet).
- History sidebar is read-only — clickable navigation to past positions is a planned follow-up.
- No engine — coming in Plan 3.

## Attribution

Chess piece graphics: Cburnett (Wikimedia, CC-BY-SA 3.0). See `ATTRIBUTION.md`.
