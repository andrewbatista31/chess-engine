# Plan 2 — Playable Hot-Seat Board Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Tauri 2 + Svelte 5 desktop chess app on top of the existing `chess-core` crate. User can play hot-seat (click or drag), undo/redo, load by FEN, save/load PGN, and see a game-over banner. No engine.

**Architecture:** Stateless backend — Tauri commands take FEN in, return FEN out. Svelte 5 stores own canonical game state in the frontend. `chess-core` remains pure (one small additive change: expose `move_to_san` in the prelude). Vertical slices: each task ships a runnable, manually-testable increment.

**Tech Stack:** Tauri 2.x, Svelte 5 (runes), Vite 5+, TypeScript 5, Vitest, `chess-core` (existing), `tauri-plugin-dialog` 2.x, Cburnett SVG piece set (CC-BY-SA 3.0).

**Spec:** `docs/superpowers/specs/2026-05-25-plan-2-playable-board-design.md` is the source of truth — re-read it before each task.

**Known Plan 2 limitations (deferred):**
- Pawn promotion auto-defaults to Queen (no picker UI). Documented in README; promotion picker is a follow-up polish task.
- History sidebar is read-only (no click-to-jump). Documented.
- No in-app About panel with attribution (lives only in `ATTRIBUTION.md` for Plan 2).

---

## File structure

Created across this plan:

```
chess-engine/
├── Cargo.toml                                  MODIFY: add "src-tauri" to workspace members
├── package.json                                NEW
├── vite.config.ts                              NEW
├── tsconfig.json                               NEW
├── tsconfig.node.json                          NEW
├── index.html                                  NEW
├── ATTRIBUTION.md                              NEW
├── src-tauri/
│   ├── Cargo.toml                              NEW
│   ├── tauri.conf.json                         NEW
│   ├── build.rs                                NEW
│   ├── capabilities/
│   │   └── default.json                        NEW
│   ├── icons/                                  NEW (32x32, 128x128, icon.ico)
│   ├── src/
│   │   ├── main.rs                             NEW
│   │   └── commands.rs                         NEW
│   └── crates/chess-core/src/
│       ├── pgn.rs                              MODIFY: `pub fn move_to_san`
│       └── lib.rs                              MODIFY: re-export `move_to_san` in prelude
└── src/
    ├── main.ts                                 NEW
    ├── App.svelte                              NEW (rewritten across tasks)
    ├── app.css                                 NEW
    ├── vite-env.d.ts                           NEW
    └── lib/
        ├── tauri.ts                            NEW
        ├── stores/
        │   ├── game.svelte.ts                  NEW (grown across tasks)
        │   ├── game.test.ts                    NEW
        │   ├── ui.svelte.ts                    NEW
        │   └── ui.test.ts                      NEW
        ├── board/
        │   ├── Board.svelte                    NEW
        │   ├── Piece.svelte                    NEW
        │   └── pieces/                         NEW: wK.svg wQ.svg wR.svg wB.svg wN.svg wP.svg bK.svg bQ.svg bR.svg bB.svg bN.svg bP.svg
        └── panels/
            ├── Toolbar.svelte                  NEW
            ├── HistoryPanel.svelte             NEW
            └── GameOverBanner.svelte           NEW
```

---

### Task 1: Scaffold Tauri 2 + Svelte 5 + Vite

**Goal:** `npm run tauri dev` opens a Tauri window showing "Hello chess-engine". Workspace builds. No board, no chess logic.

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tsconfig.node.json`, `index.html`, `src/main.ts`, `src/App.svelte`, `src/app.css`, `src/vite-env.d.ts`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/capabilities/default.json`
- Modify: `Cargo.toml` (workspace root — add `src-tauri` to members)
- Note: app icons (`src-tauri/icons/*`) — copy placeholder PNG/ICO from the Tauri default template (see Step 7). Icons can be replaced later.

- [ ] **Step 1: Add `src-tauri` to the Cargo workspace**

Edit `Cargo.toml` (workspace root):

```toml
[workspace]
resolver = "2"
members = ["src-tauri", "src-tauri/crates/*"]

[workspace.package]
edition = "2021"
rust-version = "1.75"
license = "MIT"

[workspace.dependencies]
arrayvec = "0.7"
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Create `src-tauri/Cargo.toml`**

```toml
[package]
name = "chess-engine-app"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[lib]
name = "chess_engine_app_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[[bin]]
name = "chess-engine-app"
path = "src/main.rs"

[build-dependencies]
tauri-build = { workspace = true }

[dependencies]
tauri = { workspace = true, features = [] }
serde = { workspace = true }
serde_json = { workspace = true }
chess-core = { path = "crates/chess-core" }
```

- [ ] **Step 3: Create `src-tauri/build.rs`**

```rust
fn main() {
    tauri_build::build();
}
```

- [ ] **Step 4: Create `src-tauri/tauri.conf.json`**

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "chess-engine",
  "version": "0.1.0",
  "identifier": "com.batista.chess-engine",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "chess-engine",
        "width": 1100,
        "height": 760,
        "minWidth": 800,
        "minHeight": 600
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 5: Create `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "default capabilities",
  "windows": ["main"],
  "permissions": ["core:default"]
}
```

- [ ] **Step 6: Create `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 7: Place placeholder app icons**

The Tauri build will fail without icons. From the [Tauri docs](https://tauri.app/start/create-project/) or the `create-tauri-app` default template, copy three files into `src-tauri/icons/`:
- `32x32.png`
- `128x128.png`
- `icon.ico`

Any valid PNG/ICO will do for now (Tauri's default chess-piece-free icon is fine). They can be branded later.

- [ ] **Step 8: Create root `package.json`**

```json
{
  "name": "chess-engine",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "test": "vitest run",
    "test:watch": "vitest"
  },
  "dependencies": {
    "@tauri-apps/api": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^4",
    "@tauri-apps/cli": "^2",
    "svelte": "^5",
    "svelte-check": "^4",
    "tslib": "^2",
    "typescript": "^5",
    "vite": "^5",
    "vitest": "^2",
    "@testing-library/svelte": "^5",
    "jsdom": "^25"
  }
}
```

- [ ] **Step 9: Create `vite.config.ts`**

```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
  test: {
    environment: "jsdom",
    globals: true,
  },
});
```

- [ ] **Step 10: Create `tsconfig.json`**

```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "allowSyntheticDefaultImports": true,
    "esModuleInterop": true,
    "strict": true,
    "noImplicitAny": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "moduleResolution": "Bundler",
    "allowImportingTsExtensions": true,
    "noEmit": true,
    "isolatedModules": true,
    "skipLibCheck": true,
    "types": ["svelte", "vitest/globals"]
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

- [ ] **Step 11: Create `tsconfig.node.json`**

```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "Bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts"]
}
```

- [ ] **Step 12: Create `index.html`**

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>chess-engine</title>
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

- [ ] **Step 13: Create `src/vite-env.d.ts`**

```ts
/// <reference types="svelte" />
/// <reference types="vite/client" />
```

- [ ] **Step 14: Create `src/main.ts`**

```ts
import { mount } from "svelte";
import App from "./App.svelte";
import "./app.css";

const app = mount(App, { target: document.getElementById("app")! });

export default app;
```

- [ ] **Step 15: Create `src/App.svelte`**

```svelte
<main>
  <h1>Hello chess-engine</h1>
  <p>Plan 2 scaffold — chess board coming in Task 2.</p>
</main>
```

- [ ] **Step 16: Create `src/app.css`**

```css
:root {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
  background: #2b2b2b;
  color: #e8e8e8;
}

html, body, #app { height: 100%; margin: 0; }

main {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 1rem;
}
```

- [ ] **Step 17: Install dependencies**

Run: `npm install`
Expected: `node_modules` populated; no errors.

- [ ] **Step 18: Verify the app launches**

Run: `npm run tauri dev`
Expected: A Tauri window opens showing the heading "Hello chess-engine". Closing the window exits the dev server.

If `npm run tauri dev` fails with "tauri-cli not found", verify `@tauri-apps/cli` is in devDependencies and re-run `npm install`.

- [ ] **Step 19: Commit**

```powershell
git add Cargo.toml package.json vite.config.ts tsconfig.json tsconfig.node.json index.html src-tauri/ src/ ATTRIBUTION.md
git commit -m "feat: scaffold Tauri 2 + Svelte 5 + Vite shell"
```

(`ATTRIBUTION.md` is created in Task 3 — exclude it if it doesn't exist yet.)

---

### Task 2: Static starting position with Unicode pieces

**Goal:** Show an 8×8 brown-themed SVG board with the starting position rendered using Unicode chess symbols. No interaction. `gameStore` exists with the starting FEN.

**Files:**
- Modify: `src/App.svelte`
- Create: `src/lib/board/Board.svelte`, `src/lib/board/Piece.svelte`
- Create: `src/lib/stores/game.svelte.ts`

- [ ] **Step 1: Create `src/lib/stores/game.svelte.ts`** (minimal version — just `currentFen`)

```ts
export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  currentFen = $state(STARTING_FEN);
}

export const gameStore = new GameStore();
```

- [ ] **Step 2: Create `src/lib/board/Piece.svelte`** (Unicode version — temporary)

```svelte
<script lang="ts">
  interface Props { piece: string; size: number; }
  let { piece, size }: Props = $props();

  const SYMBOLS: Record<string, string> = {
    K: "♔", Q: "♕", R: "♖", B: "♗", N: "♘", P: "♙",
    k: "♚", q: "♛", r: "♜", b: "♝", n: "♞", p: "♟",
  };
  const isWhite = piece === piece.toUpperCase();
</script>

<text
  x="50%"
  y="50%"
  font-size={size * 0.75}
  text-anchor="middle"
  dominant-baseline="central"
  font-family='"Segoe UI Symbol", "DejaVu Sans", sans-serif'
  fill={isWhite ? "#fff" : "#000"}
  stroke={isWhite ? "#000" : "none"}
  stroke-width={isWhite ? 1 : 0}
