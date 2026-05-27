# Plan 3 — Stockfish Engine Integration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add Stockfish to the existing app. Engine picker per side (Human / Stockfish + Skill 0-20). Stockfish auto-plays when it's its turn. Always-on analysis: vertical eval bar left of board + top-3 candidate moves below history sidebar.

**Architecture:** Two new crates — `chess-engine-api` (pure trait + types) and `chess-engine-uci` (Stockfish UCI subprocess adapter using tokio). Single long-lived Stockfish process managed by an `EngineManager` Tauri state. Streaming analysis via two Tauri events (`analysis_info`, `engine_bestmove`) filtered on the frontend by an incrementing `search_id`. No DIY engine — Plan 4+.

**Tech Stack:** Tauri 2.x, Svelte 5 (runes), Vite 5, TypeScript 5, Vitest, tokio 1 (process + io-util + rt-multi-thread + sync features), Stockfish 17 official Windows release bundled as a Tauri `externalBin`.

**Spec:** `docs/superpowers/specs/2026-05-26-plan-3-stockfish-engine-design.md` is the source of truth — re-read it before each task.

**Known Plan 3 limitations (deferred):**
- DIY chess engine — entirely Plan 4+
- Engine-vs-engine mode (both sides Stockfish) — Plan 4+
- "Hints off" / no-spoil mode toggle — design polish plan
- Promotion picker UI (still auto-Queen)
- In-app About panel with attribution
- Per-engine settings persistence across app restarts
- Time controls / clocks

**Notes for the implementer:**
- **Visual verification is your human partner's job.** You cannot see the GUI. Use `cargo test` + `npm test` + `npm run build` exit codes as your acceptance signal; skip "open the app and see X" steps.
- **`npm run tauri build` is slow** (first time: 1-3 min for the Rust release compile, plus 30+ seconds to bundle the 30 MB binary). Use `timeout: 600000` on Bash calls.
- **Stockfish integration test is `#[ignore]`-flagged** so unit-test runs don't fail on missing binary. Run it with `cargo test --include-ignored -p chess-engine-uci`.

---

## File structure

```
chess-engine/
├── Cargo.toml                                                MODIFY: workspace deps (tokio + new crates)
├── src-tauri/
│   ├── Cargo.toml                                            MODIFY: dep on chess-engine-api + chess-engine-uci + tokio
│   ├── tauri.conf.json                                       MODIFY: bundle.externalBin entry
│   ├── binaries/
│   │   └── stockfish-x86_64-pc-windows-msvc.exe              NEW (~30 MB, Stockfish 17 official Windows release)
│   ├── crates/
│   │   ├── chess-core/                                       (unchanged)
│   │   ├── chess-engine-api/                                 NEW
│   │   │   ├── Cargo.toml
│   │   │   └── src/lib.rs                                    — Engine trait, Score, SearchLimits, AnalysisInfo, MockEngine
│   │   └── chess-engine-uci/                                 NEW
│   │       ├── Cargo.toml
│   │       └── src/
│   │           ├── lib.rs                                    — StockfishEngine struct, Engine impl
│   │           └── parser.rs                                 — pure UCI line parser
│   └── src/
│       ├── lib.rs                                            MODIFY: app.manage(EngineManager), register 2 commands, RunEvent::Exit cleanup
│       └── commands.rs                                       MODIFY: start_analysis, stop_analysis commands + DTOs + tests
└── src/
    └── lib/
        ├── tauri.ts                                          MODIFY: engine command wrappers + event types + listeners
        ├── stores/
        │   ├── engines.svelte.ts                             NEW
        │   ├── engines.test.ts                               NEW
        │   ├── analysis.svelte.ts                            NEW
        │   └── analysis.test.ts                              NEW
        ├── panels/
        │   ├── Toolbar.svelte                                MODIFY: mount <EnginePickers />
        │   ├── EnginePickers.svelte                          NEW: two chips with popover (Human/Stockfish radio + Skill slider)
        │   ├── EvalBar.svelte                                NEW: vertical bar, Lichess sigmoid mapping
        │   └── AnalysisPanel.svelte                          NEW: top-3 moves table below HistoryPanel
        └── App.svelte                                        MODIFY: add EvalBar column, AnalysisPanel slot, $effect + listeners
```

---

### Task 1: Bundle Stockfish binary

**Goal:** Stockfish 17's Windows binary lives in `src-tauri/binaries/`, gets bundled by `npm run tauri build`, and the rebuilt app still launches. No engine logic yet.

**Files:**
- Create: `src-tauri/binaries/stockfish-x86_64-pc-windows-msvc.exe` (downloaded)
- Modify: `src-tauri/tauri.conf.json` (add to `bundle.externalBin`)

- [ ] **Step 1: Download Stockfish 17 Windows official build**

In PowerShell:

```powershell
Set-Location C:\Users\apbat\Projects\chess-engine
New-Item -ItemType Directory -Force src-tauri/binaries | Out-Null
# Stockfish 17 official release. The "popcnt" variant is the safest broadly-compatible x86_64 build.
$url = "https://github.com/official-stockfish/Stockfish/releases/download/sf_17/stockfish-windows-x86-64-modern.zip"
$zip = "$env:TEMP\stockfish.zip"
Invoke-WebRequest $url -OutFile $zip
Expand-Archive $zip -DestinationPath "$env:TEMP\stockfish_extract" -Force
# Find the exe and copy it with the Tauri-required platform-suffixed name.
$exe = Get-ChildItem -Recurse "$env:TEMP\stockfish_extract" -Filter "stockfish*.exe" | Select-Object -First 1
Copy-Item $exe.FullName "src-tauri/binaries/stockfish-x86_64-pc-windows-msvc.exe"
Remove-Item $zip
Remove-Item "$env:TEMP\stockfish_extract" -Recurse
```

Verify: `(Get-Item src-tauri/binaries/stockfish-x86_64-pc-windows-msvc.exe).Length` should be roughly 30-80 MB.

If the URL 404s (asset names sometimes change between releases), browse https://github.com/official-stockfish/Stockfish/releases/tag/sf_17 and download the equivalent "modern" or "popcnt" Windows build, then copy to the same target filename.

- [ ] **Step 2: Smoke-test the binary directly**

```powershell
echo "uci`nquit" | & .\src-tauri\binaries\stockfish-x86_64-pc-windows-msvc.exe
```

Expected: output contains a line `id name Stockfish 17` (or similar) followed by `uciok`. If you get "is not recognized" or similar, the file is missing or corrupted — re-download.

- [ ] **Step 3: Add the binary to `tauri.conf.json` bundle config**

Edit `src-tauri/tauri.conf.json`. In the `bundle` object (alongside the existing `icon` array), add an `externalBin` entry. The path is given WITHOUT the `.exe` extension and WITHOUT the platform suffix — Tauri adds both per platform.

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "chess-engine",
  "version": "0.1.0",
  "identifier": "com.batista.chess-engine",
  "build": { /* unchanged */ },
  "app": { /* unchanged */ },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.ico"],
    "externalBin": ["binaries/stockfish"]
  }
}
```

Tauri will look for `binaries/stockfish-x86_64-pc-windows-msvc.exe` on Windows because of the platform-target-triple suffix convention.

- [ ] **Step 4: Verify the dev build still works**

Run: `cargo check -p chess-engine-app`
Expected: exit 0.

(Don't run `npm run tauri dev` — you can't see the window. The bundle verification happens in Step 5.)

- [ ] **Step 5: Verify the release build bundles the binary**

Run: `npm run tauri build`
(Long! Use `timeout: 600000`.)
Expected: exit 0. The release binary at `target/release/chess-engine-app.exe` should be roughly the same size as before; the bundled `.msi` at `target/release/bundle/msi/chess-engine_0.1.0_x64_en-US.msi` should now be ~30 MB LARGER than the Plan 2 baseline because it ships Stockfish.

Verify the binary is in the bundle's resources by checking the MSI grew significantly:

```powershell
(Get-Item target/release/bundle/msi/chess-engine_0.1.0_x64_en-US.msi).Length / 1MB
```

