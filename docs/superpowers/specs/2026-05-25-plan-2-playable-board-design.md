# Plan 2 — Playable Hot-Seat Board

**Date:** 2026-05-25
**Status:** Approved (design phase)
**Builds on:** Plan 1 (`chess-core` foundation, complete and merged via PR #1)
**Next:** Plan 3 — Engine API + Stockfish UCI adapter (out of scope here)

## Goal

Ship a playable hot-seat chess desktop app: open a Tauri window, click or drag pieces, take legal moves only, see history, undo/redo, load a position by FEN, save and load games as PGN, and get a banner when the game ends. No engine — Plan 3 wires that in.

This plan's purpose is to validate the full `chess-core` ↔ Tauri ↔ Svelte pipeline with a real, usable UI before any engine complexity lands on top.

## Stack

- **Backend:** Rust + Tauri 2.x (binary crate `src-tauri` alongside the existing `chess-core` workspace member)
- **Frontend:** Svelte 5 (runes) + TypeScript + Vite 5+
- **Frontend framing:** Vanilla Svelte (not SvelteKit) — SvelteKit's SSR/routing is overkill for a single-window desktop app
- **Piece graphics:** Cburnett SVG set (Wikimedia, CC-BY-SA 3.0), inlined into `Piece.svelte`
- **Board theme:** Brown / wood (`#f0d9b5` light, `#b58863` dark)
- **Testing:** Vitest for frontend stores; `#[cfg(test)]` for Rust commands

## Workspace layout

Extends what Plan 1 set up. New files marked **NEW**.

```
chess-engine/
├── Cargo.toml                          (existing — workspace root)
├── src-tauri/
│   ├── Cargo.toml                      NEW — Tauri binary manifest
│   ├── tauri.conf.json                 NEW — Tauri 2 app config
│   ├── build.rs                        NEW
│   ├── src/
│   │   ├── main.rs                     NEW — app entry + tauri::Builder
│   │   └── commands.rs                 NEW — #[tauri::command] wrappers over chess-core
│   ├── icons/                          NEW
│   └── crates/chess-core/              (existing, unchanged)
├── src/                                NEW — Svelte frontend root
│   ├── main.ts
│   ├── App.svelte                      — top-level layout
│   ├── app.css
│   └── lib/
│       ├── tauri.ts                    — typed wrappers around invoke()
│       ├── stores/
│       │   ├── game.svelte.ts          — gameStore (rune-based)
│       │   └── ui.svelte.ts            — uiStore (selection + drag state)
│       ├── board/
│       │   ├── Board.svelte            — SVG 8×8 + pointer handlers
│       │   ├── Piece.svelte            — single piece, inline Cburnett SVG
│       │   └── pieces/                 — 12 Cburnett SVG source files
│       └── panels/
│           ├── Toolbar.svelte          — FEN input + Load/Undo/Redo/PGN buttons
│           ├── HistoryPanel.svelte     — read-only SAN list, two columns
│           └── GameOverBanner.svelte
├── package.json                        NEW
├── vite.config.ts                      NEW
├── tsconfig.json                       NEW
└── ATTRIBUTION.md                      NEW — Cburnett CC-BY-SA credit
```

## Tauri commands

All commands are **stateless** — they take FEN/position data in and return new FEN/position data out. The frontend Svelte stores own the canonical game state. No `Mutex<GameState>` on the backend. This keeps `chess-core` pure and avoids any session lifecycle / resync bugs.

All commands return `Result<T, String>` (Tauri serializes `Err(String)` cleanly to JS rejections).

```rust
// src-tauri/src/commands.rs

#[derive(Serialize, Deserialize)]
pub struct MoveDto {
    pub from: String,                       // "e2"
    pub to: String,                         // "e4"
    pub promotion: Option<char>,            // 'Q' | 'R' | 'B' | 'N' or None
}

#[derive(Serialize)]
pub struct MakeMoveResult {
    pub new_fen: String,
    pub san: String,                        // SAN of the move just played
    pub outcome: Option<OutcomeDto>,        // Some(...) if the game ended
}

#[derive(Serialize)]
pub struct OutcomeDto {
    pub kind: String,                       // "Checkmate" | "Stalemate" | "FiftyMove" | "InsufficientMaterial"
    pub winner: Option<String>,             // "White" | "Black" | None
}

#[derive(Serialize)]
pub struct GameDto {
    pub tags: HashMap<String, String>,
    pub moves: Vec<MoveEntry>,
    pub result: String,                     // "1-0" | "0-1" | "1/2-1/2" | "*"
    pub final_fen: String,
}

#[derive(Serialize, Deserialize)]
pub struct MoveEntry {
    pub san: String,                        // e.g. "Nf3", "exd5", "O-O", "e8=Q+"
    pub fen_after: String,                  // FEN of the position after this move
    pub outcome: Option<OutcomeDto>,        // Some(...) only on the final move if the game ended
}

#[tauri::command] pub fn legal_moves(fen: String) -> Result<Vec<MoveDto>, String>;
#[tauri::command] pub fn make_move(fen: String, mv: MoveDto) -> Result<MakeMoveResult, String>;
#[tauri::command] pub fn validate_fen(fen: String) -> bool;
#[tauri::command] pub fn parse_pgn(text: String) -> Result<GameDto, String>;
#[tauri::command] pub fn serialize_pgn(moves: Vec<MoveDto>, tags: HashMap<String, String>) -> Result<String, String>;
```

**Why algebraic-string DTOs (`"e2"`) instead of `chess-core`'s packed `Move(u16)`:** clean JSON on the wire, no serialization quirks, frontend never sees internal encoding. Conversion happens at the command boundary.

## Frontend structure

### Stores

**`gameStore` (`src/lib/stores/game.svelte.ts`)** — source of truth for the game:

```ts
class GameStore {
  currentFen = $state(STARTING_FEN);
  history: MoveEntry[] = $state([]);                          // entry[i] = position AFTER move i+1
  cursor = $state(0);                                         // 0 = starting position; N>0 = after N moves
  outcome: OutcomeDto | null = $derived(
    this.cursor > 0 ? this.history[this.cursor - 1].outcome : null
  );

  async makeMove(mv: MoveDto): Promise<void>;                 // calls Tauri, updates history, slices any redo branch
  undo(): void;                                               // decrement cursor, restore prior FEN
  redo(): void;                                               // increment cursor, restore later FEN
  async loadFen(fen: string): Promise<void>;                  // validate, replace state, clear history (no undo back across a load)
  async loadPgn(text: string): Promise<void>;                 // parse, replace state, populate history
  reset(): void;                                              // back to starting position, clear history
}
```

**`uiStore` (`src/lib/stores/ui.svelte.ts`)** — ephemeral interaction state:

```ts
class UiStore {
  selectedSquare: string | null = $state(null);               // for click-to-move
  legalTargets: string[] = $state([]);                        // squares to highlight
  dragging: {from: string, x: number, y: number} | null = $state(null);

  selectSquare(sq: string, legalFromHere: string[]): void;
  clearSelection(): void;
  startDrag(from: string, x: number, y: number): void;
  updateDrag(x: number, y: number): void;
  endDrag(): void;
}
```

### Components

- **`App.svelte`** — top-level layout: `Toolbar` (top strip), `Board` (center), `HistoryPanel` (right sidebar). Mounts `GameOverBanner` conditionally on `gameStore.outcome !== null`.
- **`Board.svelte`** — 8×8 SVG. Owns `pointerdown`/`pointermove`/`pointerup` that drive both click-to-move and drag from a single gesture model (mousedown without move = click; with move = drag). Renders highlights from `uiStore.legalTargets`. Calls `gameStore.makeMove()` directly — does not emit events.
- **`Piece.svelte`** — one piece. Inline `<svg>` with the Cburnett path data (no HTTP requests, easy recoloring). When `uiStore.dragging?.from === this.square`, follows the cursor via `transform`.
- **`Toolbar.svelte`** — FEN text input (border turns red on invalid) + Load/Undo/Redo/Save PGN/Load PGN buttons. Bound shortcuts: `Ctrl+Z` undo, `Ctrl+Y` redo, `Ctrl+S` save PGN, `Ctrl+O` load PGN.
- **`HistoryPanel.svelte`** — read-only two-column SAN list (white, black). Auto-scrolls to current `gameStore.cursor`. Not clickable in Plan 2 — clickable history navigation deferred.
- **`GameOverBanner.svelte`** — overlay shown when `gameStore.outcome` is non-null. Text varies by outcome ("Checkmate — White wins", "Stalemate", "Draw by 50-move rule", "Draw by insufficient material"). Single "New game" button calls `gameStore.reset()`.

## Data flow — canonical example: user drags pawn e2→e4

1. `pointerdown` on e2. `Board` reads `gameStore.currentFen`, calls `legal_moves(fen)` Tauri command, filters returned moves to those starting at e2. Sets `uiStore.dragging = {from: "e2", x, y}` and `uiStore.legalTargets = [...]`.
2. `pointermove` updates `uiStore.dragging.{x, y}`. `Piece` for e2 follows cursor via `transform`.
3. `pointerup` on e4. If "e4" ∈ `legalTargets`, `Board` calls `gameStore.makeMove({from: "e2", to: "e4"})`.
4. `gameStore.makeMove` invokes the `make_move(currentFen, mv)` Tauri command. Backend: `chess_core::parse_fen` → `legal_moves` to confirm + find the matching packed `Move` → `make_move` → `serialize_fen` → `detect_outcome` → returns `{new_fen, san: "e4", outcome: null}`.
5. `gameStore` slices `history` at `cursor` (drops any pending redo branch), pushes `{san: "e4", fen: new_fen}`, sets `cursor = history.length`, updates `currentFen`. `outcome` derived rune recomputes. If it became non-null, `GameOverBanner` mounts via the `App.svelte` conditional.

Click-to-move uses the same pipeline minus the drag visual: first click sets `selectedSquare` + `legalTargets`; second click on a legal target triggers the same `gameStore.makeMove`.

## Error handling

| Failure | Handled where | User-visible result |
|---|---|---|
| Invalid FEN typed in Toolbar | `validate_fen` returns `false` | Input border red, helper text "Invalid FEN", Load disabled |
| Move from frontend that backend rejects as illegal | `make_move` returns `Err` | Console error + dev-only toast. **Should be impossible** — frontend filters via `legal_moves`; treat as a bug |
| `parse_pgn` malformed input | Returns `Err(PgnError)` → wrapped to `Err(String)` | Modal: "Could not parse PGN: \<reason\>" with paste box visible to retry |
| Native file dialog cancelled | Tauri dialog returns `None` | Silent no-op |
| File-system error on PGN save | `Err` from `fs::write` | Modal: "Could not save: \<reason\>" |
| Tauri `invoke()` rejects (backend panic) | TS wrapper in `tauri.ts` catches | Dev: console + toast. Prod: generic "Something went wrong, please restart" |

**Conventions:**

- All Rust commands return `Result<T, String>`.
- `tauri.ts` wraps each `invoke()` in a typed function so components never touch raw strings.
- No `unwrap()` in commands except where a contract has been validated (e.g., we just called `validate_fen`). Use `.map_err(|e| e.to_string())?` for chess-core errors.

## Testing

**In scope for Plan 2:**

- **Rust command unit tests** in `src-tauri/src/commands.rs` (`#[cfg(test)]`): one happy-path test per command + one failure test per command.
- **Frontend store unit tests** with Vitest: `gameStore.makeMove` updates FEN+history+cursor; `undo`/`redo` walks the cursor correctly; `loadFen` validates and replaces state; `loadPgn` populates history correctly.
- **Manual smoke checklist** (run at end of plan):
  - Open app, play a 4-move game (e.g., 1. e4 e5 2. Nf3 Nc6)
  - Undo to start, redo to end
  - Load Kiwipete FEN `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1`
  - Export current game as PGN, re-import the exported file, confirm position matches
  - Set up Fool's Mate (1. f3 e5 2. g4 Qh4#), confirm `GameOverBanner` mounts with "Checkmate — Black wins"
  - No console errors during a 10-move game

**Deferred to a later plan:**

- **Playwright e2e** — Tauri WebDriver setup is non-trivial; better as its own infra plan once we have engines to drive end-to-end.
- **Visual regression / screenshot tests** — premature without locked-in visual design.
- **Frontend perf benchmarks** — no perf concerns at this scope.

**Verification before "done":** all unit tests green; `cargo build --release` succeeds for the Tauri binary; manual checklist passes; no console errors in a 10-move game.

## Build order

Vertical slices — each task ships a runnable, manually-testable increment.

1. **Scaffold Tauri 2 + Svelte 5 + Vite.** Workspace `Cargo.toml` updated to include `src-tauri` as a new binary crate; `chess-core` added as a dep. `npm run tauri dev` opens a window showing "Hello chess-engine".
2. **Static starting position with Unicode pieces.** 8×8 SVG board in brown theme. Each square shows the right starting piece using **Unicode** chess symbols (cheap rendering check before adding asset bundling). `gameStore` exists with hardcoded starting FEN.
3. **Replace Unicode with Cburnett SVGs.** Add 12 SVG files to `src/lib/board/pieces/`, rewrite `Piece.svelte` to inline them. Add `ATTRIBUTION.md` (CC-BY-SA 3.0 credit). Pure visual swap.
4. **Click-to-move (first end-to-end interaction).** Add Tauri commands `legal_moves` + `make_move`. `uiStore` with `selectedSquare`/`legalTargets`. Click piece → highlights → click target → board re-renders from new FEN. No history yet.
5. **History sidebar.** `gameStore.history` populated on each move. `HistoryPanel.svelte` renders two-column SAN list, auto-scrolls.
6. **Undo / redo.** `gameStore.cursor` + `undo()`/`redo()` methods. Toolbar buttons + `Ctrl+Z`/`Ctrl+Y` bindings. Board re-renders from `history[cursor].fen`.
7. **Drag-and-drop.** Add `pointermove`/`pointerup` to `Board.svelte`, sharing the same `gameStore.makeMove` pipeline as click. Piece follows cursor via `transform`.
8. **FEN load.** Toolbar FEN input + `validate_fen` command. Load button replaces `gameStore` state with the new position (clears history).
9. **PGN save / load.** Tauri commands `parse_pgn` / `serialize_pgn`. Tauri `dialog` plugin for native file pickers.
10. **Game-over banner.** `make_move` return includes `Outcome`. `gameStore.outcome` derived rune. `GameOverBanner.svelte` mounts when non-null. "New game" button resets.
11. **Polish & smoke test.** Remaining keyboard shortcuts (`Ctrl+S` save, `Ctrl+O` load), tab order, README update, manual smoke checklist from Testing section.

## Done state

When Plan 2 completes:

- `npm run tauri dev` opens a working chess app on Windows
- `cargo build --release` builds the desktop binary
- All Rust command unit tests + all Vitest store tests pass
- Manual smoke checklist passes
- You can play a full hot-seat game, undo to start, load Kiwipete, export/re-import PGN, and get a banner on Fool's Mate
- No engine code — that's Plan 3

## Out of scope for Plan 2

- Any engine (DIY or Stockfish) — Plan 3
- `/setup` route (board editor with drag-from-tray) — Plan 4+
- `/engine-vs-engine` route — needs engines, so Plan 3+
- Clickable history navigation (jump to past position) — small follow-up after Plan 2 if wanted
- Theme picker / light-vs-dark UI chrome — design polish plan
- Alternative piece sets / runtime swap — design polish plan
- Online play, opening books, clocks, puzzles, training, opening explorer

## Risks & open questions

- **Tauri 2 + Svelte 5 templates on Windows.** `create-tauri-app` template combinations evolve; the scaffold task may need slight adjustment if the generator output drifts. Mitigation: pin to the latest stable template and document the exact command used.
- **Cburnett SVG licensing reading.** CC-BY-SA 3.0 attribution required in `ATTRIBUTION.md` and visible in app's About (Plan 2 ships `ATTRIBUTION.md`; in-app About panel deferred to polish plan).
- **Pointer events vs HTML5 drag-drop for piece dragging.** Plan 2 uses Pointer Events (uniform across mouse/pen/touch, easier to share gesture model with click). HTML5 drag-drop is rejected — awkward for SVG, hard to style.
- **Native file dialog plugin.** Tauri 2's `dialog` plugin must be added to `Cargo.toml` + `tauri.conf.json` capabilities. Trivial but easy to forget.