>{SYMBOLS[piece]}</text>
```

- [ ] **Step 3: Create `src/lib/board/Board.svelte`**

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import Piece from "./Piece.svelte";

  const SQUARE_SIZE = 64;

  // Parse the board portion of a FEN into an 8×8 array (rank 8 first).
  function parseBoard(fen: string): string[][] {
    const rows = fen.split(" ")[0].split("/");
    return rows.map((row) => {
      const out: string[] = [];
      for (const ch of row) {
        if (/\d/.test(ch)) for (let i = 0; i < +ch; i++) out.push(".");
        else out.push(ch);
      }
      return out;
    });
  }

  const board = $derived(parseBoard(gameStore.currentFen));
  const fileChars = ["a", "b", "c", "d", "e", "f", "g", "h"];
</script>

<svg
  width={SQUARE_SIZE * 8}
  height={SQUARE_SIZE * 8}
  viewBox="0 0 {SQUARE_SIZE * 8} {SQUARE_SIZE * 8}"
  class="board"
>
  {#each board as row, rankIdx}
    {#each row as piece, fileIdx}
      {@const isLight = (rankIdx + fileIdx) % 2 === 0}
      {@const x = fileIdx * SQUARE_SIZE}
      {@const y = rankIdx * SQUARE_SIZE}
      <rect
        {x} {y}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        fill={isLight ? "#f0d9b5" : "#b58863"}
      />
      {#if piece !== "."}
        <svg {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE}>
          <Piece {piece} size={SQUARE_SIZE} />
        </svg>
      {/if}
    {/each}
  {/each}
</svg>

<style>
  .board { display: block; user-select: none; }
</style>
```

- [ ] **Step 4: Update `src/App.svelte`**

```svelte
<script lang="ts">
  import Board from "./lib/board/Board.svelte";
</script>

<main>
  <Board />
</main>
```

- [ ] **Step 5: Verify visually**

Run: `npm run tauri dev`
Expected: The Tauri window now shows a brown 8×8 board with the starting position rendered using Unicode chess symbols. White at the bottom (rank 1), black at top (rank 8). No interaction yet.

- [ ] **Step 6: Write a Vitest unit test for `parseBoard`**

Extract the function (or duplicate it) into a testable helper. Refactor `Board.svelte`: move `parseBoard` into `src/lib/board/fen-board.ts`:

`src/lib/board/fen-board.ts`:

```ts
export function parseBoard(fen: string): string[][] {
  const rows = fen.split(" ")[0].split("/");
  return rows.map((row) => {
    const out: string[] = [];
    for (const ch of row) {
      if (/\d/.test(ch)) for (let i = 0; i < +ch; i++) out.push(".");
      else out.push(ch);
    }
    return out;
  });
}
```

Update `Board.svelte` to import: `import { parseBoard } from "./fen-board.ts";` and delete the local copy.

`src/lib/board/fen-board.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { parseBoard } from "./fen-board.ts";

describe("parseBoard", () => {
  it("parses the starting position", () => {
    const fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const b = parseBoard(fen);
    expect(b).toHaveLength(8);
    expect(b[0]).toEqual(["r","n","b","q","k","b","n","r"]);
    expect(b[1].every((p) => p === "p")).toBe(true);
    expect(b[2].every((p) => p === ".")).toBe(true);
    expect(b[6].every((p) => p === "P")).toBe(true);
    expect(b[7]).toEqual(["R","N","B","Q","K","B","N","R"]);
  });

  it("handles empty squares encoded as digits", () => {
    const b = parseBoard("8/8/8/3k4/3K4/8/8/8 w - - 0 1");
    expect(b[3]).toEqual([".",".",".","k",".",".",".","."]);
    expect(b[4]).toEqual([".",".",".","K",".",".",".","."]);
  });
});
```

- [ ] **Step 7: Run tests**

Run: `npm test`
Expected: 2 tests pass.

- [ ] **Step 8: Commit**

```powershell
git add src/
git commit -m "feat(ui): static starting position with Unicode pieces + gameStore"
```

---

### Task 3: Replace Unicode pieces with Cburnett SVGs

**Goal:** Bundle Cburnett SVG piece files; rewrite `Piece.svelte` to inline them. Pure visual swap. Add `ATTRIBUTION.md`.

**Files:**
- Create: `src/lib/board/pieces/{wK,wQ,wR,wB,wN,wP,bK,bQ,bR,bB,bN,bP}.svg` (12 files)
- Modify: `src/lib/board/Piece.svelte`
- Create: `ATTRIBUTION.md`

- [ ] **Step 1: Download Cburnett SVG piece files**

The Cburnett (Colin M.L. Burnett) SVG chess pieces are CC-BY-SA 3.0 and available on Wikimedia Commons. The lichess `lila` repository mirrors them with consistent naming.

Download the 12 files from one of:
- **Wikimedia Commons:** search "Chess [piece] [color]t45.svg" e.g. `Chess_klt45.svg` (white king), `Chess_qdt45.svg` (black queen). Files named with format `Chess_[piece][color]t45.svg` where piece ∈ {k,q,r,b,n,p} and color ∈ {l,d}.
- **lichess-org/lila on GitHub:** `public/piece/cburnett/` contains them already named `wK.svg`, `wQ.svg`, etc.

Save them to `src/lib/board/pieces/` with **exactly** these filenames:
- `wK.svg`, `wQ.svg`, `wR.svg`, `wB.svg`, `wN.svg`, `wP.svg`
- `bK.svg`, `bQ.svg`, `bR.svg`, `bB.svg`, `bN.svg`, `bP.svg`

Each file is a standalone `<svg>` document, ~3-8 KB.

Verify: `ls src/lib/board/pieces/` shows 12 `.svg` files.

- [ ] **Step 2: Configure Vite to import SVGs as URL strings**

Vite imports `.svg` as URL strings by default — no config needed. We will use `import wK from "./pieces/wK.svg"` then render via `<image href={wK} />` inside the board SVG.

- [ ] **Step 3: Rewrite `src/lib/board/Piece.svelte`**

```svelte
<script lang="ts">
  import wK from "./pieces/wK.svg";
  import wQ from "./pieces/wQ.svg";
  import wR from "./pieces/wR.svg";
  import wB from "./pieces/wB.svg";
  import wN from "./pieces/wN.svg";
  import wP from "./pieces/wP.svg";
  import bK from "./pieces/bK.svg";
  import bQ from "./pieces/bQ.svg";
  import bR from "./pieces/bR.svg";
  import bB from "./pieces/bB.svg";
  import bN from "./pieces/bN.svg";
  import bP from "./pieces/bP.svg";

  interface Props { piece: string; size: number; }
  let { piece, size }: Props = $props();

  const URLS: Record<string, string> = {
    K: wK, Q: wQ, R: wR, B: wB, N: wN, P: wP,
    k: bK, q: bQ, r: bR, b: bB, n: bN, p: bP,
  };
</script>

<image href={URLS[piece]} width={size} height={size} />
```

- [ ] **Step 4: Create `ATTRIBUTION.md`**

```markdown
# Attribution

## Chess piece graphics

The SVG chess pieces in `src/lib/board/pieces/` are the work of
**Colin M.L. Burnett** ("Cburnett") and are licensed under
[CC BY-SA 3.0](https://creativecommons.org/licenses/by-sa/3.0/).

Source: https://commons.wikimedia.org/wiki/Category:SVG_chess_pieces

Distributing this project requires preserving this attribution and
distributing modifications to the piece files under the same license.
```

- [ ] **Step 5: Verify visually**

Run: `npm run tauri dev`
Expected: The board now shows the familiar Cburnett SVG pieces (the lichess-style chess pieces) instead of Unicode characters. Layout and positions unchanged.

- [ ] **Step 6: Run tests**

Run: `npm test`
Expected: 2 tests still pass (the `parseBoard` test is unaffected).

- [ ] **Step 7: Commit**

```powershell
git add src/lib/board/pieces/ src/lib/board/Piece.svelte ATTRIBUTION.md
git commit -m "feat(ui): swap Unicode pieces for Cburnett SVGs + CC-BY-SA attribution"
```

---

### Task 4: Click-to-move (first end-to-end interaction)