Expected: ~30 MB or more (was ~3 MB at end of Plan 2).

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/binaries/ src-tauri/tauri.conf.json
git commit -m "feat: bundle Stockfish 17 binary as Tauri externalBin"
```

---

### Task 2: Scaffold `chess-engine-api` crate

**Goal:** New workspace crate at `src-tauri/crates/chess-engine-api/` with pure trait + types + a `MockEngine` for downstream tests. Compiles, unit tests pass.

**Files:**
- Create: `src-tauri/crates/chess-engine-api/Cargo.toml`
- Create: `src-tauri/crates/chess-engine-api/src/lib.rs`
- Modify: `Cargo.toml` (workspace root — `chess-engine-api` is automatically picked up via the existing `src-tauri/crates/*` glob; verify no manual change needed)

- [ ] **Step 1: Create the crate manifest**

`src-tauri/crates/chess-engine-api/Cargo.toml`:

```toml
[package]
name = "chess-engine-api"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
chess-core = { path = "../chess-core" }
serde = { workspace = true }

[dev-dependencies]
serde_json = { workspace = true }
```

- [ ] **Step 2: Verify the workspace picks up the new member**

The workspace `Cargo.toml` has `members = ["src-tauri", "src-tauri/crates/*"]` from Plan 1 — the new crate is auto-included.

Run: `cargo check -p chess-engine-api`
Expected: error — `src/lib.rs` doesn't exist yet.

- [ ] **Step 3: Write failing tests + the type stubs**

`src-tauri/crates/chess-engine-api/src/lib.rs`:

```rust
//! Pure types for engine abstraction. No async runtime, no I/O.

use chess_core::prelude::{Color, Move, Position};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Score {
    /// Centipawns from the side-to-move's perspective.
    Cp(i32),
    /// Forced mate in N plies (positive = side to move wins, negative = side to move loses).
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
    pub pv: Vec<Move>,
    pub multipv_index: u8, // 1-based per UCI convention
    pub nodes: u64,
    pub nps: u64,
    pub time_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineKind {
    Human,
    Stockfish,
}

/// One trait, two callbacks: `on_info` per UCI info line, `on_bestmove` once at the end.
/// Callback-based to keep the trait runtime-agnostic.
pub trait Engine {
    fn name(&self) -> &str;
    fn analyze(
        &mut self,
        position: Position,
        limits: SearchLimits,
        skill_level: u8,
        on_info: Box<dyn FnMut(AnalysisInfo) + Send>,
        on_bestmove: Box<dyn FnOnce(Move) + Send>,
    );
    fn stop(&mut self);
}

/// In-process test double — emits one canned info, then one canned bestmove.
/// Use the builder methods to configure what each `analyze()` call yields.
#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use super::*;

    pub struct MockEngine {
        canned_info: Option<AnalysisInfo>,
        canned_bestmove: Option<Move>,
        stop_called: bool,
    }

    impl MockEngine {
        pub fn new(canned_bestmove: Move) -> Self {
            Self {
                canned_info: None,
                canned_bestmove: Some(canned_bestmove),
                stop_called: false,
            }
        }
        pub fn with_info(mut self, info: AnalysisInfo) -> Self {
            self.canned_info = Some(info);
            self
        }
        pub fn was_stopped(&self) -> bool { self.stop_called }
    }

    impl Engine for MockEngine {
        fn name(&self) -> &str { "MockEngine" }
        fn analyze(
            &mut self,
            _position: Position,
            _limits: SearchLimits,
            _skill_level: u8,
            mut on_info: Box<dyn FnMut(AnalysisInfo) + Send>,
            on_bestmove: Box<dyn FnOnce(Move) + Send>,
        ) {
            if let Some(info) = self.canned_info.clone() {
                on_info(info);
            }
            if let Some(mv) = self.canned_bestmove {
                on_bestmove(mv);
            }
        }
        fn stop(&mut self) { self.stop_called = true; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::prelude::*;

    #[test]
    fn score_cp_round_trips_via_serde() {
        let s = Score::Cp(42);
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(j, r#"{"kind":"Cp","value":42}"#);
        let back: Score = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn score_mate_round_trips_via_serde() {
        let s = Score::Mate(-3);
        let back: Score = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn engine_trait_is_object_safe() {
        // Compile-time check: trait can live behind a Box<dyn Engine>.
        fn _accepts(_: Box<dyn Engine + Send>) {}
    }

    #[test]
    fn mock_engine_emits_info_then_bestmove_and_stop() {
        use crate::mock::MockEngine;
        use std::sync::{Arc, Mutex};

        let m1 = Move::new(
            Square::new(File::E, Rank::Two),
            Square::new(File::E, Rank::Four),
            MoveFlag::DoublePawnPush,
        );
        let info = AnalysisInfo {
            depth: 1, score: Score::Cp(25), pv: vec![m1],
            multipv_index: 1, nodes: 100, nps: 1000, time_ms: 100,
        };
        let mut eng = MockEngine::new(m1).with_info(info.clone());

        let info_calls: Arc<Mutex<Vec<AnalysisInfo>>> = Default::default();
        let bestmove_calls: Arc<Mutex<Vec<Move>>> = Default::default();
        let ic = info_calls.clone();
        let bc = bestmove_calls.clone();

        eng.analyze(
            Position::starting(),
            SearchLimits { movetime_ms: 100, multipv: 1 },
            10,
            Box::new(move |i| ic.lock().unwrap().push(i)),
            Box::new(move |m| bc.lock().unwrap().push(m)),
        );

        assert_eq!(info_calls.lock().unwrap().len(), 1);
        assert_eq!(info_calls.lock().unwrap()[0].depth, 1);
        assert_eq!(bestmove_calls.lock().unwrap().len(), 1);
        assert_eq!(bestmove_calls.lock().unwrap()[0], m1);

        assert!(!eng.was_stopped());
        eng.stop();
        assert!(eng.was_stopped());
    }
}
```

- [ ] **Step 4: Run tests, verify they pass**

Run: `cargo test -p chess-engine-api`
Expected: 4 tests pass.

- [ ] **Step 5: Commit**

```powershell
git add src-tauri/crates/chess-engine-api/
git commit -m "feat(chess-engine-api): scaffold Engine trait + types + MockEngine"
```

---

### Task 3: `chess-engine-uci` UCI line parser

**Goal:** Pure UCI line parser at `src-tauri/crates/chess-engine-uci/src/parser.rs`. No process spawning yet. Heavy unit tests against synthetic Stockfish output.

**Files:**
- Create: `src-tauri/crates/chess-engine-uci/Cargo.toml`
- Create: `src-tauri/crates/chess-engine-uci/src/lib.rs`
- Create: `src-tauri/crates/chess-engine-uci/src/parser.rs`

- [ ] **Step 1: Create the crate manifest**

`src-tauri/crates/chess-engine-uci/Cargo.toml`:

```toml
[package]
name = "chess-engine-uci"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true

[dependencies]
chess-core = { path = "../chess-core" }
chess-engine-api = { path = "../chess-engine-api" }
tokio = { workspace = true }

[dev-dependencies]
```

- [ ] **Step 2: Add tokio to workspace deps**

In the workspace root `Cargo.toml`, extend `[workspace.dependencies]`:

```toml
[workspace.dependencies]
arrayvec = "0.7"
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }
tauri-plugin-dialog = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["process", "io-util", "rt-multi-thread", "sync", "macros"] }
```

(`tauri-plugin-dialog` is already an explicit dep in `src-tauri/Cargo.toml` from Plan 2; promote it to a workspace dep here so the version pins live in one place. Then update `src-tauri/Cargo.toml`'s `tauri-plugin-dialog = "2"` to `tauri-plugin-dialog = { workspace = true }` in Task 5 — for now, leave src-tauri alone and just add the workspace entry.)

- [ ] **Step 3: Create `src/lib.rs` (parser module declaration only for now)**

`src-tauri/crates/chess-engine-uci/src/lib.rs`:

```rust
//! Stockfish UCI subprocess adapter.

pub mod parser;
```

- [ ] **Step 4: Write the failing parser tests**

`src-tauri/crates/chess-engine-uci/src/parser.rs`:

```rust
use chess_core::prelude::{File, PieceKind, Rank, Square};
use chess_engine_api::Score;

/// A raw move parsed from UCI long algebraic notation (e.g. "e2e4", "e7e8q").
/// Position-independent — the caller resolves to a chess_core::Move against the position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

/// Information from a UCI `info` line. Score is from the side-to-move's perspective.
#[derive(Clone, Debug, PartialEq)]
pub struct RawInfo {
    pub depth: u8,
    pub score: Score,
    pub pv: Vec<RawMove>,
    pub multipv_index: u8,
    pub nodes: u64,
    pub nps: u64,
    pub time_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParsedLine {
    Info(RawInfo),
    BestMove(RawMove),
    /// id, uciok, readyok, option lines etc. Caller decides what (if anything) to do with them.
    Other(String),
    /// Could not parse. Caller logs and continues.
    Malformed(String),
}

pub fn parse_uci_line(line: &str) -> ParsedLine {
    let line = line.trim();
    if line.is_empty() { return ParsedLine::Other(String::new()); }

    if let Some(rest) = line.strip_prefix("bestmove ") {
        // "bestmove e2e4" or "bestmove e7e8q" or "bestmove (none)"
        let first = rest.split_whitespace().next().unwrap_or("");
        if first == "(none)" { return ParsedLine::Malformed(line.into()); }
        return match parse_raw_move(first) {
            Some(m) => ParsedLine::BestMove(m),
            None => ParsedLine::Malformed(line.into()),
        };
    }

    if let Some(rest) = line.strip_prefix("info ") {
        return parse_info(rest).map(ParsedLine::Info).unwrap_or_else(|| ParsedLine::Other(line.into()));
    }

    ParsedLine::Other(line.into())
}

fn parse_raw_move(s: &str) -> Option<RawMove> {
    let bytes = s.as_bytes();
    if bytes.len() < 4 || bytes.len() > 5 { return None; }
    let from_file = File::from_index(bytes[0].wrapping_sub(b'a'))?;
    let from_rank = Rank::from_index(bytes[1].wrapping_sub(b'1'))?;
    let to_file   = File::from_index(bytes[2].wrapping_sub(b'a'))?;
    let to_rank   = Rank::from_index(bytes[3].wrapping_sub(b'1'))?;
    let promotion = if bytes.len() == 5 {
        Some(match bytes[4] {
            b'q' => PieceKind::Queen,
            b'r' => PieceKind::Rook,
            b'b' => PieceKind::Bishop,
            b'n' => PieceKind::Knight,
            _ => return None,
        })
    } else { None };
    Some(RawMove {
        from: Square::new(from_file, from_rank),
        to: Square::new(to_file, to_rank),
        promotion,
    })
}

fn parse_info(rest: &str) -> Option<RawInfo> {
    // Tokenize and walk: info has key/value pairs except `pv` and `score` which have multi-token values.
    let mut depth: Option<u8> = None;
    let mut score: Option<Score> = None;
    let mut multipv_index: u8 = 1; // default
    let mut nodes: u64 = 0;
    let mut nps: u64 = 0;
    let mut time_ms: u32 = 0;
    let mut pv: Vec<RawMove> = Vec::new();

    let toks: Vec<&str> = rest.split_whitespace().collect();
    let mut i = 0usize;
    while i < toks.len() {
        match toks[i] {
            "depth" => {
                depth = toks.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "multipv" => {
                multipv_index = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(1);
                i += 2;
            }
            "nodes" => {
                nodes = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "nps" => {
                nps = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "time" => {
                time_ms = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "score" => {
                let kind = toks.get(i + 1).copied().unwrap_or("");
                let value: i32 = toks.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(0);
                score = Some(match kind {
                    "cp"   => Score::Cp(value),
                    "mate" => Score::Mate(value as i8),
                    _ => return None,
                });
                i += 3;
                // Skip optional "lowerbound"/"upperbound" qualifiers.
                while let Some(&t) = toks.get(i) {
                    if t == "lowerbound" || t == "upperbound" { i += 1; } else { break; }
                }
            }
            "pv" => {
                i += 1;
                while let Some(&t) = toks.get(i) {
                    if let Some(m) = parse_raw_move(t) {
                        pv.push(m);
                        i += 1;
                    } else { break; }
                }
            }
            _ => { i += 1; } // Skip unknown tokens (currmove, hashfull, tbhits, etc.)
        }
    }

    Some(RawInfo {
        depth: depth?,
        score: score?,
        pv,
        multipv_index,
        nodes,
        nps,
        time_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::prelude::*;

    fn sq(file: File, rank: Rank) -> Square { Square::new(file, rank) }

    #[test]
    fn parses_bestmove_quiet() {
        let p = parse_uci_line("bestmove e2e4");
        assert_eq!(p, ParsedLine::BestMove(RawMove {
            from: sq(File::E, Rank::Two),
            to:   sq(File::E, Rank::Four),
            promotion: None,
        }));
    }

    #[test]
    fn parses_bestmove_with_promotion() {
        let p = parse_uci_line("bestmove e7e8q");
        let RawMove { promotion, .. } = match p { ParsedLine::BestMove(m) => m, _ => panic!() };
        assert_eq!(promotion, Some(PieceKind::Queen));
    }

    #[test]
    fn parses_bestmove_with_underpromotion() {
        for (ch, want) in [('r', PieceKind::Rook), ('b', PieceKind::Bishop), ('n', PieceKind::Knight)] {
            let line = format!("bestmove e7e8{ch}");
            let m = match parse_uci_line(&line) { ParsedLine::BestMove(m) => m, _ => panic!() };
            assert_eq!(m.promotion, Some(want));
        }
    }

    #[test]
    fn bestmove_none_is_malformed() {
        let p = parse_uci_line("bestmove (none)");
        assert!(matches!(p, ParsedLine::Malformed(_)));
    }

    #[test]
    fn parses_simple_info_line() {
        let line = "info depth 1 seldepth 1 multipv 1 score cp -25 nodes 21 nps 21000 time 1 pv e7e5";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, p => panic!("{p:?}") };
        assert_eq!(info.depth, 1);
        assert_eq!(info.score, Score::Cp(-25));
        assert_eq!(info.multipv_index, 1);
        assert_eq!(info.nodes, 21);
        assert_eq!(info.nps, 21000);
        assert_eq!(info.time_ms, 1);
        assert_eq!(info.pv.len(), 1);
        assert_eq!(info.pv[0].from, sq(File::E, Rank::Seven));
        assert_eq!(info.pv[0].to,   sq(File::E, Rank::Five));
    }

    #[test]
    fn parses_info_with_multipv_3() {
        let line = "info depth 14 multipv 3 score cp 28 nodes 50000 nps 100000 time 500 pv e2e4 e7e5";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.multipv_index, 3);
        assert_eq!(info.pv.len(), 2);
    }

    #[test]
    fn parses_mate_score() {
        let line = "info depth 12 multipv 1 score mate 3 nodes 1000 nps 10000 time 100 pv d1h5 g7g6";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.score, Score::Mate(3));
    }

    #[test]
    fn parses_mate_score_negative() {
        let line = "info depth 5 multipv 1 score mate -2 nodes 100 nps 1000 time 10 pv h2h3";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.score, Score::Mate(-2));
    }

    #[test]
    fn skips_lowerbound_upperbound_qualifiers() {
        let line = "info depth 8 multipv 1 score cp 50 lowerbound nodes 100 nps 1000 time 5 pv e2e4";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.score, Score::Cp(50));
        assert_eq!(info.nodes, 100);
    }

    #[test]
    fn skips_unknown_tokens_like_currmove() {
        let line = "info depth 10 currmove e2e4 currmovenumber 1 score cp 20 nodes 1000 nps 5000 time 200 pv e2e4 e7e5 g1f3";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.depth, 10);
        assert_eq!(info.pv.len(), 3);
    }

    #[test]
    fn pv_stops_at_first_non_move_token() {
        // PV must be the LAST key per UCI; non-move tokens after it shouldn't appear in real output,
        // but our parser stops at the first non-move regardless.
        let line = "info depth 4 multipv 1 score cp 0 nodes 10 nps 100 time 1 pv e2e4 notamove";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.pv.len(), 1);
    }

    #[test]
    fn missing_depth_or_score_yields_other() {
        // Info without depth is non-actionable; we tolerate by treating it as Other.
        let p = parse_uci_line("info string Some informational text");
        assert!(matches!(p, ParsedLine::Other(_)));
    }

    #[test]
    fn uciok_and_readyok_are_other() {
        assert!(matches!(parse_uci_line("uciok"),   ParsedLine::Other(_)));
        assert!(matches!(parse_uci_line("readyok"), ParsedLine::Other(_)));
    }

    #[test]
    fn id_lines_are_other() {
        assert!(matches!(parse_uci_line("id name Stockfish 17"), ParsedLine::Other(_)));
        assert!(matches!(parse_uci_line("id author the Stockfish developers"), ParsedLine::Other(_)));
    }

    #[test]
    fn option_lines_are_other() {
        let line = "option name Hash type spin default 16 min 1 max 33554432";
        assert!(matches!(parse_uci_line(line), ParsedLine::Other(_)));
    }

    #[test]
    fn empty_line_is_other() {
        assert!(matches!(parse_uci_line(""), ParsedLine::Other(_)));
        assert!(matches!(parse_uci_line("   "), ParsedLine::Other(_)));
    }
}
```

- [ ] **Step 5: Run the tests**

Run: `cargo test -p chess-engine-uci`
Expected: 15 tests pass.

- [ ] **Step 6: Commit**

```powershell
git add Cargo.toml src-tauri/crates/chess-engine-uci/
git commit -m "feat(chess-engine-uci): UCI line parser with synthetic-input tests"
```

---

### Task 4: `StockfishEngine` — spawn + handshake + analyze

**Goal:** Implement the `Engine` trait by spawning Stockfish, running the UCI handshake, and driving search via the parser. Integration test (`#[ignore]`) spawns the real bundled binary.

**Files:**
- Modify: `src-tauri/crates/chess-engine-uci/src/lib.rs`
- Create: `src-tauri/crates/chess-engine-uci/tests/integration.rs`

- [ ] **Step 1: Replace `src/lib.rs` with the StockfishEngine implementation**

```rust
//! Stockfish UCI subprocess adapter.

pub mod parser;

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use chess_core::prelude::{
    legal_moves, make_move, move_to_san, Color, Move, MoveFlag, PieceKind, Position,
};
use chess_engine_api::{AnalysisInfo, Engine, SearchLimits, Score};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::mpsc;

use parser::{parse_uci_line, ParsedLine, RawInfo, RawMove};

pub struct StockfishEngine {
    binary_path: PathBuf,
    runtime: tokio::runtime::Handle,
    // Lazy-spawn state. None until first analyze().
    handles: Arc<Mutex<Option<ProcessHandles>>>,
    // Sender to the running reader task, used by stop().
    stop_tx: Arc<Mutex<Option<mpsc::UnboundedSender<StopSignal>>>>,
}

struct ProcessHandles {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    _child: tokio::process::Child,
}

enum StopSignal { Stop }

impl StockfishEngine {
    pub fn new(binary_path: PathBuf, runtime: tokio::runtime::Handle) -> Self {
        Self {
            binary_path,
            runtime,
            handles: Arc::new(Mutex::new(None)),
            stop_tx: Arc::new(Mutex::new(None)),
        }
    }

    fn ensure_spawned(&self) -> Result<(), String> {
        let mut guard = self.handles.lock().unwrap();
        if guard.is_some() { return Ok(()); }

        let handles = self.runtime.block_on(async {
            let mut child = tokio::process::Command::new(&self.binary_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| format!("Could not spawn Stockfish: {e}"))?;
            let stdin = child.stdin.take().ok_or("no stdin")?;
            let stdout = BufReader::new(child.stdout.take().ok_or("no stdout")?);
            Ok::<_, String>(ProcessHandles { stdin, stdout, _child: child })
        })?;

        // Drive the UCI handshake.
        let ProcessHandles { mut stdin, mut stdout, _child } = handles;
        self.runtime.block_on(async {
            stdin.write_all(b"uci\n").await.map_err(|e| e.to_string())?;
            stdin.flush().await.map_err(|e| e.to_string())?;
            let mut line = String::new();
            loop {
                line.clear();
                let n = stdout.read_line(&mut line).await.map_err(|e| e.to_string())?;
                if n == 0 { return Err("Stockfish closed stdout before uciok".into()); }
                if line.trim() == "uciok" { break; }
            }
            stdin.write_all(b"isready\n").await.map_err(|e| e.to_string())?;
            stdin.flush().await.map_err(|e| e.to_string())?;
            loop {
                line.clear();
                let n = stdout.read_line(&mut line).await.map_err(|e| e.to_string())?;
                if n == 0 { return Err("Stockfish closed stdout before readyok".into()); }
                if line.trim() == "readyok" { break; }
            }
            Ok::<_, String>(())
        })?;

        *guard = Some(ProcessHandles { stdin, stdout, _child });
        Ok(())
    }
}

impl Engine for StockfishEngine {
    fn name(&self) -> &str { "Stockfish" }

    fn analyze(
        &mut self,
        position: Position,
        limits: SearchLimits,
        skill_level: u8,
        mut on_info: Box<dyn FnMut(AnalysisInfo) + Send>,
        on_bestmove: Box<dyn FnOnce(Move) + Send>,
    ) {
        if let Err(e) = self.ensure_spawned() {
            eprintln!("[StockfishEngine] spawn failed: {e}");
            // Best-effort: still call on_bestmove with a synthetic null? No — drop on the floor.
            return;
        }

        let fen = chess_core::prelude::serialize_fen(&position);
        let cmds = format!(
            "stop\nsetoption name Skill Level value {skill}\nsetoption name MultiPV value {mpv}\nposition fen {fen}\ngo movetime {ms}\n",
            skill = skill_level.min(20),
            mpv = limits.multipv.max(1),
            ms = limits.movetime_ms,
        );

        let (stop_tx, mut stop_rx) = mpsc::unbounded_channel::<StopSignal>();
        *self.stop_tx.lock().unwrap() = Some(stop_tx);

        let handles_arc = self.handles.clone();
        let runtime = self.runtime.clone();

        // Synchronously drive the analyze loop on the runtime. Returns when bestmove arrives.
        runtime.block_on(async move {
            let mut guard = handles_arc.lock().unwrap();
            let h = guard.as_mut().expect("ensure_spawned succeeded above");
            if let Err(e) = h.stdin.write_all(cmds.as_bytes()).await {
                eprintln!("[StockfishEngine] write failed: {e}");
                return;
            }
            if let Err(e) = h.stdin.flush().await { eprintln!("[StockfishEngine] flush failed: {e}"); return; }

            let mut line = String::new();
            let mut maybe_bestmove: Option<Move> = None;
            loop {
                line.clear();
                let read_fut = h.stdout.read_line(&mut line);
                tokio::select! {
                    _ = stop_rx.recv() => {
                        // Send `stop` and keep reading until bestmove arrives so Stockfish stays in a clean state.
                        let _ = h.stdin.write_all(b"stop\n").await;
                        let _ = h.stdin.flush().await;
                    }
                    res = read_fut => {
                        let n = match res { Ok(n) => n, Err(_) => break };
                        if n == 0 { break; } // EOF — process died.
                        match parse_uci_line(line.trim_end()) {
                            ParsedLine::Info(raw) => {
                                if let Some(info) = raw_to_analysis_info(raw, &position) {
                                    on_info(info);
                                }
                            }
                            ParsedLine::BestMove(rm) => {
                                maybe_bestmove = raw_to_move(rm, &position);
                                break;
                            }
                            ParsedLine::Other(_) | ParsedLine::Malformed(_) => {}
                        }
                    }
                }
            }

            if let Some(m) = maybe_bestmove { on_bestmove(m); }
        });

        *self.stop_tx.lock().unwrap() = None;
    }

    fn stop(&mut self) {
        if let Some(tx) = self.stop_tx.lock().unwrap().as_ref() {
            let _ = tx.send(StopSignal::Stop);
        }
    }
}

/// Resolve a parser::RawMove against a Position to a chess_core::Move.
fn raw_to_move(rm: RawMove, pos: &Position) -> Option<Move> {
    legal_moves(pos).into_iter().find(|m| {
        if m.from() != rm.from || m.to() != rm.to { return false; }
        if m.flag().is_promotion() {
            promo_kind_of_flag(m.flag()) == rm.promotion
        } else {
            rm.promotion.is_none()
        }
    })
}

fn promo_kind_of_flag(flag: MoveFlag) -> Option<PieceKind> {
    use MoveFlag::*;
    Some(match flag {
        PromoKnight | PromoCaptureN => PieceKind::Knight,
        PromoBishop | PromoCaptureB => PieceKind::Bishop,
        PromoRook | PromoCaptureR   => PieceKind::Rook,
        PromoQueen | PromoCaptureQ  => PieceKind::Queen,
        _ => return None,
    })
}

/// Convert a parser::RawInfo (with raw moves) to an AnalysisInfo with chess_core::Move PV.
/// Walks the position forward through each PV move to resolve subsequent moves correctly.
/// Returns None if any move in the PV fails to resolve.
fn raw_to_analysis_info(raw: RawInfo, base_pos: &Position) -> Option<AnalysisInfo> {
    let mut pos = base_pos.clone();
    let mut pv: Vec<Move> = Vec::with_capacity(raw.pv.len());
    for rm in raw.pv {
        let m = raw_to_move(rm, &pos)?;
        make_move(&mut pos, m);
        pv.push(m);
    }
    Some(AnalysisInfo {
        depth: raw.depth,
        score: raw.score,
        pv,
        multipv_index: raw.multipv_index,
        nodes: raw.nodes,
        nps: raw.nps,
        time_ms: raw.time_ms,
    })
}

/// Render a PV (sequence of chess_core::Move) starting from `pos` as SAN strings.
/// Caller uses this to populate Tauri events for the frontend.
pub fn pv_to_san(pos: &Position, pv: &[Move]) -> Vec<String> {
    let mut p = pos.clone();
    let mut out = Vec::with_capacity(pv.len());
    for &m in pv {
        out.push(move_to_san(&p, m));
        make_move(&mut p, m);
    }
    out
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p chess-engine-uci`
Expected: exit 0. May produce warnings about unused imports; clean those up if any.

- [ ] **Step 3: Write the integration test**

`src-tauri/crates/chess-engine-uci/tests/integration.rs`:

```rust
#![cfg(test)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chess_core::prelude::*;
use chess_engine_api::{Engine, SearchLimits};
use chess_engine_uci::StockfishEngine;

fn binary_path() -> PathBuf {
    // The bundled binary lives at <workspace-root>/src-tauri/binaries/stockfish-x86_64-pc-windows-msvc.exe.
    // CARGO_MANIFEST_DIR for an integration test is the crate root (chess-engine-uci),
    // which is <workspace>/src-tauri/crates/chess-engine-uci. Walk up two levels.
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // chess-engine-uci -> crates
    p.pop(); // crates -> src-tauri
    p.push("binaries");
    p.push("stockfish-x86_64-pc-windows-msvc.exe");
    p
}

#[ignore = "spawns real Stockfish — run with --include-ignored"]
#[test]
fn stockfish_analyzes_starting_position() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let mut engine = StockfishEngine::new(binary_path(), rt.handle().clone());

    let infos: Arc<Mutex<Vec<chess_engine_api::AnalysisInfo>>> = Default::default();
    let bestmove: Arc<Mutex<Option<chess_core::prelude::Move>>> = Default::default();

    let ic = infos.clone();
    let bc = bestmove.clone();

    engine.analyze(
        Position::starting(),
        SearchLimits { movetime_ms: 200, multipv: 3 },
        20,
        Box::new(move |i| ic.lock().unwrap().push(i)),
        Box::new(move |m| *bc.lock().unwrap() = Some(m)),
    );

    let infos_len = infos.lock().unwrap().len();
    assert!(infos_len >= 1, "expected at least one info line, got {infos_len}");

    let m = bestmove.lock().unwrap().expect("expected a bestmove");
    // Starting-position legal moves include the 20 standard openings; any of them is acceptable.
    let legals = legal_moves(&Position::starting());
    assert!(legals.into_iter().any(|legal| legal == m));
}
```

- [ ] **Step 4: Run unit tests (excluding the integration test)**

Run: `cargo test -p chess-engine-uci`
Expected: 15 parser tests pass; integration test is shown as `1 ignored`.

- [ ] **Step 5: Run the integration test**

Run: `cargo test --include-ignored -p chess-engine-uci`
Expected: 16 tests pass (15 parser + 1 integration). Integration test takes ~0.5 s.

- [ ] **Step 6: Commit**

```powershell
git add src-tauri/crates/chess-engine-uci/
git commit -m "feat(chess-engine-uci): StockfishEngine with spawn/handshake/analyze + integration test"
```

---

### Task 5: `EngineManager` Tauri state + 2 commands

**Goal:** Add `EngineManager` to Tauri state. Implement `start_analysis` and `stop_analysis` commands. Emit `analysis_info` and `engine_bestmove` events. Register commands in `lib.rs`. Mock-based unit tests so the suite doesn't depend on Stockfish.

**Files:**
- Modify: `src-tauri/Cargo.toml` (add deps)
- Modify: `src-tauri/src/lib.rs` (manage state, register commands, RunEvent::Exit cleanup)
- Modify: `src-tauri/src/commands.rs` (add 2 commands + DTOs + tests)

- [ ] **Step 1: Add deps to `src-tauri/Cargo.toml`**

Replace the `[dependencies]` block:

```toml
[dependencies]
tauri = { workspace = true, features = [] }
tauri-plugin-dialog = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tokio = { workspace = true }
chess-core = { path = "crates/chess-core" }
chess-engine-api = { path = "crates/chess-engine-api" }
chess-engine-uci = { path = "crates/chess-engine-uci" }

[dev-dependencies]
chess-engine-api = { path = "crates/chess-engine-api", features = ["mock"] }
```

And in `chess-engine-api/Cargo.toml`, add a feature flag for the mock module (so it's available to downstream tests without `#[cfg(test)]` gating problems):

```toml
[features]
mock = []
```

Then in `chess-engine-api/src/lib.rs`, change `#[cfg(any(test, feature = "mock"))]` (already what we wrote).

- [ ] **Step 2: Add new DTOs and commands to `src-tauri/src/commands.rs`**

Append to `src-tauri/src/commands.rs` (before the existing `#[cfg(test)]` module):

```rust
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex as StdMutex;

use chess_engine_api::{AnalysisInfo as ApiAnalysisInfo, Engine, SearchLimits, Score};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

#[derive(Serialize, Clone, Debug)]
#[serde(tag = "kind", content = "value")]
pub enum ScoreDto {
    Cp(i32),
    Mate(i8),
}

impl From<Score> for ScoreDto {
    fn from(s: Score) -> Self {
        match s {
            Score::Cp(v) => ScoreDto::Cp(v),
            Score::Mate(v) => ScoreDto::Mate(v),
        }
    }
}

#[derive(Serialize, Clone, Debug)]
pub struct AnalysisInfoEvent {
    pub search_id: u64,
    pub depth: u8,
    pub score: ScoreDto,
    pub pv_san: Vec<String>,
    pub multipv_index: u8,
    pub nodes: u64,
    pub nps: u64,
    pub time_ms: u32,
}

#[derive(Serialize, Clone, Debug)]
pub struct EngineBestMoveEvent {
    pub search_id: u64,
    pub mv: MoveDto,
}

#[derive(Serialize, Clone, Debug)]
pub struct EngineErrorEvent {
    pub search_id: u64,
    pub message: String,
}

pub struct EngineManager {
    pub engine: StdMutex<Option<Box<dyn Engine + Send>>>,
    pub next_search_id: AtomicU64,
    pub current_search_id: AtomicU64,
    pub stockfish_path: PathBuf,
}

impl EngineManager {
    pub fn new(stockfish_path: PathBuf) -> Self {
        Self {
            engine: StdMutex::new(None),
            next_search_id: AtomicU64::new(1),
            current_search_id: AtomicU64::new(0),
            stockfish_path,
        }
    }
}

/// Lazily construct the Stockfish engine the first time it's needed.
/// Test setups can pre-populate `manager.engine` with a MockEngine before invoking commands.
fn ensure_stockfish(manager: &EngineManager) -> Result<(), String> {
    let mut guard = manager.engine.lock().map_err(|e| e.to_string())?;
    if guard.is_some() { return Ok(()); }
    let rt = tokio::runtime::Handle::try_current()
        .map_err(|_| "No tokio runtime available".to_string())?;
    let eng = chess_engine_uci::StockfishEngine::new(manager.stockfish_path.clone(), rt);
    *guard = Some(Box::new(eng));
    Ok(())
}

#[tauri::command]
pub async fn start_analysis<R: Runtime>(
    app: AppHandle<R>,
    fen: String,
    skill_level: u8,
    movetime_ms: u32,
    multipv: u8,
) -> Result<u64, String> {
    let pos = cc::parse_fen(&fen).map_err(|e| format!("Invalid FEN: {e:?}"))?;
    let manager = app.state::<EngineManager>();
    ensure_stockfish(&manager)?;

    let search_id = manager.next_search_id.fetch_add(1, Ordering::SeqCst);
    manager.current_search_id.store(search_id, Ordering::SeqCst);

    // Clone what the closures need.
    let app_clone = app.clone();
    let pos_clone = pos.clone();

    // The engine's analyze() blocks until bestmove arrives. Run it on a Tokio blocking
    // thread so the Tauri command returns the search_id immediately.
    tokio::task::spawn_blocking(move || {
        let manager = app_clone.state::<EngineManager>();
        let mut guard = match manager.engine.lock() {
            Ok(g) => g,
            Err(e) => { eprintln!("[start_analysis] engine mutex poisoned: {e}"); return; }
        };
        let engine = match guard.as_mut() {
            Some(e) => e,
            None => { eprintln!("[start_analysis] engine slot empty"); return; }
        };

        let pos_for_san = pos_clone.clone();
        let app_for_info = app_clone.clone();
        let app_for_best = app_clone.clone();

        engine.analyze(
            pos_clone,
            SearchLimits { movetime_ms, multipv },
            skill_level,
            Box::new(move |info: ApiAnalysisInfo| {
                let pv_san = chess_engine_uci::pv_to_san(&pos_for_san, &info.pv);
                let evt = AnalysisInfoEvent {
                    search_id,
                    depth: info.depth,
                    score: info.score.into(),
                    pv_san,
                    multipv_index: info.multipv_index,
                    nodes: info.nodes,
                    nps: info.nps,
                    time_ms: info.time_ms,
                };
                let _ = app_for_info.emit("analysis_info", evt);
            }),
            Box::new(move |m: cc::Move| {
                let evt = EngineBestMoveEvent { search_id, mv: move_to_dto(m) };
                let _ = app_for_best.emit("engine_bestmove", evt);
            }),
        );
        // engine mutex releases here when guard drops.
    });

    Ok(search_id)
}
```

**Note on `stop_analysis`:** Plan 3 deliberately does NOT expose a `stop_analysis` command. The frontend triggers new analyses via the `$effect` watching `gameStore.currentFen`; each new `start_analysis` call queues behind the previous (the engine mutex serializes), and its first UCI action is `stop` to preempt the prior search. Worst case: ~1 s delay between a fast user move and the new analysis starting (matching `movetime_ms`). Adding a real preempt-now path requires plumbing a stop signal that bypasses the mutex; deferred to a later plan.

- [ ] **Step 3: Register the EngineManager state and commands in `src-tauri/src/lib.rs`**

Replace `src-tauri/src/lib.rs`:

```rust
mod commands;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Resolve the bundled Stockfish path. In dev mode this resolves to
            // src-tauri/binaries/...exe relative to the workspace. In release mode it resolves
            // to the bundle's resource dir.
            let path = app
                .path()
                .resolve(
                    "binaries/stockfish-x86_64-pc-windows-msvc.exe",
                    tauri::path::BaseDirectory::Resource,
                )
                .unwrap_or_else(|_| {
                    // Dev fallback: relative to CARGO_MANIFEST_DIR (src-tauri/).
                    std::path::PathBuf::from("binaries/stockfish-x86_64-pc-windows-msvc.exe")
                });
            app.manage(commands::EngineManager::new(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::legal_moves,
            commands::make_move,
            commands::validate_fen,
            commands::parse_pgn,
            commands::serialize_pgn,
            commands::save_pgn_file,
            commands::load_pgn_file,
            commands::start_analysis,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| {
            if let RunEvent::Exit = event {
                // Drop the engine so its kill_on_drop Child terminates Stockfish.
                if let Some(mgr) = app_handle.try_state::<commands::EngineManager>() {
                    let _ = mgr.engine.lock().map(|mut g| *g = None);
                }
            }
        });
}
```

- [ ] **Step 4: Add a mock-based test for the command DTOs and search_id increment**

Append to the `#[cfg(test)]` module in `commands.rs`:

```rust
    #[test]
    fn score_dto_serializes_with_tag_kind_value() {
        let dto = ScoreDto::Cp(42);
        let j = serde_json::to_string(&dto).unwrap();
        assert_eq!(j, r#"{"kind":"Cp","value":42}"#);
    }

    #[test]
    fn engine_manager_search_id_increments() {
        let mgr = EngineManager::new("nope".into());
        let id1 = mgr.next_search_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let id2 = mgr.next_search_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        assert!(id2 > id1);
    }

    // Full end-to-end command tests would require Tauri's State runtime, which is fiddly to set up
    // in unit tests. The mock-based integration coverage lives in chess-engine-api (MockEngine tests)
    // and chess-engine-uci (real-Stockfish integration test). The Tauri layer is exercised by the
    // manual smoke test in Task 11.
```

- [ ] **Step 5: Run all backend tests**

Run: `cargo test -p chess-engine-api` (4 tests)
Run: `cargo test -p chess-engine-uci` (15 tests — integration is `#[ignore]`)
Run: `cargo test -p chess-engine-app` (9 existing + 2 new = 11)
Expected: all green.

- [ ] **Step 6: Verify the release build still works**

Run: `cargo check -p chess-engine-app`
Expected: exit 0.

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/Cargo.toml src-tauri/crates/chess-engine-api/Cargo.toml src-tauri/src/
git commit -m "feat: EngineManager Tauri state + start_analysis/stop_analysis commands"
```

---

### Task 6: `enginesStore` + `EnginePickers` UI

**Goal:** Svelte store representing which side is Human vs Stockfish + skill level. Two chip-style buttons in the Toolbar that open a popover for editing the slot.

**Files:**
- Create: `src/lib/stores/engines.svelte.ts`
- Create: `src/lib/stores/engines.test.ts`
- Create: `src/lib/panels/EnginePickers.svelte`
- Modify: `src/lib/panels/Toolbar.svelte` (mount `<EnginePickers />`)

- [ ] **Step 1: Write failing tests for `enginesStore`**

`src/lib/stores/engines.test.ts`:

```ts
import { describe, it, expect, beforeEach } from "vitest";
import { enginesStore } from "./engines.svelte.ts";

describe("enginesStore", () => {
  beforeEach(() => {
    enginesStore.white = { kind: "Human", skill: 10 };
    enginesStore.black = { kind: "Stockfish", skill: 10 };
  });

  it("default is white=Human, black=Stockfish(10)", () => {
    expect(enginesStore.white.kind).toBe("Human");
    expect(enginesStore.black.kind).toBe("Stockfish");
    expect(enginesStore.black.skill).toBe(10);
  });

  it("engineFor returns the slot iff Stockfish, else null", () => {
    expect(enginesStore.engineFor("w")).toBeNull();
    expect(enginesStore.engineFor("b")).toEqual({ kind: "Stockfish", skill: 10 });
  });

  it("anyEngine is true if either side is Stockfish", () => {
    expect(enginesStore.anyEngine).toBe(true);
    enginesStore.black = { kind: "Human", skill: 10 };
    expect(enginesStore.anyEngine).toBe(false);
    enginesStore.white = { kind: "Stockfish", skill: 5 };
    expect(enginesStore.anyEngine).toBe(true);
  });

  it("engineForSideToMove parses FEN side and returns the right slot", () => {
    const whitesTurn = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const blacksTurn = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
    expect(enginesStore.engineForSideToMove(whitesTurn)).toBeNull(); // white is Human
    expect(enginesStore.engineForSideToMove(blacksTurn)?.kind).toBe("Stockfish");
  });
});
```

- [ ] **Step 2: Run, verify failure**

Run: `npm test`
Expected: FAIL — `enginesStore` not defined.

- [ ] **Step 3: Implement `enginesStore`**

`src/lib/stores/engines.svelte.ts`:

```ts
export type EngineSlot =
  | { kind: "Human"; skill: number }
  | { kind: "Stockfish"; skill: number };

class EnginesStore {
  white: EngineSlot = $state({ kind: "Human", skill: 10 });
  black: EngineSlot = $state({ kind: "Stockfish", skill: 10 });

  engineFor(side: "w" | "b"): EngineSlot | null {
    const slot = side === "w" ? this.white : this.black;
    return slot.kind === "Stockfish" ? slot : null;
  }

  anyEngine: boolean = $derived(
    this.white.kind === "Stockfish" || this.black.kind === "Stockfish"
  );

  engineForSideToMove(fen: string): EngineSlot | null {
    const side = fen.split(" ")[1] as "w" | "b";
    return this.engineFor(side);
  }
}

export const enginesStore = new EnginesStore();
```

- [ ] **Step 4: Run tests, verify pass**

Run: `npm test`
Expected: 4 new tests pass (plus 14 existing from Plan 2 = 18 total).

- [ ] **Step 5: Create `EnginePickers.svelte`**

`src/lib/panels/EnginePickers.svelte`:

```svelte
<script lang="ts">
  import { enginesStore, type EngineSlot } from "../stores/engines.svelte.ts";

  let openSide: "w" | "b" | null = $state(null);

  function label(slot: EngineSlot): string {
    return slot.kind === "Human" ? "Human" : `Stockfish (${slot.skill})`;
  }

  function setKind(side: "w" | "b", kind: "Human" | "Stockfish") {
    const cur = side === "w" ? enginesStore.white : enginesStore.black;
    const next: EngineSlot = { kind, skill: cur.skill };
    if (side === "w") enginesStore.white = next; else enginesStore.black = next;
  }

  function setSkill(side: "w" | "b", skill: number) {
    const cur = side === "w" ? enginesStore.white : enginesStore.black;
    const next: EngineSlot = { kind: cur.kind, skill };
    if (side === "w") enginesStore.white = next; else enginesStore.black = next;
  }
</script>

<div class="pickers">
  {#each ["w", "b"] as const as side}
    {@const slot = side === "w" ? enginesStore.white : enginesStore.black}
    <div class="chip-wrap">
      <button
        class="chip"
        onclick={() => (openSide = openSide === side ? null : side)}
      >
        {side === "w" ? "⚪" : "⚫"} {label(slot)}
      </button>
      {#if openSide === side}
        <div class="popover" role="dialog">
          <label>
            <input
              type="radio"
              name="kind-{side}"
              checked={slot.kind === "Human"}
              onchange={() => setKind(side, "Human")}
            />
            Human
          </label>
          <label>
            <input
              type="radio"
              name="kind-{side}"
              checked={slot.kind === "Stockfish"}
              onchange={() => setKind(side, "Stockfish")}
            />
            Stockfish
          </label>
          <div class="skill-row">
            Skill
            <input
              type="range"
              min="0"
              max="20"
              value={slot.skill}
              disabled={slot.kind === "Human"}
              oninput={(e) => setSkill(side, +(e.currentTarget as HTMLInputElement).value)}
            />
            <span class="skill-val">{slot.skill}</span>
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .pickers { display: flex; gap: 6px; }
  .chip-wrap { position: relative; }
  .chip {
    background: #2e3d4a;
    border: 1px solid #4a90d9;
    color: #eee;
    padding: 4px 10px;
    border-radius: 12px;
    cursor: pointer;
    font-size: 12px;
  }
  .chip:hover { background: #3a4a5a; }
  .popover {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: #1f1f1f;
    border: 1px solid #555;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 30;
    min-width: 200px;
    color: #ddd;
    font-size: 12px;
  }
  .popover label { display: flex; align-items: center; gap: 6px; cursor: pointer; }
  .skill-row { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .skill-row input[type="range"] { flex: 1; }
  .skill-val { width: 24px; text-align: right; color: #fff; }
</style>
```

- [ ] **Step 6: Mount in Toolbar**

In `src/lib/panels/Toolbar.svelte`, replace the `<div class="spacer">` line with the picker mount.

Find the existing template block:

```svelte
<button onclick={loadFen} disabled={!fenValid || fenChecking}>Load</button>
<div class="spacer"></div>
<button onclick={() => gameStore.undo()} disabled={!canUndo}>← Undo</button>
```

Replace with:

```svelte
<button onclick={loadFen} disabled={!fenValid || fenChecking}>Load</button>
<div class="spacer"></div>
<EnginePickers />
<div class="spacer-narrow"></div>
<button onclick={() => gameStore.undo()} disabled={!canUndo}>← Undo</button>
```

Add to the script block at the top:

```ts
import EnginePickers from "./EnginePickers.svelte";
```

Add to the style block:

```css
.spacer-narrow { width: 12px; }
```

- [ ] **Step 7: Run all tests**

Run: `npm test`
Expected: 18 tests pass.

Run: `npm run build`
Expected: exit 0 (may show pre-existing Board.svelte a11y warnings; ignore).

- [ ] **Step 8: Commit**

```powershell
git add src/
git commit -m "feat(ui): enginesStore + EnginePickers chips in Toolbar"
```

---

### Task 7: `analysisStore` + event listeners + auto-trigger `$effect`

**Goal:** New store with `searchId`, `lines`, derived `evalPercent`. Wire Tauri event listeners at app mount. Add `$effect` to App.svelte that calls `startAnalysis` whenever `currentFen` changes (gated by `anyEngine`). Frontend doesn't render analysis UI yet (next two tasks); console-verify the events flow.

**Files:**
- Create: `src/lib/stores/analysis.svelte.ts`
- Create: `src/lib/stores/analysis.test.ts`
- Modify: `src/lib/tauri.ts` (add engine commands + event types)
- Modify: `src/App.svelte` (mount listeners + effect)

- [ ] **Step 1: Add typed wrappers and event types to `src/lib/tauri.ts`**

Append to `src/lib/tauri.ts`:

```ts
export interface ScoreDto {
  kind: "Cp" | "Mate";
  value: number;
}

export interface AnalysisInfoEvent {
  search_id: number;
  depth: number;
  score: ScoreDto;
  pv_san: string[];
  multipv_index: number;
  nodes: number;
  nps: number;
  time_ms: number;
}

export interface EngineBestMoveEvent {
  search_id: number;
  mv: MoveDto;
}
```

Then extend the `tauri` object (alongside the existing methods) with:

```ts
  startAnalysis(fen: string, skill_level: number, movetime_ms: number, multipv: number): Promise<number> {
    return invoke("start_analysis", { fen, skillLevel: skill_level, movetimeMs: movetime_ms, multipv });
  },
```

Note: Tauri 2 expects camelCase keys in invoke args (it converts to snake_case Rust args automatically). Above uses the camelCase names.

- [ ] **Step 2: Write failing tests for `analysisStore`**

`src/lib/stores/analysis.test.ts`:

```ts
import { describe, it, expect, beforeEach } from "vitest";
import { analysisStore } from "./analysis.svelte.ts";

describe("analysisStore", () => {
  beforeEach(() => {
    analysisStore.reset(0);
    analysisStore.searchId = null;
  });

  it("ignores info with stale searchId", () => {
    analysisStore.reset(5);
    analysisStore.applyInfo({
      search_id: 4,
      depth: 1, score: { kind: "Cp", value: 0 }, pv_san: ["e4"],
      multipv_index: 1, nodes: 1, nps: 1, time_ms: 1,
    });
    expect(analysisStore.lines.size).toBe(0);
  });

  it("accepts info with current searchId, keyed by multipv_index", () => {
    analysisStore.reset(7);
    analysisStore.applyInfo({
      search_id: 7,
      depth: 12, score: { kind: "Cp", value: 25 }, pv_san: ["e4", "e5"],
      multipv_index: 1, nodes: 1000, nps: 5000, time_ms: 200,
    });
    analysisStore.applyInfo({
      search_id: 7,
      depth: 12, score: { kind: "Cp", value: 12 }, pv_san: ["d4", "d5"],
      multipv_index: 2, nodes: 1000, nps: 5000, time_ms: 200,
    });
    expect(analysisStore.lines.size).toBe(2);
    expect(analysisStore.lines.get(1)?.pv_san[0]).toBe("e4");
    expect(analysisStore.lines.get(2)?.pv_san[0]).toBe("d4");
  });

  it("evalPercent maps cp=0 to 0.5", () => {
    analysisStore.reset(1);
    analysisStore.applyInfo({
      search_id: 1, depth: 1, score: { kind: "Cp", value: 0 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    expect(analysisStore.evalPercent).toBeCloseTo(0.5, 2);
  });

  it("evalPercent maps cp=+400 to ~0.73 (Lichess sigmoid)", () => {
    analysisStore.reset(2);
    analysisStore.applyInfo({
      search_id: 2, depth: 1, score: { kind: "Cp", value: 400 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    // 2/(1+exp(-1)) - 1 = ~0.4621 → mapped to [0,1]: (0.4621+1)/2 = ~0.731
    expect(analysisStore.evalPercent).toBeCloseTo(0.731, 2);
  });

  it("evalPercent maps Mate(+N) to 1.0 and Mate(-N) to 0.0", () => {
    analysisStore.reset(3);
    analysisStore.applyInfo({
      search_id: 3, depth: 1, score: { kind: "Mate", value: 3 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    expect(analysisStore.evalPercent).toBeCloseTo(1.0, 4);

    analysisStore.reset(4);
    analysisStore.applyInfo({
      search_id: 4, depth: 1, score: { kind: "Mate", value: -2 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    expect(analysisStore.evalPercent).toBeCloseTo(0.0, 4);
  });

  it("evalPercent is 0.5 when there's no analysis yet", () => {
    expect(analysisStore.evalPercent).toBeCloseTo(0.5, 4);
  });
});
```

- [ ] **Step 3: Run, verify failure**

Run: `npm test`
Expected: FAIL — `analysisStore` not defined.

- [ ] **Step 4: Implement `analysisStore`**

`src/lib/stores/analysis.svelte.ts`:

```ts
import type { AnalysisInfoEvent } from "../tauri.ts";

class AnalysisStore {
  searchId: number | null = $state(null);
  lines: Map<number, AnalysisInfoEvent> = $state(new Map());

  evalPercent: number = $derived.by(() => {
    const best = this.lines.get(1);
    if (!best) return 0.5;
    if (best.score.kind === "Mate") {
      return best.score.value > 0 ? 1.0 : 0.0;
    }
    // Lichess-style sigmoid: 2/(1+exp(-cp/400)) - 1 in (-1, 1).
    // Score is from side-to-move's POV; UI convention: positive = White advantage.
    // The store doesn't know side-to-move on its own — callers should pass cp already
    // flipped from White's perspective. For Plan 3 we accept the side-to-move convention
    // and the EvalBar inverts the bar when it's black to move.
    const cp = best.score.value;
    const v = 2 / (1 + Math.exp(-cp / 400)) - 1; // (-1, 1)
    return (v + 1) / 2; // (0, 1)
  });

  applyInfo(info: AnalysisInfoEvent): void {
    if (info.search_id !== this.searchId) return;
    const next = new Map(this.lines);
    next.set(info.multipv_index, info);
    this.lines = next;
  }

  reset(newSearchId: number): void {
    this.searchId = newSearchId;
    this.lines = new Map();
  }
}

export const analysisStore = new AnalysisStore();
```

- [ ] **Step 5: Run tests, verify pass**

Run: `npm test`
Expected: 24 tests pass (18 previous + 6 new analysisStore).

- [ ] **Step 6: Wire Tauri event listeners + $effect into `src/App.svelte`**

Replace `src/App.svelte`:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import Board from "./lib/board/Board.svelte";
  import HistoryPanel from "./lib/panels/HistoryPanel.svelte";
  import Toolbar from "./lib/panels/Toolbar.svelte";
  import GameOverBanner from "./lib/panels/GameOverBanner.svelte";
  import { gameStore } from "./lib/stores/game.svelte.ts";
  import { enginesStore } from "./lib/stores/engines.svelte.ts";
  import { analysisStore } from "./lib/stores/analysis.svelte.ts";
  import { tauri, type AnalysisInfoEvent, type EngineBestMoveEvent } from "./lib/tauri.ts";

  let unlistenInfo: UnlistenFn | null = null;
  let unlistenBest: UnlistenFn | null = null;

  onMount(async () => {
    unlistenInfo = await listen<AnalysisInfoEvent>("analysis_info", (e) => {
      analysisStore.applyInfo(e.payload);
    });
    unlistenBest = await listen<EngineBestMoveEvent>("engine_bestmove", (e) => {
      // Auto-play wiring lands in Task 10. For now just console-log.
      console.log("[engine_bestmove]", e.payload);
    });
  });

  onDestroy(() => {
    unlistenInfo?.();
    unlistenBest?.();
  });

  // Reactive analysis trigger: any FEN change, if any engine is active, kick off a new search.
  $effect(() => {
    const fen = gameStore.currentFen;
    if (!enginesStore.anyEngine) return;
    const skill =
      enginesStore.white.kind === "Stockfish" ? enginesStore.white.skill : enginesStore.black.skill;
    tauri
      .startAnalysis(fen, skill, 1000, 3)
      .then((id) => analysisStore.reset(id))
      .catch((err) => console.error("[startAnalysis]", err));
  });
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

- [ ] **Step 7: Verify build**

Run: `npm test` — 24 pass.
Run: `npm run build` — exit 0.

- [ ] **Step 8: Commit**

```powershell
git add src/
git commit -m "feat(ui): analysisStore + Tauri event listeners + auto-trigger analysis on FEN change"
```

---

### Task 8: `EvalBar` component

**Goal:** Vertical bar to the left of the board. Reads `analysisStore.evalPercent`. White-on-top fill animates between values.

**Files:**
- Create: `src/lib/panels/EvalBar.svelte`
- Modify: `src/App.svelte` (add a left grid column for the eval bar)

- [ ] **Step 1: Create `EvalBar.svelte`**

`src/lib/panels/EvalBar.svelte`:

```svelte
<script lang="ts">
  import { analysisStore } from "../stores/analysis.svelte.ts";

  const whitePct = $derived(Math.max(0, Math.min(1, analysisStore.evalPercent)) * 100);

  const label = $derived.by(() => {
    const best = analysisStore.lines.get(1);
    if (!best) return "";
    if (best.score.kind === "Mate") return `M${Math.abs(best.score.value)}`;
    const cp = best.score.value / 100;
    return cp >= 0 ? `+${cp.toFixed(2)}` : cp.toFixed(2);
  });
</script>

<aside class="evalbar" aria-label="Evaluation bar">
  <div class="white-fill" style:height="{whitePct}%"></div>
  <span class="label">{label}</span>
</aside>

<style>
  .evalbar {
    width: 24px;
    height: 100%;
    background: #222;            /* black side fills the rest by default */
    border-right: 1px solid #444;
    position: relative;
    overflow: hidden;
  }
  .white-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: #fff;
    transition: height 200ms ease-out;
  }
  .label {
    position: absolute;
    left: 50%;
    bottom: 4px;
    transform: translateX(-50%);
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 10px;
    color: #444;
    background: rgba(255, 255, 255, 0.85);
    padding: 1px 4px;
    border-radius: 2px;
    pointer-events: none;
    z-index: 1;
  }
