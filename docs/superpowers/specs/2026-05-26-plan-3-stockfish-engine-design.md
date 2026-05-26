# Plan 3 — Stockfish Engine Integration

**Date:** 2026-05-26
**Status:** Approved (design phase)
**Builds on:** Plan 2 (playable hot-seat board, merged to main as PR #2)
**Defers:** DIY chess engine (Plan 4+), engine-vs-engine mode, hints-off / no-spoil toggle, in-app About panel

## Goal

Add Stockfish to the existing app. Engine picker per side (Human / Stockfish + Skill Level 0-20). When it's the engine's turn, Stockfish moves automatically within ~1 second. Always-on analysis panel: vertical eval bar to the left of the board + top-3 candidate moves below the history sidebar.

Plan 3's purpose is to land the engine abstraction (one trait, one implementation) plus the streaming-analysis UI, on the existing Plan 2 hot-seat foundation, without introducing the complexity of writing a from-scratch engine. The DIY engine (chess-engine-diy) is the entire scope of Plans 4+ and intentionally not touched here.

## Stack

- **Backend:** Rust + Tauri 2.x. New crates `chess-engine-api` (pure types) and `chess-engine-uci` (Stockfish adapter). `tokio` for the subprocess and reader task.
- **Frontend:** Svelte 5 (runes) + TypeScript + Vite 5 (unchanged from Plan 2). Two new stores, three new components.
- **External engine:** Stockfish 17 official Windows release, bundled as a Tauri `externalBin` resource.
- **Testing:** Vitest for frontend stores; `#[cfg(test)]` for Rust crates; a single `#[ignore]`-flagged integration test that spawns real Stockfish.

## Workspace layout

Extends what Plan 2 produced. New files marked **NEW**.

```
chess-engine/
├── src-tauri/
│   ├── Cargo.toml                                     MODIFY: deps tokio, chess-engine-api, chess-engine-uci
│   ├── tauri.conf.json                                MODIFY: bundle.externalBin entry for stockfish
│   ├── binaries/
│   │   └── stockfish-x86_64-pc-windows-msvc.exe       NEW: bundled, ~30 MB
│   ├── crates/
│   │   ├── chess-core/                                (unchanged)
│   │   ├── chess-engine-api/                          NEW
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs                             — Engine trait, Score, SearchLimits, AnalysisInfo, EngineKind
│   │   └── chess-engine-uci/                          NEW
│   │       ├── Cargo.toml
│   │       └── src/
│   │           ├── lib.rs                             — StockfishEngine struct, Engine impl
│   │           └── parser.rs                          — pure UCI line parser (unit-tested)
│   └── src/
│       ├── lib.rs                                     MODIFY: app.manage(EngineManager), register 2 commands
│       └── commands.rs                                MODIFY: add start_analysis, stop_analysis
└── src/
    └── lib/
        ├── tauri.ts                                   MODIFY: engine command wrappers + event types
        ├── stores/
        │   ├── engines.svelte.ts                      NEW
        │   ├── engines.test.ts                        NEW
        │   ├── analysis.svelte.ts                     NEW
        │   ├── analysis.test.ts                       NEW
        │   └── game.svelte.ts                         (unchanged in this plan)
        ├── panels/
        │   ├── Toolbar.svelte                         MODIFY: mount <EnginePickers /> right-side
        │   ├── EnginePickers.svelte                   NEW: two chips with Human/Stockfish + Skill slider popover
        │   ├── EvalBar.svelte                         NEW: vertical bar, Lichess sigmoid mapping
        │   └── AnalysisPanel.svelte                   NEW: top-N moves table below HistoryPanel
        └── App.svelte                                 MODIFY: add EvalBar to left, AnalysisPanel slot,
                                                                $effect that triggers start_analysis on FEN change
```

## chess-engine-api crate (pure types)

```rust
//! Pure types for engine abstraction. No async runtime, no I/O.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Score {
    /// Centipawns from the side-to-move's perspective.
    Cp(i32),
    /// Forced mate in N plies, sign indicates which side (positive = side to move wins).
    Mate(i8),
}

#[derive(Clone, Debug)]
pub struct SearchLimits {
    pub movetime_ms: u32,
    pub multipv: u8,
}

#[derive(Clone, Debug)]
pub struct AnalysisInfo {
    pub depth: u8,
    pub score: Score,
    pub pv: Vec<chess_core::Move>,
    pub multipv_index: u8,          // 1-based per UCI convention
    pub nodes: u64,
    pub nps: u64,
    pub time_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EngineKind { Human, Stockfish }

/// One trait, two callbacks: `on_info` is called per UCI `info` line,
/// `on_bestmove` exactly once when the search ends (either time expired or `stop()` invoked).
/// Callback-based instead of async so the trait stays runtime-agnostic.
pub trait Engine {
    fn name(&self) -> &str;
    fn analyze(
        &mut self,
        position: chess_core::Position,
        limits: SearchLimits,
        skill_level: u8,            // Stockfish-specific; ignored by other engines
        on_info: Box<dyn FnMut(AnalysisInfo) + Send>,
        on_bestmove: Box<dyn FnOnce(chess_core::Move) + Send>,
    );
    fn stop(&mut self);
}
```

`#[cfg(test)]` also exposes a `MockEngine` that emits one canned `AnalysisInfo` + one canned bestmove on `analyze()`. Used by `chess-engine-uci` tests and by `chess-engine-app` Tauri command tests so neither suite has to spawn real Stockfish.

## chess-engine-uci crate (Stockfish adapter)

**`parser` module — pure UCI line parser:**

```rust
pub enum ParsedLine {
    Info(AnalysisInfo),
    BestMove(chess_core::Move),
    Other,                    // id, option, uciok, readyok, etc. — caller decides if relevant
    Malformed(String),
}

pub fn parse_uci_line(line: &str, side_to_move: chess_core::Color) -> ParsedLine;
```

Unit tests cover ~15 synthetic Stockfish output strings: starting-position info at depths 1-20, multipv 1/2/3, mate scores (`score mate 3`, `score mate -5`), promotions (`bestmove e7e8q`), the readyok/uciok handshake, and malformed lines (`info depth abc`, truncated PV).

**`StockfishEngine` struct — implements `Engine`:**

- Owns `Option<tokio::process::Child>` (lazy-spawn on first `analyze()`).
- Owns a `tokio::sync::Mutex<ChildState>` where `ChildState` is `{ stdin, stdout_lines: BoxStream }` (acquired briefly for each operation).
- On lazy spawn: `Command::new(stockfish_path).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()` → write `uci\n` → read lines until `uciok` → write `isready\n` → read until `readyok`. Cache the now-ready child.
- `analyze(pos, limits, skill_level, on_info, on_bestmove)`:
  1. Write `stop\n` (no-op if no prior search active).
  2. Write `setoption name Skill Level value {skill_level}\n`.
  3. Write `setoption name MultiPV value {limits.multipv}\n`.
  4. Write `position fen {pos.to_fen()}\n`.
  5. Write `go movetime {limits.movetime_ms}\n`.
  6. Spawn a tokio task that reads stdout lines until `bestmove` arrives or stop is requested. For each line, parse via `parser::parse_uci_line`; route `Info` to `on_info(info)`, `BestMove` to `on_bestmove(m)` then exit task.
- `stop()`: writes `stop\n` to stdin. The reader task's `bestmove` will arrive promptly; `on_bestmove` still fires.

**Integration test** (`#[ignore]`-flagged): spawn the bundled Stockfish from `src-tauri/binaries/...exe`, call `analyze` on the starting position with `movetime_ms=100`, assert ≥1 info call and one bestmove call within 2 seconds. Marked `#[ignore]` so a CI environment without the binary doesn't fail; run locally with `cargo test --include-ignored -p chess-engine-uci`.

## Tauri layer

**`EngineManager` state** (managed via `app.manage(...)` in `lib.rs`):

```rust
pub struct EngineManager {
    engine: Mutex<Option<Box<dyn Engine + Send>>>,    // Lazy: None until first start_analysis
    next_search_id: AtomicU64,
    current_search_id: AtomicU64,                      // For stop_analysis filtering
    app: AppHandle,                                    // For emitting events
}
```

**2 Tauri commands added to `src-tauri/src/commands.rs`:**

```rust
#[tauri::command]
pub async fn start_analysis(
    state: State<'_, EngineManager>,
    fen: String,
    skill_level: u8,
    movetime_ms: u32,
    multipv: u8,
) -> Result<u64, String>;

#[tauri::command]
pub async fn stop_analysis(
    state: State<'_, EngineManager>,
    search_id: u64,
) -> Result<(), String>;
```

Implementation:

- `start_analysis` lazy-initializes `state.engine` to a `StockfishEngine` (resolves the sidecar path via Tauri's `path().resolve("binaries/stockfish-x86_64-pc-windows-msvc", BaseDirectory::Resource)`). Bumps `next_search_id`, sets `current_search_id`. Calls `engine.analyze(...)` with closures that wrap the events:
  - `on_info`: emit `app.emit("analysis_info", AnalysisInfoEvent { search_id, ...info })`.
  - `on_bestmove`: emit `app.emit("engine_bestmove", EngineBestMoveEvent { search_id, mv: move_to_dto(m) })`.
- `stop_analysis(search_id)`: if `search_id == current_search_id`, calls `engine.stop()`. Otherwise no-op (stale).

## Tauri events (server-push to frontend)

```ts
// emitted per UCI info line
interface AnalysisInfoEvent {
  search_id: number;
  depth: number;
  score: { kind: "Cp"; value: number } | { kind: "Mate"; value: number };
  pv: MoveDto[];
  multipv_index: number;
  nodes: number;
  nps: number;
  time_ms: number;
}

// emitted exactly once per search when bestmove arrives
interface EngineBestMoveEvent {
  search_id: number;
  mv: MoveDto;
}
```

Frontend listeners are registered at app mount in `App.svelte`'s top-level `onMount`. Both handlers filter by `analysisStore.searchId` before applying — stale events from previous searches are silently dropped.

## Frontend structure

### Stores

**`enginesStore` (`src/lib/stores/engines.svelte.ts`):**

```ts
type EngineSlot = { kind: "Human" | "Stockfish"; skill: number };

class EnginesStore {
  white: EngineSlot = $state({ kind: "Human", skill: 10 });
  black: EngineSlot = $state({ kind: "Stockfish", skill: 10 });

  engineFor(side: "w" | "b"): EngineSlot | null;       // null if Human
  anyEngine: boolean = $derived(this.white.kind === "Stockfish" || this.black.kind === "Stockfish");
  /** Engine for the side that's to move in the current FEN; used by the auto-play handler. */
  engineForSideToMove(fen: string): EngineSlot | null;
}
```

**`analysisStore` (`src/lib/stores/analysis.svelte.ts`):**

```ts
class AnalysisStore {
  searchId: number | null = $state(null);
  lines: Map<number, AnalysisInfoEvent> = $state(new Map());   // keyed by multipv_index

  /** EvalBar reads this (0.0 to 1.0). Sigmoid: 2/(1+exp(-cp/400)) - 1, mapped to [0,1]. */
  evalPercent: number = $derived(/* compute from lines.get(1)?.score */);

  applyInfo(info: AnalysisInfoEvent): void {              // ignores stale events
    if (info.search_id !== this.searchId) return;
    this.lines.set(info.multipv_index, info);
  }
  reset(newSearchId: number): void {
    this.searchId = newSearchId;
    this.lines = new Map();
  }
}
```

### Components

- **`EnginePickers.svelte`** — two clickable chips in the toolbar right side. Each chip shows current engine name + skill (e.g., `⚪ Human`, `⚫ Stockfish (10)`). Click opens a popover with: radio for Human/Stockfish, range slider 0-20 for skill (disabled when Human). Writes to `enginesStore`.
- **`EvalBar.svelte`** — fixed-width (24 px) vertical bar in a new grid column to the LEFT of the board. Reads `analysisStore.evalPercent`. White-on-top fill that animates with a CSS transition. Shows `+0.42` / `M3` text at the appropriate end.
- **`AnalysisPanel.svelte`** — table below `HistoryPanel` in the right sidebar (sidebar becomes vertical stack: History on top, Analysis below). One row per multipv_index sorted by index. Columns: SAN (from PV[0] via `chess_core::move_to_san`), score (`+0.42` or `M3`), depth, PV preview (first 4-5 plies SAN-rendered).

### App-level wiring

`src/App.svelte` adds:

- A grid column for `<EvalBar />` left of the existing board column. New grid template: `auto 1fr 280px` (eval bar, board, sidebar).
- An `<AnalysisPanel />` in the sidebar grid cell below `<HistoryPanel />`.
- `onMount`: register `listen("analysis_info", ...)` and `listen("engine_bestmove", ...)` handlers. Both filter by `analysisStore.searchId`.
- A top-level `$effect` that analyzes every position when any engine is active:
  ```ts
  $effect(() => {
    const fen = gameStore.currentFen;
    if (!enginesStore.anyEngine) return;
    // Use whichever side has Stockfish for the skill cap; if both are human we already returned.
    const skill = (enginesStore.white.kind === "Stockfish"
      ? enginesStore.white.skill
      : enginesStore.black.skill);
    tauri.startAnalysis(fen, skill, 1000, 3).then(id => analysisStore.reset(id));
  });
  ```
  Thinking time is fixed at 1000 ms for Plan 3 — strength is varied via Skill Level only.
- `engine_bestmove` handler: parse current FEN's side-to-move; if `enginesStore.engineFor(side)` is Stockfish, call `gameStore.makeMove(payload.mv)`.

## Data flow — canonical: user plays e2e4 vs Stockfish (Black)

1. User clicks/drags e2→e4. `gameStore.makeMove({from:"e2",to:"e4",promotion:null})` (Plan 2 path) → `currentFen` becomes the after-1.e4 position.
2. `$effect` in `App.svelte` fires (FEN changed). `enginesStore.anyEngine` is true → call `tauri.startAnalysis(currentFen, 10, 1000, 3)`. Receives `search_id = 7`. Calls `analysisStore.reset(7)`.
3. Backend: `EngineManager.start_analysis` writes UCI `stop`/`setoption`/`position`/`go` to Stockfish stdin. Spawns reader task. Returns 7.
4. Stockfish emits `info depth 1 multipv 1 score cp -25 pv e7e5`. Reader parses → emits `analysis_info` event `{search_id:7, depth:1, score:{kind:"Cp",value:-25}, pv:[{from:"e7",to:"e5",promotion:null}], ...}`. Frontend listener invokes `analysisStore.applyInfo`; `searchId` matches, line stored. `EvalBar` slides to show "Black slight advantage"; `AnalysisPanel` row 1 shows "e5  -0.25  d1".
5. More `info` events stream — deeper depths, multipv 2 and 3 populate. UI updates live.
6. After ~1000 ms, Stockfish emits `bestmove e7e5`. Reader emits `engine_bestmove` event `{search_id:7, mv:{from:"e7",to:"e5",promotion:null}}`. Frontend listener checks: `searchId == 7` ✓; side-to-move parsed from `currentFen` is "b"; `enginesStore.engineFor("b")` returns `{kind:"Stockfish",...}`. Calls `gameStore.makeMove(payload.mv)`.
7. Loop: `gameStore.makeMove` updates `currentFen`. The `$effect` from step 2 fires again, now analyzing the position after 1.e4 e5 from White's perspective. Continues until the game ends or the human stops responding.

Hot-seat mode (both sides Human): `anyEngine` is false, the `$effect` is a no-op. Eval bar shows neutral; AnalysisPanel is empty. Same as Plan 2.

## Error handling

| Failure | Handled where | User-visible result |
|---|---|---|
| Stockfish binary missing (sidecar resolution fails) | `EngineManager` on first spawn | `start_analysis` returns `Err("Stockfish binary not found...")`. Frontend toast + disables EnginePickers Stockfish options. |
| Stockfish process exits unexpectedly mid-search | Reader task detects EOF | Emits `engine_error` event. Frontend toast: "Engine crashed — restart the app." Next `start_analysis` re-spawns. |
| Malformed UCI line | `parser::parse_uci_line` returns `Malformed` | Reader skips the line, console-logs; analysis continues. |
| `start_analysis` rejection (mutex contention, spawn fail) | `tauri.startAnalysis()` rejects in JS | Console error; `analysisStore.searchId` stays null; eval bar blank. |
| `stop_analysis` for stale `search_id` | Backend silently no-ops | Silent. |
| `engine_bestmove` illegal in current FEN | `gameStore.makeMove` would throw | Should be impossible (Stockfish only emits legal moves on legal positions); treat as bug — log + skip. |
| User mid-search changes engine picker | Frontend `$effect` re-runs, fires new `start_analysis`; backend's first action is `stop`, ensuring clean transition | Smooth — old events filtered by `searchId`. |
| User undoes / loads new FEN during search | Same path as above — `$effect` re-fires on `currentFen` change | Smooth. |

**Conventions:**

- All Tauri commands return `Result<T, String>` (consistent with Plans 1-2).
- Reader task NEVER panics. All Stockfish output is best-effort; bad lines are logged and skipped.
- Frontend `tauri.ts` wrappers throw typed errors. App-level effect catches them and downgrades to console + toast.

## Testing

**In scope for Plan 3:**

- **chess-engine-api unit tests**: type construction, `Score::Cp` / `Mate` variants serialize via serde, `MockEngine` behaves correctly.
- **chess-engine-uci parser unit tests**: ~15 synthetic Stockfish strings cover info-at-depth-1, multipv 2/3, mate scores, promotions, malformed inputs.
- **chess-engine-uci integration test** (`#[ignore]`-flagged): spawns the real bundled Stockfish, analyzes the starting position with `movetime_ms=100`, asserts ≥1 info call and one bestmove call within 2 seconds.
- **Tauri command unit tests**: `start_analysis` and `stop_analysis` against a `MockEngine` (no real Stockfish). Verify `search_id` increments, events emit, `stop_analysis` with stale id no-ops.
- **Vitest store tests**: `analysisStore.applyInfo` filters by `searchId`, `evalPercent` sigmoid math matches known Lichess values (cp=0 → 0.5, cp=400 → ~0.73, Mate(n) → 1.0 or 0.0), `enginesStore.engineFor` / `anyEngine` / `engineForSideToMove` derivations.
- **Manual smoke checklist** at end of plan:
  - App opens, default pickers visible (`⚪ Human`, `⚫ Stockfish (10)`)
  - Play 1. e4 → eval bar updates within ~100 ms; analysis panel shows top-3 Black responses; Stockfish auto-plays its move within ~1.5 s
  - Switch Black to Human in the picker → engine stops auto-moving; eval bar/panel still update (hints-on for the human)
  - Switch BOTH to Human → analysis goes blank within ~1 s
  - Switch White back to Stockfish mid-game → Stockfish picks up on the current position and plays at the right time
  - Undo a move while Stockfish is mid-think → search restarts on the prior position cleanly
  - Set up Fool's Mate position via FEN, Stockfish Black → it should NOT move (game over); banner shows
  - Close the app → Stockfish process exits cleanly (verify Task Manager, no orphan)

**Deferred to a later plan:**

- DIY engine (entirely Plan 4+)
- Engine-vs-engine mode + pause/resume controls
- "Hints off" / no-spoil mode toggle
- In-app About panel with attribution
- Playwright e2e (continued deferral; same reasons as Plan 2)

**Verification before "done":** all unit tests green; `cargo test --include-ignored -p chess-engine-uci` green (real Stockfish integration test); `cargo build --release` succeeds with binary bundled; manual smoke checklist passes; no orphaned Stockfish processes after app close.

## Build order

Vertical slices. Each task ships a runnable, testable increment.

1. **Bundle Stockfish binary.** Download Stockfish 17 Windows official build. Place at `src-tauri/binaries/stockfish-x86_64-pc-windows-msvc.exe` (Tauri's required platform-suffixed name). Add to `tauri.conf.json` `bundle.externalBin`. Verify `npm run tauri build` produces an installer that ships the binary. No engine logic yet.
2. **Scaffold `chess-engine-api` crate.** Trait + types + `MockEngine`. Unit tests cover construction + serde round-trip on `Score`.
3. **`chess-engine-uci` UCI line parser.** Pure parser module. ~15 synthetic-input unit tests. No process spawning.
4. **`StockfishEngine`: spawn + handshake + analyze.** Full UCI dance via tokio subprocess + reader task. Integration test (`#[ignore]`) spawns the real binary and verifies a 100ms analysis returns info + bestmove.
5. **`EngineManager` Tauri state + 2 commands + mock-based tests.** `start_analysis` / `stop_analysis` registered. Events emitted. Mock-based unit tests so the Tauri suite doesn't need Stockfish.
6. **`enginesStore` + `EnginePickers` UI in Toolbar.** Pickers usable, Vitest tested. No analysis triggered yet — just stores chip state.
7. **`analysisStore` + event listeners + auto-trigger `$effect`.** Wire the reactive loop. Analysis runs on every FEN change when `anyEngine` is true. Console-verify event flow.
8. **`EvalBar` component.** Vertical bar reading `analysisStore.evalPercent`. Add grid column in App.svelte.
9. **`AnalysisPanel` component.** Top-3 moves table below `HistoryPanel`. Reads `analysisStore.lines`.
10. **Auto-play wiring.** `engine_bestmove` handler dispatches `gameStore.makeMove` when side-to-move is Stockfish. Vitest tested via mock stores.
11. **Polish + crash handling + smoke test.** Stockfish-died restart in `EngineManager`. Clean app-exit kill of child process. README update. Release build. Manual smoke checklist from Testing section.

## Done state

When this plan completes:

- `npm run tauri build` produces an installer with the Stockfish binary bundled
- All unit tests green (chess-engine-api + chess-engine-uci + new Tauri command tests + Vitest stores)
- `cargo test --include-ignored -p chess-engine-uci` green (real Stockfish integration test)
- Manual smoke checklist passes
- You can play either side vs Stockfish with adjustable skill level, see eval bar + top-3 candidate moves stream live, switch engines mid-game cleanly, and undo/load FEN without orphaning the Stockfish process
- No engine-vs-engine, no DIY engine — those are Plans 4+

## Out of scope for Plan 3

- DIY chess engine — Plans 4-8+ (perft → material → alpha-beta → quiescence → TT → ordering → eval → pruning)
- Engine-vs-engine mode (both sides Stockfish) — Plan 4+
- Pause/resume controls for engine searches — Plan 4+
- `/setup` route (board editor) — later
- `/engine-vs-engine` route — needs engine-vs-engine first
- Hints-off / no-spoil mode toggle — design polish plan
- Promotion picker UI (still auto-Queen from Plan 2)
- In-app About panel — design polish plan
- Per-engine settings persistence across app restarts
- Time controls / clocks
- Engine ELO calibration testing (we use Skill Level 0-20 as a UX proxy; precise Elo targeting is out)

## Risks & open questions

- **Stockfish binary distribution / antivirus false positives.** Bundled `.exe` inside an `.msi` may trigger Windows Defender SmartScreen on first launch. Mitigation: document this in README; defer code-signing to a later plan.
- **Tauri `externalBin` platform-suffix naming.** Tauri 2 requires the binary to be named `<name>-<target-triple>.exe` (e.g., `stockfish-x86_64-pc-windows-msvc.exe`). Easy to get wrong; build will fail loudly if so.
- **Tokio + Tauri integration.** Spawning tokio child processes from inside `#[tauri::command]` async fns works but requires the right `tokio` features (`process`, `rt-multi-thread`, `io-util`). Pin these in `chess-engine-uci`'s Cargo.toml.
- **`$effect` re-entrancy on `currentFen` change.** When the engine plays its move, `currentFen` changes again, which fires the `$effect` again. This is correct (we want it to analyze the new position) — but verify Svelte 5's `$effect` doesn't infinite-loop. Mitigation: the effect only triggers `start_analysis`; the analysis itself completes asynchronously and doesn't synchronously re-trigger the effect.
- **Reader task cleanup on app exit.** Need a Tauri `RunEvent::Exit` handler that kills the Stockfish child process. Otherwise orphan processes accumulate during dev iteration.