**Goal:** Add Tauri commands `legal_moves` and `make_move`. Click a piece → legal destinations highlight → click one to move. The board re-renders from the new FEN returned by the backend. No history yet (that's Task 5).

**Sub-task 4a — expose `move_to_san` from chess-core:**

- [ ] **Step 1: Make `move_to_san` public in `chess-core`**

Modify `src-tauri/crates/chess-core/src/pgn.rs`:

Change `fn move_to_san(pos: &Position, m: Move) -> String {` to `pub fn move_to_san(pos: &Position, m: Move) -> String {`.

- [ ] **Step 2: Re-export `move_to_san` in the prelude**

Modify `src-tauri/crates/chess-core/src/lib.rs`, update the `pgn` re-export line:

```rust
pub use crate::pgn::{move_to_san, parse_pgn, serialize_pgn, Game, PgnError};
```

- [ ] **Step 3: Verify chess-core still passes**

Run: `cargo test -p chess-core`
Expected: all existing tests still pass (no behavior change, just visibility).

- [ ] **Step 4: Commit the chess-core change separately**

```powershell
git add src-tauri/crates/chess-core/
git commit -m "feat(chess-core): expose move_to_san in prelude for SAN-from-move use"
```

**Sub-task 4b — Tauri command DTOs and helpers:**

- [ ] **Step 5: Create `src-tauri/src/commands.rs`**

```rust
use chess_core::prelude as cc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveDto {
    pub from: String,
    pub to: String,
    pub promotion: Option<char>,
}

#[derive(Serialize, Debug, Clone)]
pub struct OutcomeDto {
    pub kind: String,
    pub winner: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
pub struct MakeMoveResult {
    pub new_fen: String,
    pub san: String,
    pub outcome: Option<OutcomeDto>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveEntry {
    pub san: String,
    pub fen_after: String,
    pub outcome: Option<OutcomeDto>,
}

#[derive(Serialize, Debug, Clone)]
pub struct GameDto {
    pub tags: HashMap<String, String>,
    pub moves: Vec<MoveEntry>,
    pub result: String,
    pub final_fen: String,
}

fn parse_square(s: &str) -> Option<cc::Square> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 { return None; }
    let f = cc::File::from_index(bytes[0].wrapping_sub(b'a'))?;
    let r = cc::Rank::from_index(bytes[1].wrapping_sub(b'1'))?;
    Some(cc::Square::new(f, r))
}

fn square_str(sq: cc::Square) -> String {
    let mut s = String::with_capacity(2);
    s.push((b'a' + sq.file() as u8) as char);
    s.push((b'1' + sq.rank() as u8) as char);
    s
}

fn promo_kind_of_flag(flag: cc::MoveFlag) -> Option<cc::PieceKind> {
    use cc::MoveFlag::*;
    use cc::PieceKind::*;
    Some(match flag {
        PromoKnight | PromoCaptureN => Knight,
        PromoBishop | PromoCaptureB => Bishop,
        PromoRook | PromoCaptureR   => Rook,
        PromoQueen | PromoCaptureQ  => Queen,
        _ => return None,
    })
}

fn move_to_dto(m: cc::Move) -> MoveDto {
    let promotion = match promo_kind_of_flag(m.flag()) {
        Some(cc::PieceKind::Queen)  => Some('Q'),
        Some(cc::PieceKind::Rook)   => Some('R'),
        Some(cc::PieceKind::Bishop) => Some('B'),
        Some(cc::PieceKind::Knight) => Some('N'),
        _ => None,
    };
    MoveDto { from: square_str(m.from()), to: square_str(m.to()), promotion }
}

fn outcome_to_dto(o: cc::Outcome, side_to_move_after: cc::Color) -> OutcomeDto {
    use cc::Outcome::*;
    match o {
        Checkmate => OutcomeDto {
            kind: "Checkmate".into(),
            winner: Some(match side_to_move_after {
                cc::Color::White => "Black".into(),
                cc::Color::Black => "White".into(),
            }),
        },
        Stalemate => OutcomeDto { kind: "Stalemate".into(), winner: None },
        FiftyMoveRule => OutcomeDto { kind: "FiftyMove".into(), winner: None },
        InsufficientMaterial => OutcomeDto { kind: "InsufficientMaterial".into(), winner: None },
    }
}

#[tauri::command]
pub fn legal_moves(fen: String) -> Result<Vec<MoveDto>, String> {
    let pos = cc::parse_fen(&fen).map_err(|e| format!("Invalid FEN: {e:?}"))?;
    Ok(cc::legal_moves(&pos).into_iter().map(move_to_dto).collect())
}

#[tauri::command]
pub fn make_move(fen: String, mv: MoveDto) -> Result<MakeMoveResult, String> {
    let mut pos = cc::parse_fen(&fen).map_err(|e| format!("Invalid FEN: {e:?}"))?;
    let from_sq = parse_square(&mv.from).ok_or_else(|| format!("Invalid 'from': {}", mv.from))?;
    let to_sq   = parse_square(&mv.to).ok_or_else(|| format!("Invalid 'to': {}", mv.to))?;
    let want_promo: Option<cc::PieceKind> = mv.promotion.and_then(|c| match c {
        'Q' => Some(cc::PieceKind::Queen),
        'R' => Some(cc::PieceKind::Rook),
        'B' => Some(cc::PieceKind::Bishop),
        'N' => Some(cc::PieceKind::Knight),
        _ => None,
    });

    let candidate = cc::legal_moves(&pos).into_iter().find(|m| {
        if m.from() != from_sq || m.to() != to_sq { return false; }
        if m.flag().is_promotion() {
            // For promotion moves, require a matching promotion choice; default Queen if absent.
            promo_kind_of_flag(m.flag()) == Some(want_promo.unwrap_or(cc::PieceKind::Queen))
        } else {
            want_promo.is_none()
        }
    }).ok_or_else(|| format!("Illegal move: {} -> {}", mv.from, mv.to))?;

    let san = cc::move_to_san(&pos, candidate);
    cc::make_move(&mut pos, candidate);
    let new_fen = cc::serialize_fen(&pos);
    let outcome = cc::detect_outcome(&pos).map(|o| outcome_to_dto(o, pos.side_to_move));
    Ok(MakeMoveResult { new_fen, san, outcome })
}

#[cfg(test)]
mod tests {
    use super::*;
    const START: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn legal_moves_starting_position_is_20() {
        let moves = legal_moves(START.into()).unwrap();
        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn legal_moves_rejects_garbage_fen() {
        assert!(legal_moves("not a fen".into()).is_err());
    }

    #[test]
    fn make_move_e2e4_advances_position() {
        let result = make_move(
            START.into(),
            MoveDto { from: "e2".into(), to: "e4".into(), promotion: None },
        ).unwrap();
        assert!(result.new_fen.starts_with("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b"));
        assert_eq!(result.san, "e4");
        assert!(result.outcome.is_none());
    }

    #[test]
    fn make_move_rejects_illegal_e2e5() {
        let err = make_move(
            START.into(),
            MoveDto { from: "e2".into(), to: "e5".into(), promotion: None },
        ).unwrap_err();
        assert!(err.contains("Illegal move"));
    }

    #[test]
    fn make_move_detects_checkmate() {
        // Fool's Mate position just before Qh4#: White has played 1. f3 e5 2. g4
        let fen = "rnbqkbnr/pppp1ppp/8/4p3/6P1/5P2/PPPPP2P/RNBQKBNR b KQkq g3 0 2";
        let result = make_move(
            fen.into(),
            MoveDto { from: "d8".into(), to: "h4".into(), promotion: None },
        ).unwrap();
        assert_eq!(result.san, "Qh4#");
        let outcome = result.outcome.expect("checkmate expected");
        assert_eq!(outcome.kind, "Checkmate");
        assert_eq!(outcome.winner.as_deref(), Some("Black"));
    }
}
```

- [ ] **Step 6: Wire commands into `src-tauri/src/main.rs`**

Replace `src-tauri/src/main.rs`:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::legal_moves,
            commands::make_move,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 7: Run Rust command tests**

Run: `cargo test -p chess-engine-app`
Expected: 5 tests pass (`legal_moves_starting_position_is_20`, `legal_moves_rejects_garbage_fen`, `make_move_e2e4_advances_position`, `make_move_rejects_illegal_e2e5`, `make_move_detects_checkmate`).

**Sub-task 4c — typed Tauri wrappers and frontend wiring:**

- [ ] **Step 8: Create `src/lib/tauri.ts`**

```ts
import { invoke } from "@tauri-apps/api/core";

export interface MoveDto {
  from: string;
  to: string;
  promotion: "Q" | "R" | "B" | "N" | null;
}

export interface OutcomeDto {
  kind: "Checkmate" | "Stalemate" | "FiftyMove" | "InsufficientMaterial";
  winner: "White" | "Black" | null;
}

export interface MakeMoveResult {
  new_fen: string;
  san: string;
  outcome: OutcomeDto | null;
}

export interface MoveEntry {
  san: string;
  fen_after: string;
  outcome: OutcomeDto | null;
}

export interface GameDto {
  tags: Record<string, string>;
  moves: MoveEntry[];
  result: string;
  final_fen: string;
}

export const tauri = {
  legalMoves(fen: string): Promise<MoveDto[]> {
    return invoke("legal_moves", { fen });
  },
  makeMove(fen: string, mv: MoveDto): Promise<MakeMoveResult> {
    return invoke("make_move", { fen, mv });
  },
};
```

- [ ] **Step 9: Create `src/lib/stores/ui.svelte.ts`**

```ts
import type { MoveDto } from "../tauri.ts";

class UiStore {
  selectedSquare: string | null = $state(null);
  legalTargets: string[] = $state([]);
  /** Cache of legal moves keyed by `from` square, so the second click can find the matching MoveDto without another round trip. */
  legalByFrom: Map<string, MoveDto[]> = $state(new Map());

  selectSquare(sq: string, legalFromHere: MoveDto[]): void {
    this.selectedSquare = sq;
    this.legalTargets = legalFromHere.map((m) => m.to);
    this.legalByFrom = new Map([[sq, legalFromHere]]);
  }

  clearSelection(): void {
    this.selectedSquare = null;
    this.legalTargets = [];
    this.legalByFrom = new Map();
  }

  /** Returns the MoveDto matching the currently selected piece moving to `to`, or null. */
  findMove(to: string): MoveDto | null {
    if (!this.selectedSquare) return null;
    const candidates = this.legalByFrom.get(this.selectedSquare) ?? [];
    // Auto-default promotion to Queen for Plan 2.
    const exact = candidates.find((m) => m.to === to && (m.promotion === null || m.promotion === "Q"));
    return exact ?? null;
  }
}

export const uiStore = new UiStore();
```

- [ ] **Step 10: Extend `gameStore` with `makeMove`**

Replace `src/lib/stores/game.svelte.ts`:

```ts
import { tauri, type MoveDto, type OutcomeDto } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  currentFen = $state(STARTING_FEN);

  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    this.currentFen = result.new_fen;
  }
}

export const gameStore = new GameStore();
```

- [ ] **Step 11: Add click handling to `src/lib/board/Board.svelte`**

Replace the full file:

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { uiStore } from "../stores/ui.svelte.ts";
  import { tauri } from "../tauri.ts";
  import { parseBoard } from "./fen-board.ts";
  import Piece from "./Piece.svelte";

  const SQUARE_SIZE = 64;
  const board = $derived(parseBoard(gameStore.currentFen));
  const sideToMove = $derived(gameStore.currentFen.split(" ")[1]);

  function squareName(rankIdx: number, fileIdx: number): string {
    const file = "abcdefgh"[fileIdx];
    const rank = (8 - rankIdx).toString();
    return file + rank;
  }

  function isOwnPiece(piece: string): boolean {
    if (piece === ".") return false;
    const isWhite = piece === piece.toUpperCase();
    return (isWhite && sideToMove === "w") || (!isWhite && sideToMove === "b");
  }

  async function onSquareClick(rankIdx: number, fileIdx: number, piece: string) {
    const sq = squareName(rankIdx, fileIdx);

    // Clicking a legal target completes a move.
    if (uiStore.selectedSquare && uiStore.legalTargets.includes(sq)) {
      const mv = uiStore.findMove(sq);
      if (mv) {
        await gameStore.makeMove(mv);
        uiStore.clearSelection();
        return;
      }
    }

    // Clicking own piece selects it.
    if (isOwnPiece(piece)) {
      const all = await tauri.legalMoves(gameStore.currentFen);
      const fromHere = all.filter((m) => m.from === sq);
      if (fromHere.length > 0) {
        uiStore.selectSquare(sq, fromHere);
        return;
      }
    }

    // Anything else clears selection.
    uiStore.clearSelection();
  }
</script>

<svg
  width={SQUARE_SIZE * 8}
  height={SQUARE_SIZE * 8}
  viewBox="0 0 {SQUARE_SIZE * 8} {SQUARE_SIZE * 8}"
  class="board"
>
  {#each board as row, rankIdx}
    {#each row as piece, fileIdx}
      {@const isLight = (rankIdx + fileIdx) % 2 === 0}
      {@const sq = squareName(rankIdx, fileIdx)}
      {@const x = fileIdx * SQUARE_SIZE}
      {@const y = rankIdx * SQUARE_SIZE}
      {@const isSelected = uiStore.selectedSquare === sq}
      {@const isTarget = uiStore.legalTargets.includes(sq)}
      <rect
        {x} {y}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        fill={isLight ? "#f0d9b5" : "#b58863"}
        onclick={() => onSquareClick(rankIdx, fileIdx, piece)}
        style:cursor="pointer"
      />
      {#if isSelected}
        <rect {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} fill="rgba(255,255,0,0.35)" pointer-events="none" />
      {/if}
      {#if isTarget}
        <circle
          cx={x + SQUARE_SIZE / 2}
          cy={y + SQUARE_SIZE / 2}
          r={SQUARE_SIZE * (piece === "." ? 0.15 : 0.45)}
          fill={piece === "." ? "rgba(0,0,0,0.35)" : "none"}
          stroke={piece === "." ? "none" : "rgba(0,0,0,0.55)"}
          stroke-width={piece === "." ? 0 : 4}
          pointer-events="none"
        />
      {/if}
      {#if piece !== "."}
        <svg {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} pointer-events="none">
          <Piece {piece} size={SQUARE_SIZE} />
        </svg>
      {/if}
    {/each}
  {/each}
</svg>

<style>
  .board { display: block; user-select: none; }
</style>
```

- [ ] **Step 12: Add Vitest test for `gameStore.makeMove`**

Create `src/lib/stores/game.test.ts`:

```ts
import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("../tauri.ts", () => ({
  tauri: {
    legalMoves: vi.fn(),
    makeMove: vi.fn(async (_fen: string, mv: { from: string; to: string }) => ({
      new_fen: `after-${mv.from}${mv.to}`,
      san: `${mv.from}${mv.to}`,
      outcome: null,
    })),
  },
}));

// Import AFTER vi.mock so the store sees the mock.
const { gameStore, STARTING_FEN } = await import("./game.svelte.ts");

describe("gameStore.makeMove", () => {
  beforeEach(() => {
    gameStore.currentFen = STARTING_FEN;
  });

  it("updates currentFen to the result returned by the backend", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    expect(gameStore.currentFen).toBe("after-e2e4");
  });
});
```

- [ ] **Step 13: Run all tests**

Run: `cargo test -p chess-core` (existing) then `cargo test -p chess-engine-app` (new) then `npm test`
Expected: chess-core ✓, chess-engine-app 5 ✓, vitest 3 ✓ (2 existing + 1 new).

- [ ] **Step 14: Manual verification**

Run: `npm run tauri dev`
Expected:
1. Click a white pawn (e.g., e2) → that square highlights yellow, two circles appear (e3 and e4).
2. Click e4 → pawn moves to e4. Highlights clear. Now it's Black's turn.
3. Click a black piece (e.g., e7 pawn) → black's legal moves highlight.
4. Try clicking an illegal target → selection clears.
5. Try clicking a white piece while it's Black's turn → nothing highlights.

- [ ] **Step 15: Commit**

```powershell
git add src-tauri/src/ src/lib/
git commit -m "feat: click-to-move with Tauri commands (legal_moves, make_move)"
```

---

### Task 5: Move history sidebar

**Goal:** Each move is recorded to `gameStore.history`. The right sidebar (`HistoryPanel`) shows a two-column SAN list. Layout updates to the 3-zone grid (top toolbar slot empty, board center, history right).

**Files:**
- Modify: `src/lib/stores/game.svelte.ts` (add history)
- Create: `src/lib/panels/HistoryPanel.svelte`
- Modify: `src/App.svelte` (3-zone grid)
- Create: `src/lib/stores/game.test.ts` additions

- [ ] **Step 1: Write the failing store test for history**

Replace `src/lib/stores/game.test.ts`:

```ts
import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("../tauri.ts", () => ({
  tauri: {
    legalMoves: vi.fn(),
    makeMove: vi.fn(async (_fen: string, mv: { from: string; to: string }) => ({
      new_fen: `after-${mv.from}${mv.to}`,
      san: `${mv.from}${mv.to}`,
      outcome: null,
    })),
  },
}));

const { gameStore, STARTING_FEN } = await import("./game.svelte.ts");

describe("gameStore", () => {
  beforeEach(() => {
    gameStore.currentFen = STARTING_FEN;
    gameStore.history = [];
    gameStore.cursor = 0;
  });

  it("updates currentFen and appends to history on makeMove", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    expect(gameStore.currentFen).toBe("after-e2e4");
    expect(gameStore.history).toHaveLength(1);
    expect(gameStore.history[0].san).toBe("e2e4");
    expect(gameStore.history[0].fen_after).toBe("after-e2e4");
    expect(gameStore.cursor).toBe(1);
  });

  it("accumulates multiple moves", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    gameStore.currentFen = "after-e2e4";
    await gameStore.makeMove({ from: "e7", to: "e5", promotion: null });
    expect(gameStore.history).toHaveLength(2);
    expect(gameStore.cursor).toBe(2);
  });
});
```

- [ ] **Step 2: Run test, verify it fails**

Run: `npm test`
Expected: FAIL — `history` / `cursor` not defined.

- [ ] **Step 3: Extend `gameStore`**

Replace `src/lib/stores/game.svelte.ts`:

```ts
import { tauri, type MoveDto, type MoveEntry, type OutcomeDto } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  currentFen = $state(STARTING_FEN);
  history: MoveEntry[] = $state([]);
  cursor = $state(0);

  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    // Drop any pending redo branch.
    this.history = this.history.slice(0, this.cursor);
    this.history.push({
      san: result.san,
      fen_after: result.new_fen,
      outcome: result.outcome,
    });
    this.cursor = this.history.length;
    this.currentFen = result.new_fen;
  }
}

export const gameStore = new GameStore();
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `npm test`
Expected: 3 tests pass (parseBoard 2 + new gameStore 2 = 4; actually 4 total). Re-count if off.

- [ ] **Step 5: Create `src/lib/panels/HistoryPanel.svelte`**

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";

  // Pair moves into [white, black] rows.
  const pairs = $derived.by(() => {
    const out: Array<{ num: number; white: string; black: string | null }> = [];
    for (let i = 0; i < gameStore.history.length; i += 2) {
      out.push({
        num: i / 2 + 1,
        white: gameStore.history[i].san,
        black: gameStore.history[i + 1]?.san ?? null,
      });
    }
    return out;
  });

  let listEl: HTMLElement | undefined = $state();

  $effect(() => {
    // Auto-scroll to bottom whenever history grows.
    void gameStore.history.length;
    if (listEl) listEl.scrollTop = listEl.scrollHeight;
  });
</script>

<aside class="history" bind:this={listEl}>
  <h2>Moves</h2>
  {#if pairs.length === 0}
    <p class="empty">No moves yet.</p>
  {:else}
    <ol>
      {#each pairs as p}
        <li>
          <span class="num">{p.num}.</span>
          <span class="san">{p.white}</span>
          {#if p.black}<span class="san">{p.black}</span>{/if}
        </li>
      {/each}
    </ol>
  {/if}
</aside>

<style>
  .history {
    background: #1a1a1a;
    color: #e8e8e8;
    padding: 12px;
    overflow-y: auto;
    font-family: ui-monospace, "Cascadia Code", monospace;
    border-left: 1px solid #444;
  }
  .history h2 {
    margin: 0 0 8px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #bbb;
  }
  .empty { color: #888; font-style: italic; }
  ol { list-style: none; padding: 0; margin: 0; }
  li {
    display: grid;
    grid-template-columns: 32px 1fr 1fr;
    gap: 6px;
    padding: 2px 0;
  }
  .num { color: #888; text-align: right; }
  .san { padding: 0 4px; }
</style>
```

- [ ] **Step 6: Update `src/App.svelte` to the 3-zone grid**

```svelte
<script lang="ts">
  import Board from "./lib/board/Board.svelte";
  import HistoryPanel from "./lib/panels/HistoryPanel.svelte";
</script>

<div class="app">
  <header class="toolbar">
    <!-- Toolbar arrives in Task 6 -->
  </header>
  <section class="board-area">
    <Board />
  </section>
  <HistoryPanel />
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    grid-template-columns: 1fr 280px;
    height: 100vh;
  }
  .toolbar {
    grid-column: 1 / span 2;
    background: #1f1f1f;
    border-bottom: 1px solid #444;
    height: 48px;
  }
  .board-area {
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
  }
</style>
```

- [ ] **Step 7: Manual verification**

Run: `npm run tauri dev`
Expected: Window now shows: top strip (empty, just dark bar) + centered board (left zone) + history sidebar (right). Playing moves now adds entries to the history list (auto-scrolls).

- [ ] **Step 8: Commit**

```powershell
git add src/
git commit -m "feat(ui): move history sidebar + 3-zone layout shell"
```

---

### Task 6: Undo / redo

**Goal:** `cursor` walks the history; `undo()`/`redo()` restore previous/next positions. Toolbar gets Undo/Redo buttons. `Ctrl+Z` and `Ctrl+Y` bound.

**Files:**
- Modify: `src/lib/stores/game.svelte.ts` (add undo/redo + STARTING_FEN-aware cursor handling)
- Modify: `src/lib/stores/game.test.ts` (add undo/redo tests)
- Create: `src/lib/panels/Toolbar.svelte`
- Modify: `src/App.svelte` (mount Toolbar)

- [ ] **Step 1: Write failing undo/redo tests**

Append to `src/lib/stores/game.test.ts`:

```ts
describe("gameStore undo/redo", () => {
  beforeEach(() => {
    gameStore.currentFen = STARTING_FEN;
    gameStore.history = [];
    gameStore.cursor = 0;
  });

  it("undo decrements cursor and restores prior FEN", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    expect(gameStore.cursor).toBe(1);
    gameStore.undo();
    expect(gameStore.cursor).toBe(0);
    expect(gameStore.currentFen).toBe(STARTING_FEN);
  });

  it("redo restores the next FEN after undo", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    gameStore.undo();
    gameStore.redo();
    expect(gameStore.cursor).toBe(1);
    expect(gameStore.currentFen).toBe("after-e2e4");
  });

  it("undo at cursor 0 is a no-op", () => {
    gameStore.undo();
    expect(gameStore.cursor).toBe(0);
    expect(gameStore.currentFen).toBe(STARTING_FEN);
  });

  it("redo at end-of-history is a no-op", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    gameStore.redo();
    expect(gameStore.cursor).toBe(1);
  });

  it("a new move after undo drops the redo branch", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    await gameStore.makeMove({ from: "e7", to: "e5", promotion: null });
    gameStore.undo();
    await gameStore.makeMove({ from: "d7", to: "d5", promotion: null });
    expect(gameStore.history).toHaveLength(2);
    expect(gameStore.history[1].san).toBe("d7d5");
  });
});
```

- [ ] **Step 2: Run, verify failure**

Run: `npm test`
Expected: FAIL — `undo` / `redo` not defined.

- [ ] **Step 3: Add undo/redo methods**

Replace `src/lib/stores/game.svelte.ts`:

```ts
import { tauri, type MoveDto, type MoveEntry } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  /** The FEN we started this game from. Undo can walk back to (but not past) this. */
  startingFen = $state(STARTING_FEN);
  currentFen = $state(STARTING_FEN);
  history: MoveEntry[] = $state([]);
  cursor = $state(0);

  private fenAt(c: number): string {
    return c === 0 ? this.startingFen : this.history[c - 1].fen_after;
  }

  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    this.history = this.history.slice(0, this.cursor);
    this.history.push({
      san: result.san,
      fen_after: result.new_fen,
      outcome: result.outcome,
    });
    this.cursor = this.history.length;
    this.currentFen = result.new_fen;
  }

  undo(): void {
    if (this.cursor === 0) return;
    this.cursor -= 1;
    this.currentFen = this.fenAt(this.cursor);
  }

  redo(): void {
    if (this.cursor >= this.history.length) return;
    this.cursor += 1;
    this.currentFen = this.fenAt(this.cursor);
  }

  reset(): void {
    this.startingFen = STARTING_FEN;
    this.currentFen = STARTING_FEN;
    this.history = [];
    this.cursor = 0;
  }
}

