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
