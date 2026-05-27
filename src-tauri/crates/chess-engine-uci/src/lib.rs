//! Stockfish UCI subprocess adapter.

pub mod parser;

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use chess_core::prelude::{
    legal_moves, make_move, move_to_san, Move, MoveFlag, PieceKind, Position,
};
use chess_engine_api::{AnalysisInfo, Engine, SearchLimits};
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