</style>
```

- [ ] **Step 2: Add a left grid column for the EvalBar in `src/App.svelte`**

Modify `src/App.svelte`. In the `<script>` block, add:

```ts
import EvalBar from "./lib/panels/EvalBar.svelte";
```

Restructure the template:

```svelte
<div class="app">
  <header class="toolbar">
    <Toolbar />
  </header>
  <EvalBar />
  <section class="board-area">
    <Board />
  </section>
  <HistoryPanel />
</div>
<GameOverBanner />
```

Update the grid in the `<style>` block:

```css
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    grid-template-columns: auto 1fr 280px;   /* eval bar + board area + sidebar */
    height: 100vh;
  }
  .toolbar {
    grid-column: 1 / span 3;
    background: #1f1f1f;
    border-bottom: 1px solid #444;
    height: 48px;
  }
```

- [ ] **Step 3: Verify build**

Run: `npm run build`
Expected: exit 0.

Run: `npm test`
Expected: 24 pass.

- [ ] **Step 4: Commit**

```powershell
git add src/
git commit -m "feat(ui): EvalBar component left of board"
```

---

### Task 9: `AnalysisPanel` component

**Goal:** Table below `HistoryPanel` showing top-3 candidate moves with score, depth, PV preview.

**Files:**
- Create: `src/lib/panels/AnalysisPanel.svelte`
- Modify: `src/App.svelte` (add AnalysisPanel below HistoryPanel via a grid sub-layout)

- [ ] **Step 1: Create `AnalysisPanel.svelte`**

`src/lib/panels/AnalysisPanel.svelte`:

```svelte
<script lang="ts">
  import { analysisStore } from "../stores/analysis.svelte.ts";

  function formatScore(score: { kind: "Cp" | "Mate"; value: number }): string {
    if (score.kind === "Mate") {
      return `M${Math.abs(score.value)}${score.value < 0 ? " (lost)" : ""}`;
    }
    const cp = score.value / 100;
    return cp >= 0 ? `+${cp.toFixed(2)}` : cp.toFixed(2);
  }

  const sorted = $derived.by(() => {
    return Array.from(analysisStore.lines.values()).sort(
      (a, b) => a.multipv_index - b.multipv_index
    );
  });
