# chess-engine

A desktop chess application with a from-scratch club-strength chess engine, plus Stockfish integration for comparison and analysis.

**Status:** design phase. See [docs/superpowers/specs/2026-05-20-chess-engine-design.md](docs/superpowers/specs/2026-05-20-chess-engine-design.md).

## Stack

- Rust (engine, rules, Tauri commands)
- Svelte + TypeScript (frontend)
- Tauri (desktop app shell)
- Stockfish (bundled, UCI)

## Features (planned for v1)

- Play yourself (free play, both sides local)
- Play vs engine (DIY or Stockfish, either side)
- Engine vs engine (DIY vs Stockfish)
- Position setup via FEN or board editor
- Live analysis: top-N candidate moves, principal variations, eval bar
- PGN save/load, undo/redo
