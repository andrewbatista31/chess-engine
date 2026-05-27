use chess_core::prelude as cc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MoveDto {
    pub from: String,
    pub to: String,
    pub promotion: Option<char>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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
    pub mv: MoveDto,
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

#[tauri::command]
pub fn validate_fen(fen: String) -> bool {
    cc::parse_fen(&fen).is_ok()
}

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
        entries.push(MoveEntry { san, fen_after, outcome, mv: move_to_dto(*m) });
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

#[tauri::command]
pub fn save_pgn_file(path: String, text: String) -> Result<(), String> {
    fs::write(&path, text).map_err(|e| format!("Could not save: {e}"))
}

#[tauri::command]
pub fn load_pgn_file(path: String) -> Result<String, String> {
    fs::read_to_string(&path).map_err(|e| format!("Could not read: {e}"))
}

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex as StdMutex;

use chess_engine_api::{AnalysisInfo as ApiAnalysisInfo, Engine, SearchLimits, Score};
use tauri::{AppHandle, Emitter, Manager, Runtime};

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
                // Convert Stockfish's side-to-move-POV score to White-POV for the UI.
                let score_white_pov = if pos_for_san.side_to_move == cc::Color::Black {
                    match info.score {
                        Score::Cp(v) => Score::Cp(-v),
                        Score::Mate(v) => Score::Mate(-v),
                    }
                } else {
                    info.score
                };
                let pv_san = chess_engine_uci::pv_to_san(&pos_for_san, &info.pv);
                let evt = AnalysisInfoEvent {
                    search_id,
                    depth: info.depth,
                    score: score_white_pov.into(),
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

#[tauri::command]
pub async fn reset_engine(manager: tauri::State<'_, EngineManager>) -> Result<(), String> {
    let mut guard = manager.engine.lock().map_err(|e| e.to_string())?;
    *guard = None; // Dropping the StockfishEngine kills the child process.
    Ok(())
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

    #[test]
    fn validate_fen_accepts_valid() {
        assert!(validate_fen(START.into()));
    }
    #[test]
    fn validate_fen_rejects_invalid() {
        assert!(!validate_fen("not a fen".into()));
        assert!(!validate_fen("8/8/8/8/8/8/8 w - - 0 1".into())); // only 7 ranks
    }

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
}
