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
}