export const gameStore = new GameStore();
```

- [ ] **Step 4: Run tests, verify pass**

Run: `npm test`
Expected: all tests pass.

- [ ] **Step 5: Create `src/lib/panels/Toolbar.svelte`**

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";

  const canUndo = $derived(gameStore.cursor > 0);
  const canRedo = $derived(gameStore.cursor < gameStore.history.length);

  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "z") {
      e.preventDefault();
      gameStore.undo();
    } else if (e.ctrlKey && e.key === "y") {
      e.preventDefault();
      gameStore.redo();
    }
  }
</script>

<svelte:window on:keydown={onKey} />

<div class="toolbar-inner">
  <div class="placeholder">FEN input arrives in Task 8</div>
  <div class="spacer"></div>
  <button onclick={() => gameStore.undo()} disabled={!canUndo}>← Undo</button>
  <button onclick={() => gameStore.redo()} disabled={!canRedo}>Redo →</button>
  <div class="placeholder pgn-slot">PGN buttons arrive in Task 9</div>
</div>

<style>
  .toolbar-inner {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    color: #ddd;
  }
  .spacer { flex: 1; }
  .placeholder {
    font-style: italic;
    color: #777;
    font-size: 12px;
  }
  button {
    background: #333;
    border: 1px solid #555;
    color: #eee;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { background: #3d3d3d; }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
```