</script>

<aside class="analysis">
  <h2>Analysis</h2>
  {#if sorted.length === 0}
    <p class="empty">No analysis yet.</p>
  {:else}
    <ol>
      {#each sorted as info}
        <li>
          <div class="row-1">
            <span class="san">{info.pv_san[0] ?? "—"}</span>
            <span class="score">{formatScore(info.score)}</span>
            <span class="depth">d{info.depth}</span>
          </div>
          {#if info.pv_san.length > 1}
            <div class="pv">{info.pv_san.slice(0, 5).join(" ")}{info.pv_san.length > 5 ? "…" : ""}</div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</aside>

<style>
  .analysis {
    background: #1a1a1a;
    color: #e8e8e8;
    padding: 12px;
    border-top: 1px solid #333;
    font-family: ui-monospace, "Cascadia Code", monospace;
    overflow-y: auto;
    min-height: 120px;
  }
  .analysis h2 {
    margin: 0 0 8px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #bbb;
  }
  .empty { color: #888; font-style: italic; font-size: 12px; }
  ol { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .row-1 {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 8px;
    align-items: baseline;
    font-size: 13px;
  }
  .san { color: #fff; font-weight: 500; }
  .score { color: #4caf50; font-variant-numeric: tabular-nums; }
  .depth { color: #777; font-size: 10px; }
  .pv { color: #999; font-size: 11px; padding-left: 8px; }
</style>
```

- [ ] **Step 2: Modify `src/App.svelte` to stack HistoryPanel + AnalysisPanel in the sidebar**

In the `<script>` block, add:

```ts
import AnalysisPanel from "./lib/panels/AnalysisPanel.svelte";
```

Change the template's right-sidebar slot from just `<HistoryPanel />` to a stacked sub-layout. Replace:

```svelte
<HistoryPanel />
```

with:

```svelte
<aside class="sidebar">
  <HistoryPanel />
  <AnalysisPanel />
</aside>
```

Update the styles — the sidebar now needs internal stacking:

```css
  .sidebar {
    display: grid;
    grid-template-rows: 1fr auto;   /* History flexes, Analysis hugs content */
    min-height: 0;                  /* allow History to scroll */
    background: #1a1a1a;
  }
```

You may need to give the HistoryPanel a `min-height: 0` and an explicit overflow to allow the grid child to shrink — verify visually after build.

- [ ] **Step 3: Verify build**

Run: `npm test` — 24 pass.
Run: `npm run build` — exit 0.

- [ ] **Step 4: Commit**

```powershell
git add src/
git commit -m "feat(ui): AnalysisPanel below HistoryPanel"
```

---

### Task 10: Auto-play wiring (engine_bestmove handler)

**Goal:** Replace the `console.log` in the `engine_bestmove` listener with logic that calls `gameStore.makeMove` when the side-to-move is Stockfish. Add a Vitest test for the dispatch decision logic.

**Files:**
- Modify: `src/App.svelte` (replace listener body)
- Create: `src/lib/stores/auto-play.test.ts` (decision-logic test)
- Optionally extract the decision into a pure helper for testability.

- [ ] **Step 1: Extract the "should engine play this?" decision into a pure helper**

Create `src/lib/stores/auto-play.ts`:

```ts
import type { EngineSlot } from "./engines.svelte.ts";

/**
 * Given the current FEN and the engine slot mapping, returns true iff the side-to-move
 * is a Stockfish engine and therefore the bestmove event should be auto-played.
 */
export function shouldEnginePlay(
  fen: string,
  white: EngineSlot,
  black: EngineSlot,
): boolean {
  const side = fen.split(" ")[1];
  if (side !== "w" && side !== "b") return false;
  const slot = side === "w" ? white : black;
  return slot.kind === "Stockfish";
}
```

- [ ] **Step 2: Write tests for the helper**

`src/lib/stores/auto-play.test.ts`:

```ts
import { describe, it, expect } from "vitest";
import { shouldEnginePlay } from "./auto-play.ts";

const WHITES_TURN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const BLACKS_TURN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
const HUMAN = { kind: "Human", skill: 10 } as const;
const SF = { kind: "Stockfish", skill: 10 } as const;

describe("shouldEnginePlay", () => {
  it("returns true when side-to-move is Stockfish", () => {
    expect(shouldEnginePlay(WHITES_TURN, SF, HUMAN)).toBe(true);
    expect(shouldEnginePlay(BLACKS_TURN, HUMAN, SF)).toBe(true);
  });

  it("returns false when side-to-move is Human", () => {
    expect(shouldEnginePlay(WHITES_TURN, HUMAN, SF)).toBe(false);
    expect(shouldEnginePlay(BLACKS_TURN, SF, HUMAN)).toBe(false);
  });

  it("returns false when both sides are Human", () => {
    expect(shouldEnginePlay(WHITES_TURN, HUMAN, HUMAN)).toBe(false);
    expect(shouldEnginePlay(BLACKS_TURN, HUMAN, HUMAN)).toBe(false);
  });

  it("returns true on both turns when both sides are Stockfish (engine-vs-engine future)", () => {
    expect(shouldEnginePlay(WHITES_TURN, SF, SF)).toBe(true);
    expect(shouldEnginePlay(BLACKS_TURN, SF, SF)).toBe(true);
  });

  it("returns false for malformed FEN", () => {
    expect(shouldEnginePlay("garbage", SF, SF)).toBe(false);
  });
});
```

- [ ] **Step 3: Run tests**

Run: `npm test`
Expected: 29 pass (24 + 5).

- [ ] **Step 4: Wire the bestmove listener in `App.svelte`**

In `src/App.svelte`'s `<script>`, add the import:

```ts
import { shouldEnginePlay } from "./lib/stores/auto-play.ts";
```

Replace the `engine_bestmove` listener body:

```ts
    unlistenBest = await listen<EngineBestMoveEvent>("engine_bestmove", async (e) => {
      const payload = e.payload;
      if (payload.search_id !== analysisStore.searchId) return; // stale
      if (!shouldEnginePlay(gameStore.currentFen, enginesStore.white, enginesStore.black)) return;
      try {
        await gameStore.makeMove(payload.mv);
      } catch (err) {
        console.error("[engine_bestmove] makeMove failed", err);
      }
    });
```

- [ ] **Step 5: Verify all tests + build**

Run: `npm test` — 29 pass.
Run: `npm run build` — exit 0.

- [ ] **Step 6: Commit**

```powershell
git add src/
git commit -m "feat(ui): auto-play engine bestmove via gameStore.makeMove"
```

---

### Task 11: Polish + crash handling + smoke test

**Goal:** Stockfish-died restart logic in `EngineManager`. Cleanup on `RunEvent::Exit`. README update. Release build. Manual smoke checklist.

**Files:**
- Modify: `src-tauri/src/lib.rs` (RunEvent::Exit cleanup — already added in Task 5; verify)
- Modify: `src-tauri/src/commands.rs` (restart-on-dead logic in `ensure_stockfish`)
- Modify: `README.md`

- [ ] **Step 1: Stockfish-restart on death**

In `src-tauri/src/commands.rs`, modify `ensure_stockfish` to detect when the engine slot exists but the underlying Stockfish process has died (the next analyze call would fail with a broken-pipe error). For Plan 3, the simplest mitigation: if `start_analysis` fails internally (no info events arrive within 2 seconds), the EngineManager force-resets the engine slot to `None`, and the next `start_analysis` call re-spawns.

This is hard to detect from outside the engine. Pragmatic compromise: add a `reset` Tauri command that the frontend can call when it suspects a stuck engine.

Add to `commands.rs`:

```rust
#[tauri::command]
pub async fn reset_engine(manager: State<'_, EngineManager>) -> Result<(), String> {
    let mut guard = manager.engine.lock().map_err(|e| e.to_string())?;
    *guard = None; // Dropping the StockfishEngine kills the child process.
    Ok(())
}
```

Register in `lib.rs` `generate_handler!` list alongside the other engine commands:

```rust
commands::reset_engine,
```

Add the typed wrapper in `src/lib/tauri.ts`:

```ts
  resetEngine(): Promise<void> {
    return invoke("reset_engine");
  },
```

(Future: add an `engine_error` Tauri event and have the frontend auto-call resetEngine on receipt. For Plan 3, just expose the manual reset.)

- [ ] **Step 2: Verify RunEvent::Exit cleanup is in place**

In `src-tauri/src/lib.rs`, confirm the `.run(|app_handle, event| ...)` closure handles `RunEvent::Exit` by clearing `manager.engine` (added in Task 5 Step 3). If not, add it.

- [ ] **Step 3: Update root `README.md`**

Replace `README.md`:

```markdown
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

The app ships with Stockfish 17 (CC0-licensed) at `src-tauri/binaries/`. It's launched as a child process and communicated with via UCI over stdin/stdout. Skill Level 0 plays around 1100 Elo; Skill Level 20 is full strength.

## Limitations (Plan 3)

- Pawn promotion auto-defaults to Queen (no picker UI yet).
- History sidebar is read-only — clickable navigation to past positions is a planned follow-up.
- No DIY engine yet — Plan 4+.
- No engine-vs-engine mode yet — Plan 4+.
- No "hints off" / training mode toggle yet.

## Attribution

- Chess piece graphics: Cburnett (Wikimedia, CC-BY-SA 3.0). See `ATTRIBUTION.md`.
- Stockfish 17: GPL-3.0, https://stockfishchess.org/
```

- [ ] **Step 4: Run all automated tests one more time**

```powershell
cargo test -p chess-core
cargo test -p chess-engine-api
cargo test -p chess-engine-uci
cargo test --include-ignored -p chess-engine-uci
cargo test -p chess-engine-app
npm test
```

All should pass.

- [ ] **Step 5: Build the release binary**

Run: `npm run tauri build`
Expected: exit 0. The `.msi` should be ~30 MB (Plan 2 baseline ~3 MB + Stockfish ~30 MB).

- [ ] **Step 6: SKIP the visual smoke checklist** (the implementer can't see the GUI)

Document in the report that the smoke checklist needs the user to run `npm run tauri dev` and tick through:
1. App opens, default pickers visible (`⚪ Human`, `⚫ Stockfish (10)`)
2. Play 1. e4 → eval bar shifts within ~100 ms; analysis panel shows top-3 Black responses; Stockfish auto-plays within ~1.5 s
3. Switch Black to Human in the picker → engine stops moving; analysis stays on
4. Switch BOTH to Human → analysis goes blank
5. Switch back to Stockfish mid-game → engine picks up at current position
6. Set up Fool's Mate position via FEN → game-over banner shows (Plan 2 path still works)
7. Undo a move while Stockfish is mid-think → search restarts cleanly
8. Close the app → Stockfish process exits cleanly (verify Task Manager, no orphan)

- [ ] **Step 7: Commit**

```powershell
git add src-tauri/src/ src/ README.md
git commit -m "polish: reset_engine command + README update; smoke checklist deferred to user"
```

---

## Done state

When this plan completes:

- `npm run tauri build` produces an installer (~30 MB) bundling Stockfish 17
- All unit tests green:
  - chess-core: 44 unit + 6 perft
  - chess-engine-api: 4
  - chess-engine-uci: 15 parser (+ 1 integration `#[ignore]`-flagged)
  - chess-engine-app: 11 (existing 9 + 2 new for Score/EngineManager)
  - npm: 29 vitest (existing 14 + engines 4 + analysis 6 + auto-play 5)
- `cargo test --include-ignored -p chess-engine-uci` green (real Stockfish integration test passes)
- Manual smoke checklist passes (USER VERIFIES)
- You can play either side vs Stockfish with adjustable skill level, see eval bar + top-3 candidate moves stream live, switch engines mid-game cleanly, undo without orphaning Stockfish
- 11 task commits on `plan-3-stockfish-engine` branch beyond the spec commit (`69837e6`)

## Branch state and next steps

This plan is implemented on the existing `plan-3-stockfish-engine` branch (already branched off `main` after Plans 1+2 merged). When ready, open PR #3 against `main`.

Plan 4 begins the DIY engine: scaffold `chess-engine-diy` crate with a layered build (perft already from Plan 1 → material-only minimax → alpha-beta + iterative deepening).