- [ ] **Step 6: Mount the Toolbar**

Update `src/App.svelte`:

```svelte
<script lang="ts">
  import Board from "./lib/board/Board.svelte";
  import HistoryPanel from "./lib/panels/HistoryPanel.svelte";
  import Toolbar from "./lib/panels/Toolbar.svelte";
</script>

<div class="app">
  <header class="toolbar">
    <Toolbar />
  </header>
  <section class="board-area">
    <Board />
  </section>
  <HistoryPanel />
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    grid-template-columns: 1fr 280px;
    height: 100vh;
  }
  .toolbar {
    grid-column: 1 / span 2;
    background: #1f1f1f;
    border-bottom: 1px solid #444;
    height: 48px;
  }
  .board-area {
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
  }
</style>
```

- [ ] **Step 7: Manual verification**

Run: `npm run tauri dev`
Expected:
1. Play 2-3 moves. Undo button enabled; click it — board reverts. History stays visible (the entry isn't removed, just the cursor moves).
2. Redo button now enabled; click it — board advances.
3. Press Ctrl+Z several times — board walks back to the starting position. Undo disables at cursor 0.
4. Press Ctrl+Y to walk forward; disables at end.
5. From mid-history, make a different move — the old "redo branch" disappears from history.

- [ ] **Step 8: Commit**

```powershell
git add src/
git commit -m "feat(ui): undo/redo via cursor + Toolbar with keyboard shortcuts"
```

---

### Task 7: Drag-and-drop

**Goal:** In addition to click-to-move, the user can grab a piece and drag it to a destination. Same `gameStore.makeMove` pipeline.

**Files:**
- Modify: `src/lib/stores/ui.svelte.ts` (add drag state)
- Modify: `src/lib/board/Board.svelte` (pointer event handlers)

- [ ] **Step 1: Extend `uiStore` with drag state**

Replace `src/lib/stores/ui.svelte.ts`:

```ts
import type { MoveDto } from "../tauri.ts";

interface DragState {
  from: string;
  /** Cursor position in board-local coordinates (px from board origin). */
  x: number;
  y: number;
}

class UiStore {
  selectedSquare: string | null = $state(null);
  legalTargets: string[] = $state([]);
  legalByFrom: Map<string, MoveDto[]> = $state(new Map());
  dragging: DragState | null = $state(null);

  selectSquare(sq: string, legalFromHere: MoveDto[]): void {
    this.selectedSquare = sq;
    this.legalTargets = legalFromHere.map((m) => m.to);
    this.legalByFrom = new Map([[sq, legalFromHere]]);
  }

  clearSelection(): void {
    this.selectedSquare = null;
    this.legalTargets = [];
    this.legalByFrom = new Map();
    this.dragging = null;
  }

  startDrag(from: string, x: number, y: number): void {
    this.dragging = { from, x, y };
  }

  updateDrag(x: number, y: number): void {
    if (this.dragging) this.dragging = { ...this.dragging, x, y };
  }

  endDrag(): void {
    this.dragging = null;
  }

  findMove(to: string): MoveDto | null {
    if (!this.selectedSquare) return null;
    const candidates = this.legalByFrom.get(this.selectedSquare) ?? [];
    return candidates.find((m) => m.to === to && (m.promotion === null || m.promotion === "Q")) ?? null;
  }
}

export const uiStore = new UiStore();
```

- [ ] **Step 2: Replace `src/lib/board/Board.svelte` with the pointer-driven version**

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { uiStore } from "../stores/ui.svelte.ts";
  import { tauri } from "../tauri.ts";
  import { parseBoard } from "./fen-board.ts";
  import Piece from "./Piece.svelte";

  const SQUARE_SIZE = 64;
  const BOARD_PX = SQUARE_SIZE * 8;
  const board = $derived(parseBoard(gameStore.currentFen));
  const sideToMove = $derived(gameStore.currentFen.split(" ")[1]);

  let svgEl: SVGSVGElement | undefined = $state();
  let pointerDownOn: { sq: string; piece: string; x: number; y: number } | null = null;
  let didDrag = false;

  function squareName(rankIdx: number, fileIdx: number): string {
    return "abcdefgh"[fileIdx] + (8 - rankIdx).toString();
  }
  function squareFromXY(x: number, y: number): string | null {
    if (x < 0 || y < 0 || x >= BOARD_PX || y >= BOARD_PX) return null;
    const fileIdx = Math.floor(x / SQUARE_SIZE);
    const rankIdx = Math.floor(y / SQUARE_SIZE);
    return squareName(rankIdx, fileIdx);
  }
  function isOwnPiece(piece: string): boolean {
    if (piece === ".") return false;
    const isWhite = piece === piece.toUpperCase();
    return (isWhite && sideToMove === "w") || (!isWhite && sideToMove === "b");
  }
  function localXY(e: PointerEvent): { x: number; y: number } {
    if (!svgEl) return { x: 0, y: 0 };
    const r = svgEl.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  async function onPointerDown(e: PointerEvent, rankIdx: number, fileIdx: number, piece: string) {
    const sq = squareName(rankIdx, fileIdx);
    const { x, y } = localXY(e);

    // If a target square clicked while a piece is selected, complete the move.
    if (uiStore.selectedSquare && uiStore.legalTargets.includes(sq)) {
      const mv = uiStore.findMove(sq);
      if (mv) {
        await gameStore.makeMove(mv);
        uiStore.clearSelection();
        return;
      }
    }

    // Otherwise, if the press is on own piece, prime for click-or-drag.
    if (isOwnPiece(piece)) {
      pointerDownOn = { sq, piece, x, y };
      didDrag = false;
      const all = await tauri.legalMoves(gameStore.currentFen);
      const fromHere = all.filter((m) => m.from === sq);
      if (fromHere.length > 0) {
        uiStore.selectSquare(sq, fromHere);
      }
      svgEl?.setPointerCapture(e.pointerId);
    } else {
      uiStore.clearSelection();
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!pointerDownOn) return;
    const { x, y } = localXY(e);
    const dx = x - pointerDownOn.x;
    const dy = y - pointerDownOn.y;
    if (!didDrag && Math.hypot(dx, dy) > 5) {
      didDrag = true;
      uiStore.startDrag(pointerDownOn.sq, x, y);
    } else if (didDrag) {
      uiStore.updateDrag(x, y);
    }
  }

  async function onPointerUp(e: PointerEvent) {
    if (!pointerDownOn) return;
    const { x, y } = localXY(e);
    const downSq = pointerDownOn.sq;
    pointerDownOn = null;
    if (didDrag) {
      const targetSq = squareFromXY(x, y);
      uiStore.endDrag();
      if (targetSq && targetSq !== downSq && uiStore.legalTargets.includes(targetSq)) {
        const mv = uiStore.findMove(targetSq);
        if (mv) {
          await gameStore.makeMove(mv);
        }
      }
      uiStore.clearSelection();
    }
    // If !didDrag, selection from onPointerDown stays — user will click target next.
  }
</script>

<svg
  bind:this={svgEl}
  width={BOARD_PX}
  height={BOARD_PX}
  viewBox="0 0 {BOARD_PX} {BOARD_PX}"
  class="board"
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
>
  {#each board as row, rankIdx}
    {#each row as piece, fileIdx}
      {@const isLight = (rankIdx + fileIdx) % 2 === 0}
      {@const sq = squareName(rankIdx, fileIdx)}
      {@const x = fileIdx * SQUARE_SIZE}
      {@const y = rankIdx * SQUARE_SIZE}
      {@const isSelected = uiStore.selectedSquare === sq}
      {@const isTarget = uiStore.legalTargets.includes(sq)}
      <rect
        {x} {y}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        fill={isLight ? "#f0d9b5" : "#b58863"}
        onpointerdown={(e) => onPointerDown(e, rankIdx, fileIdx, piece)}
        style:cursor="pointer"
      />
      {#if isSelected}
        <rect {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} fill="rgba(255,255,0,0.35)" pointer-events="none" />
      {/if}
      {#if isTarget}
        <circle
          cx={x + SQUARE_SIZE / 2}
          cy={y + SQUARE_SIZE / 2}
          r={SQUARE_SIZE * (piece === "." ? 0.15 : 0.45)}
          fill={piece === "." ? "rgba(0,0,0,0.35)" : "none"}
          stroke={piece === "." ? "none" : "rgba(0,0,0,0.55)"}
          stroke-width={piece === "." ? 0 : 4}
          pointer-events="none"
        />
      {/if}
      {#if piece !== "." && !(uiStore.dragging && uiStore.dragging.from === sq)}
        <svg {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} pointer-events="none">
          <Piece {piece} size={SQUARE_SIZE} />
        </svg>
      {/if}
    {/each}
  {/each}
  {#if uiStore.dragging}
    {@const dragPiece = board[8 - parseInt(uiStore.dragging.from[1])][("abcdefgh").indexOf(uiStore.dragging.from[0])]}
    {#if dragPiece !== "."}
      <svg
        x={uiStore.dragging.x - SQUARE_SIZE / 2}
        y={uiStore.dragging.y - SQUARE_SIZE / 2}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        pointer-events="none"
      >
        <Piece piece={dragPiece} size={SQUARE_SIZE} />
      </svg>
    {/if}
  {/if}
</svg>

<style>
  .board { display: block; user-select: none; touch-action: none; }
</style>
```

- [ ] **Step 3: Manual verification**

Run: `npm run tauri dev`
Expected:
1. Click a piece, then click destination — still works (click-to-move).
2. Press-and-drag a piece — the piece follows the cursor. Release on a legal target → move plays. Release elsewhere → snaps back.
3. Press-and-release without moving — same as a single click (selects the piece, highlights legals).
4. Press on own piece while it's the other side's turn → nothing happens.

- [ ] **Step 4: Commit**

```powershell
git add src/
git commit -m "feat(ui): drag-and-drop via pointer events, sharing click-to-move pipeline"
```

---

### Task 8: FEN load

**Goal:** A FEN input in the Toolbar. Typing turns the border red if invalid; clicking Load replaces the game state with the new position (clears history).

**Files:**
- Modify: `src-tauri/src/commands.rs` (add `validate_fen`)
- Modify: `src-tauri/src/main.rs` (register command)
- Modify: `src/lib/tauri.ts` (typed wrapper + loadFen method on store)
- Modify: `src/lib/stores/game.svelte.ts` (add `loadFen`)
- Modify: `src/lib/panels/Toolbar.svelte` (FEN input + Load button)

- [ ] **Step 1: Add `validate_fen` Tauri command**

Append to `src-tauri/src/commands.rs` (before the `#[cfg(test)]` block):

```rust
#[tauri::command]
pub fn validate_fen(fen: String) -> bool {
    cc::parse_fen(&fen).is_ok()
}
```

Append to the `tests` module:

```rust
    #[test]
    fn validate_fen_accepts_valid() {
        assert!(validate_fen(START.into()));
    }
    #[test]
    fn validate_fen_rejects_invalid() {
        assert!(!validate_fen("not a fen".into()));
        assert!(!validate_fen("8/8/8/8/8/8/8 w - - 0 1".into())); // only 7 ranks
    }
```

- [ ] **Step 2: Register the command**

In `src-tauri/src/main.rs`, extend the handler list:

```rust
.invoke_handler(tauri::generate_handler![
    commands::legal_moves,
    commands::make_move,
    commands::validate_fen,
])
```

- [ ] **Step 3: Run Rust tests**

Run: `cargo test -p chess-engine-app`
Expected: 7 tests pass.

- [ ] **Step 4: Add `validateFen` to the typed wrapper**

In `src/lib/tauri.ts`, replace the `export const tauri = { ... }` block with:

```ts
export const tauri = {
  legalMoves(fen: string): Promise<MoveDto[]> {
    return invoke("legal_moves", { fen });
  },
  makeMove(fen: string, mv: MoveDto): Promise<MakeMoveResult> {
    return invoke("make_move", { fen, mv });
  },
  validateFen(fen: string): Promise<boolean> {
    return invoke("validate_fen", { fen });
  },
};
```

- [ ] **Step 5: Add `loadFen` method to `gameStore`**

In `src/lib/stores/game.svelte.ts`, add a method inside the `GameStore` class:

```ts
  async loadFen(fen: string): Promise<void> {
    const ok = await tauri.validateFen(fen);
    if (!ok) throw new Error("Invalid FEN");
    this.startingFen = fen;
    this.currentFen = fen;
    this.history = [];
    this.cursor = 0;
  }
```

- [ ] **Step 6: Update Toolbar with FEN input**

Replace `src/lib/panels/Toolbar.svelte`:

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { tauri } from "../tauri.ts";

  let fenInput = $state(gameStore.currentFen);
  let fenValid = $state(true);
  let fenChecking = false;

  const canUndo = $derived(gameStore.cursor > 0);
  const canRedo = $derived(gameStore.cursor < gameStore.history.length);

  async function checkFen(s: string) {
    fenChecking = true;
    try {
      fenValid = await tauri.validateFen(s);
    } finally {
      fenChecking = false;
    }
  }

  async function loadFen() {
    if (!fenValid) return;
    try {
      await gameStore.loadFen(fenInput);
    } catch (e) {
      fenValid = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "z") { e.preventDefault(); gameStore.undo(); }
    else if (e.ctrlKey && e.key === "y") { e.preventDefault(); gameStore.redo(); }
  }
</script>

<svelte:window on:keydown={onKey} />

<div class="toolbar-inner">
  <input
    type="text"
    class="fen"
    class:invalid={!fenValid}
    bind:value={fenInput}
    oninput={() => checkFen(fenInput)}
    placeholder="Paste a FEN…"
    spellcheck="false"
  />
  <button onclick={loadFen} disabled={!fenValid || fenChecking}>Load</button>
  <div class="spacer"></div>
  <button onclick={() => gameStore.undo()} disabled={!canUndo}>← Undo</button>
  <button onclick={() => gameStore.redo()} disabled={!canRedo}>Redo →</button>
  <div class="placeholder pgn-slot">PGN buttons arrive in Task 9</div>
</div>

<style>
  .toolbar-inner {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    color: #ddd;
  }
  .spacer { flex: 1; }
  .placeholder { font-style: italic; color: #777; font-size: 12px; }
  .fen {
    flex: 1;
    max-width: 600px;
    background: #2a2a2a;
    color: #eee;
    border: 1px solid #555;
    padding: 6px 8px;
    border-radius: 4px;
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 12px;
  }
  .fen.invalid { border-color: #c0392b; }
  button {
    background: #333;
    border: 1px solid #555;
    color: #eee;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { background: #3d3d3d; }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
```

- [ ] **Step 7: Add `loadFen` Vitest test**

Append to `src/lib/stores/game.test.ts`:

```ts
describe("gameStore.loadFen", () => {
  beforeEach(() => {
    gameStore.reset();
  });

  it("replaces state with the new FEN and clears history when valid", async () => {
    (tauri.validateFen as any) = vi.fn(async () => true);
    const fen = "8/8/8/3k4/3K4/8/8/8 w - - 0 1";
    await gameStore.loadFen(fen);
    expect(gameStore.currentFen).toBe(fen);
    expect(gameStore.history).toEqual([]);
    expect(gameStore.cursor).toBe(0);
  });

  it("throws on invalid FEN", async () => {
    (tauri.validateFen as any) = vi.fn(async () => false);
    await expect(gameStore.loadFen("garbage")).rejects.toThrow();
  });
});
```

Also update the top-of-file mock to include `validateFen`:

```ts
vi.mock("../tauri.ts", () => ({
  tauri: {
    legalMoves: vi.fn(),
    makeMove: vi.fn(async (_fen: string, mv: { from: string; to: string }) => ({
      new_fen: `after-${mv.from}${mv.to}`,
      san: `${mv.from}${mv.to}`,
      outcome: null,
    })),
    validateFen: vi.fn(async () => true),
  },
}));
const { gameStore, STARTING_FEN } = await import("./game.svelte.ts");
const { tauri } = await import("../tauri.ts");
```

- [ ] **Step 8: Run tests**

Run: `npm test`
Expected: all tests pass (including 2 new loadFen tests).

- [ ] **Step 9: Manual verification**

Run: `npm run tauri dev`
Expected:
1. Paste Kiwipete `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1` → border stays normal. Click Load → board updates.
2. Type garbage → border turns red, Load disabled.
3. Reload Kiwipete; undo button is disabled (history cleared).

- [ ] **Step 10: Commit**

```powershell
git add src-tauri/src/ src/
git commit -m "feat: FEN validation command + Toolbar FEN load with red-border invalid state"
```

---

### Task 9: PGN save / load

**Goal:** Save current game to a `.pgn` file via native dialog. Load a `.pgn` file and replay it into `gameStore`.

**Files:**
- Modify: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json` (add dialog plugin)
- Modify: `src-tauri/src/commands.rs` (add `parse_pgn`, `serialize_pgn`, `save_pgn_file`, `load_pgn_file`)
- Modify: `src-tauri/src/main.rs`
- Modify: `package.json` (add `@tauri-apps/plugin-dialog`)
- Modify: `src/lib/tauri.ts`
- Modify: `src/lib/stores/game.svelte.ts` (add `loadPgn`, `savePgn`)
- Modify: `src/lib/panels/Toolbar.svelte` (Save/Load PGN buttons)

- [ ] **Step 1: Add `tauri-plugin-dialog` to `src-tauri/Cargo.toml`**

Add under `[dependencies]`:

```toml
tauri-plugin-dialog = "2"
```

- [ ] **Step 2: Add dialog permission**

Edit `src-tauri/capabilities/default.json`:

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default"
  ]
}
```

- [ ] **Step 3: Register the plugin in `src-tauri/src/main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::legal_moves,
            commands::make_move,
            commands::validate_fen,
            commands::parse_pgn,
            commands::serialize_pgn,
            commands::save_pgn_file,
            commands::load_pgn_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Add PGN + file commands to `src-tauri/src/commands.rs`**

Append (above the `#[cfg(test)]` block):

```rust
use std::fs;

/// Parse PGN text into a list of moves keyed for the frontend.
#[tauri::command]
pub fn parse_pgn(text: String) -> Result<GameDto, String> {
    let game = cc::parse_pgn(&text).map_err(|e| format!("PGN parse error: {e:?}"))?;
    let mut pos = cc::Position::starting();
    let mut entries = Vec::with_capacity(game.moves.len());
    for m in &game.moves {
        let san = cc::move_to_san(&pos, *m);
        cc::make_move(&mut pos, *m);
        let fen_after = cc::serialize_fen(&pos);
        let outcome = cc::detect_outcome(&pos).map(|o| outcome_to_dto(o, pos.side_to_move));
        entries.push(MoveEntry { san, fen_after, outcome });
    }
    Ok(GameDto {
        tags: game.tags,
        moves: entries,
        result: game.result,
        final_fen: cc::serialize_fen(&pos),
    })
}

/// Serialize a list of MoveDtos (plus tags) into PGN text.
#[tauri::command]
pub fn serialize_pgn(moves: Vec<MoveDto>, tags: HashMap<String, String>) -> Result<String, String> {
    let mut pos = cc::Position::starting();
    let mut core_moves: Vec<cc::Move> = Vec::with_capacity(moves.len());
    for mv in moves {
        let from_sq = parse_square(&mv.from).ok_or_else(|| format!("Invalid 'from': {}", mv.from))?;
        let to_sq = parse_square(&mv.to).ok_or_else(|| format!("Invalid 'to': {}", mv.to))?;
        let want_promo: Option<cc::PieceKind> = mv.promotion.and_then(|c| match c {
            'Q' => Some(cc::PieceKind::Queen),
            'R' => Some(cc::PieceKind::Rook),
            'B' => Some(cc::PieceKind::Bishop),
            'N' => Some(cc::PieceKind::Knight),
            _ => None,
        });
        let core_mv = cc::legal_moves(&pos).into_iter().find(|m| {
            if m.from() != from_sq || m.to() != to_sq { return false; }
            if m.flag().is_promotion() {
                promo_kind_of_flag(m.flag()) == Some(want_promo.unwrap_or(cc::PieceKind::Queen))
            } else {
                want_promo.is_none()
            }
        }).ok_or_else(|| format!("Illegal move during serialize: {} -> {}", mv.from, mv.to))?;
        cc::make_move(&mut pos, core_mv);
        core_moves.push(core_mv);
    }
    let result = tags.get("Result").cloned().unwrap_or_else(|| "*".to_string());
    let game = cc::Game {
        tags,
        moves: core_moves,
        result,
        final_position: pos,
    };
    Ok(cc::serialize_pgn(&game))
}

/// Write PGN text to a path chosen by the user via the dialog plugin (path supplied by the frontend).
#[tauri::command]
pub fn save_pgn_file(path: String, text: String) -> Result<(), String> {
    fs::write(&path, text).map_err(|e| format!("Could not save: {e}"))
}

#[tauri::command]
pub fn load_pgn_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Could not read: {e}"))
}
```

Append to the `tests` module:

```rust
    #[test]
    fn parse_pgn_round_trips_scholars_mate() {
        let pgn = "[Event \"Test\"]\n\n1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6 4. Qxf7# 1-0\n";
        let game = parse_pgn(pgn.into()).unwrap();
        assert_eq!(game.moves.len(), 7);
        assert_eq!(game.result, "1-0");
        let last = &game.moves[6];
        assert_eq!(last.san, "Qxf7#");
        assert_eq!(last.outcome.as_ref().unwrap().kind, "Checkmate");
    }

    #[test]
    fn serialize_pgn_then_parse_pgn_round_trip() {
        let mut tags = HashMap::new();
        tags.insert("White".into(), "A".into());
        tags.insert("Result".into(), "*".into());
        let moves = vec![
            MoveDto { from: "e2".into(), to: "e4".into(), promotion: None },
            MoveDto { from: "e7".into(), to: "e5".into(), promotion: None },
        ];
        let text = serialize_pgn(moves, tags).unwrap();
        let game = parse_pgn(text).unwrap();
        assert_eq!(game.moves.len(), 2);
    }
```

- [ ] **Step 5: Add `@tauri-apps/plugin-dialog` to `package.json`**

Under `dependencies`:

```json
"@tauri-apps/plugin-dialog": "^2"
```

Run: `npm install`

- [ ] **Step 6: Extend `src/lib/tauri.ts`**

Add to the `tauri` object:

```ts
  parsePgn(text: string): Promise<GameDto> {
    return invoke("parse_pgn", { text });
  },
  serializePgn(moves: MoveDto[], tags: Record<string, string>): Promise<string> {
    return invoke("serialize_pgn", { moves, tags });
  },
  savePgnFile(path: string, text: string): Promise<void> {
    return invoke("save_pgn_file", { path, text });
  },
  loadPgnFile(path: string): Promise<string> {
    return invoke("load_pgn_file", { path });
  },
```

- [ ] **Step 7: Extend `MoveEntry` to carry the `MoveDto`**

PGN serialization needs the original `MoveDto` for each historical move (so we can replay the position forward through `serialize_pgn`). Extend `MoveEntry` everywhere.

In `src-tauri/src/commands.rs`, update `MoveEntry`:

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveEntry {
    pub san: String,
    pub fen_after: String,
    pub outcome: Option<OutcomeDto>,
    pub mv: MoveDto,
}
```

In `parse_pgn`, update the `entries.push` to populate `mv`:

```rust
        entries.push(MoveEntry { san, fen_after, outcome, mv: move_to_dto(*m) });
```

In `src/lib/tauri.ts`, update the `MoveEntry` interface:

```ts
export interface MoveEntry {
  san: string;
  fen_after: string;
  outcome: OutcomeDto | null;
  mv: MoveDto;
}
```

- [ ] **Step 8: Update `gameStore.makeMove` to record the `MoveDto` on each entry**

In `src/lib/stores/game.svelte.ts`, replace `makeMove`:

```ts
  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    this.history = this.history.slice(0, this.cursor);
    this.history.push({
      san: result.san,
      fen_after: result.new_fen,
      outcome: result.outcome,
      mv,
    });
    this.cursor = this.history.length;
    this.currentFen = result.new_fen;
  }
```

- [ ] **Step 9: Add a `tags` field and `loadPgn` / `savePgn` to `gameStore`**

In `src/lib/stores/game.svelte.ts`, add a field inside `GameStore`:

```ts
  tags: Record<string, string> = $state({});
```

Update `reset()` to clear tags too:

```ts
  reset(): void {
    this.startingFen = STARTING_FEN;
    this.currentFen = STARTING_FEN;
    this.history = [];
    this.cursor = 0;
    this.tags = {};
  }
```

Add the two methods:

```ts
  async savePgn(path: string): Promise<void> {
    const moves = this.history.slice(0, this.cursor).map((e) => e.mv);
    const text = await tauri.serializePgn(moves, this.tags);
    await tauri.savePgnFile(path, text);
  }

  async loadPgn(text: string): Promise<void> {
    const game = await tauri.parsePgn(text);
    this.startingFen = STARTING_FEN;
    this.currentFen = game.final_fen;
    this.tags = game.tags;
    this.history = game.moves.map((m) => ({
      san: m.san,
      fen_after: m.fen_after,
      outcome: m.outcome,
      mv: m.mv,
    }));
    this.cursor = this.history.length;
  }
```

- [ ] **Step 10: Wire PGN buttons into Toolbar**

Replace the `pgn-slot` placeholder in `src/lib/panels/Toolbar.svelte` with:

```svelte
<script lang="ts">
  // … existing imports …
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { tauri } from "../tauri.ts";
</script>

…

  async function savePgn() {
    const path = await save({
      defaultPath: "game.pgn",
      filters: [{ name: "PGN", extensions: ["pgn"] }],
    });
    if (!path) return;
    try {
      await gameStore.savePgn(path);
    } catch (e) {
      alert(`Could not save: ${e}`);
    }
  }

  async function loadPgn() {
    const path = await open({
      filters: [{ name: "PGN", extensions: ["pgn"] }],
      multiple: false,
    });
    if (!path || Array.isArray(path)) return;
    try {
      const text = await tauri.loadPgnFile(path);
      await gameStore.loadPgn(text);
    } catch (e) {
      alert(`Could not load: ${e}`);
    }
  }
```

Replace the placeholder block in the template:

```svelte
<button onclick={savePgn}>Save PGN</button>
<button onclick={loadPgn}>Load PGN</button>
```

- [ ] **Step 11: Run all tests**

Run: `cargo test -p chess-engine-app` (now 9 tests) and `npm test`.
Expected: all green.

- [ ] **Step 12: Manual verification**

Run: `npm run tauri dev`
Expected:
1. Play 4 moves. Click Save PGN → native save dialog opens. Save as `test.pgn`. Open the file in a text editor — confirm SAN moves are present.
2. Make new moves to diverge. Click Load PGN → pick `test.pgn` → board returns to the 4-move position. History panel shows the 4 SAN entries. Undo walks back through them correctly.
3. Cancel either dialog — no error, no state change.

- [ ] **Step 13: Commit**

```powershell
git add src-tauri/ src/ package.json package-lock.json
git commit -m "feat: PGN save/load via native dialog + parse_pgn/serialize_pgn commands"
```

---

### Task 10: Game-over banner

**Goal:** When `gameStore.outcome` is non-null, show an overlay banner. "New game" button resets to starting position.

**Files:**
- Modify: `src/lib/stores/game.svelte.ts` (`outcome` derived rune is already populated via `MoveEntry.outcome`; expose a `outcome` getter)
- Create: `src/lib/panels/GameOverBanner.svelte`
- Modify: `src/App.svelte` (mount banner)

- [ ] **Step 1: Add `outcome` derived to `gameStore`**

In `src/lib/stores/game.svelte.ts`, after the `cursor` declaration, add:

```ts
  outcome = $derived(
    this.cursor > 0 ? this.history[this.cursor - 1].outcome : null
  );
```

- [ ] **Step 2: Add Vitest test for outcome derivation**

Append to `src/lib/stores/game.test.ts`:

```ts
describe("gameStore.outcome", () => {
  beforeEach(() => { gameStore.reset(); });

  it("is null at the start", () => {
    expect(gameStore.outcome).toBeNull();
  });

  it("is the outcome of the last move when present", async () => {
    (tauri.makeMove as any) = vi.fn(async () => ({
      new_fen: "mate-fen",
      san: "Qh4#",
      outcome: { kind: "Checkmate", winner: "Black" },
    }));
    await gameStore.makeMove({ from: "d8", to: "h4", promotion: null });
    expect(gameStore.outcome).toEqual({ kind: "Checkmate", winner: "Black" });
  });

  it("returns to null after undo from a terminal position", async () => {
    (tauri.makeMove as any) = vi.fn(async () => ({
      new_fen: "mate-fen",
      san: "Qh4#",
      outcome: { kind: "Checkmate", winner: "Black" },
    }));
    await gameStore.makeMove({ from: "d8", to: "h4", promotion: null });
    gameStore.undo();
    expect(gameStore.outcome).toBeNull();
  });
});
```

- [ ] **Step 3: Run tests**

Run: `npm test`
Expected: all green (3 new tests pass).

- [ ] **Step 4: Create `GameOverBanner.svelte`**

`src/lib/panels/GameOverBanner.svelte`:

```svelte
<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import type { OutcomeDto } from "../tauri.ts";

  function label(o: OutcomeDto): string {
    switch (o.kind) {
      case "Checkmate": return `Checkmate — ${o.winner} wins`;
      case "Stalemate": return "Stalemate — draw";
      case "FiftyMove": return "Draw by 50-move rule";
      case "InsufficientMaterial": return "Draw by insufficient material";
    }
  }
</script>

{#if gameStore.outcome}
  <div class="overlay" role="dialog" aria-modal="true">
    <div class="card">
      <h2>{label(gameStore.outcome)}</h2>
      <button onclick={() => gameStore.reset()}>New game</button>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }
  .card {
    background: #1f1f1f;
    border: 1px solid #555;
    border-radius: 8px;
    padding: 24px 36px;
    text-align: center;
    color: #eee;
    min-width: 320px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }
  .card h2 {
    margin: 0 0 18px;
    font-size: 22px;
  }
  button {
    background: #2e7d32;
    border: none;
    color: white;
    padding: 8px 20px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }
  button:hover { background: #388e3c; }
</style>
```

- [ ] **Step 5: Mount in `App.svelte`**

Update `src/App.svelte`:

```svelte
<script lang="ts">
  import Board from "./lib/board/Board.svelte";
  import HistoryPanel from "./lib/panels/HistoryPanel.svelte";
  import Toolbar from "./lib/panels/Toolbar.svelte";
  import GameOverBanner from "./lib/panels/GameOverBanner.svelte";
</script>

<div class="app">
  <header class="toolbar">
    <Toolbar />
  </header>
  <section class="board-area">
    <Board />
  </section>
  <HistoryPanel />
</div>
<GameOverBanner />

<!-- styles unchanged from Task 6 -->
```

- [ ] **Step 6: Manual verification**

Run: `npm run tauri dev`
Expected:
1. Play Fool's Mate (1. f3 e5 2. g4 Qh4#). Banner appears: "Checkmate — Black wins". Click "New game" → resets to starting position. Banner dismissed.
2. Load a stalemate FEN (e.g., `7k/5Q2/6K1/8/8/8/8/8 b - - 0 1`). Banner should NOT appear until a move is made (the outcome is only checked AFTER a move per current logic). To test stalemate detection: play 1. Kg1 from `4k3/8/3K4/8/8/8/8/3Q4 w - - 0 1` (a real stalemate setup — verify via chess-core perft first).
3. Undo from a banner state — banner dismisses.

- [ ] **Step 7: Commit**

```powershell
git add src/
git commit -m "feat(ui): game-over banner with New game reset"
```

---

### Task 11: Polish & smoke test

**Goal:** Remaining keyboard shortcuts (`Ctrl+S` save PGN, `Ctrl+O` load PGN), README update, and the full manual smoke checklist from the spec.

**Files:**
- Modify: `src/lib/panels/Toolbar.svelte` (Ctrl+S, Ctrl+O)
- Modify: `README.md` (project root)

- [ ] **Step 1: Add Ctrl+S / Ctrl+O shortcuts to Toolbar**

In `src/lib/panels/Toolbar.svelte`, extend the `onKey` handler:

```ts
  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "z") { e.preventDefault(); gameStore.undo(); }
    else if (e.ctrlKey && e.key === "y") { e.preventDefault(); gameStore.redo(); }
    else if (e.ctrlKey && e.key === "s") { e.preventDefault(); savePgn(); }
    else if (e.ctrlKey && e.key === "o") { e.preventDefault(); loadPgn(); }
  }
```

- [ ] **Step 2: Update root `README.md`**

Replace (or create) `README.md`:

```markdown
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
```

- [ ] **Step 3: Run the full smoke checklist** (from the spec, Testing section)

Run: `npm run tauri dev`, then perform each check. Mark each as it passes:

- [ ] Open app — Tauri window appears with board, toolbar, history panel.
- [ ] Play 1. e4 e5 2. Nf3 Nc6 — board updates, history shows the 4 SANs.
- [ ] Press `Ctrl+Z` four times — board returns to starting position.
- [ ] Press `Ctrl+Y` four times — board returns to the 4-move position.
- [ ] Paste Kiwipete FEN `r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1` → click Load → board updates.
- [ ] Make 2 moves, press `Ctrl+S`, save as `test.pgn`. Open file — confirm SAN.
- [ ] Press `Ctrl+O`, choose `test.pgn` → board returns to the 2-move position.
- [ ] Reset (Load original starting FEN), play Fool's Mate (1. f3 e5 2. g4 Qh4#) → "Checkmate — Black wins" banner appears.
- [ ] Click "New game" → banner dismisses, starting position restored.
- [ ] Play a 10-move opening (any) — no console errors throughout (`npm run tauri dev` console).

- [ ] **Step 4: Run all automated tests one more time**

```powershell
cargo test -p chess-core
cargo test -p chess-engine-app
npm test
```
Expected: all green.

- [ ] **Step 5: Build the release binary**

Run: `npm run tauri build`
Expected: produces `src-tauri/target/release/chess-engine-app.exe` (and an installer in `src-tauri/target/release/bundle/`). Open the `.exe` directly to confirm the production build also works.

- [ ] **Step 6: Commit**

```powershell
git add src/ README.md
git commit -m "polish: Ctrl+S/Ctrl+O shortcuts + README; smoke checklist green"
```

---

## Done state

When this plan completes:

- `npm run tauri dev` opens a working hot-seat chess app on Windows
- `npm run tauri build` produces a release `.exe`
- `cargo test -p chess-core` ✓ (existing 44 + perft)
- `cargo test -p chess-engine-app` ✓ (~9 Tauri command tests)
- `npm test` ✓ (~10 Vitest store + helper tests)
- Smoke checklist (Step 3 of Task 11) all green
- 11 commits on `plan-2-playable-board` branch beyond the spec commit (`66483ce`)

## Branch state and next steps

This plan is implemented on the existing `plan-2-playable-board` branch (already stacked on top of `plan-1-chess-core`). When Plan 1's PR #1 merges to `main`, rebase `plan-2-playable-board` onto the updated `main` before opening Plan 2's PR.

Plan 3 starts: Engine API trait + Stockfish UCI adapter, wired through a new `Engines` toolbar dropdown ("Human" / "Stockfish") on each side.
