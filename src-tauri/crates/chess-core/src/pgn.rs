use std::collections::HashMap;

use crate::make_move::make_move;
use crate::moves::{Move, MoveFlag};
use crate::movegen::legal_moves;
use crate::position::Position;
use crate::types::{File, PieceKind, Rank, Square};

#[derive(Debug)]
pub struct Game {
    pub tags: HashMap<String, String>,
    pub moves: Vec<Move>,
    pub result: String,
    pub final_position: Position,
}

#[derive(Debug)]
pub enum PgnError {
    Malformed,
    IllegalSan(String),
    AmbiguousSan(String),
}

pub fn parse_pgn(input: &str) -> Result<Game, PgnError> {
    let mut tags = HashMap::new();
    let mut body = String::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() { continue; }
        if let Some(rest) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            // [Name "Value"]
            let mut parts = rest.splitn(2, ' ');
            let name = parts.next().ok_or(PgnError::Malformed)?.to_string();
            let value = parts.next().ok_or(PgnError::Malformed)?;
            let value = value.trim_matches('"');
            tags.insert(name, value.to_string());
        } else {
            body.push_str(line);
            body.push(' ');
        }
    }

    let mut pos = Position::starting();
    let mut moves = vec![];
    let mut result = "*".to_string();

    for token in body.split_ascii_whitespace() {
        // Skip move numbers like "1." or "12...".
        if token.chars().next().map_or(false, |c| c.is_ascii_digit())
            && token.contains('.') {
            continue;
        }
        if matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*") {
            result = token.to_string();
            break;
        }
        let m = parse_san(&pos, token)?;
        make_move(&mut pos, m);
        moves.push(m);
    }

    Ok(Game { tags, moves, result, final_position: pos })
}

fn parse_san(pos: &Position, san: &str) -> Result<Move, PgnError> {
    // Strip + and # and ! ? annotations.
    let san = san.trim_end_matches(|c: char| matches!(c, '+' | '#' | '!' | '?'));

    // Castling.
    if san == "O-O" || san == "0-0" {
        return find_move(pos, |m| m.flag() == MoveFlag::KingCastle, san);
    }
    if san == "O-O-O" || san == "0-0-0" {
        return find_move(pos, |m| m.flag() == MoveFlag::QueenCastle, san);
    }

    // Parse promotion suffix: e.g. "e8=Q" or "exd8=Q".
    let (san_core, promo) = if let Some(idx) = san.rfind('=') {
        let promo_char = san.as_bytes().get(idx + 1).copied().ok_or(PgnError::Malformed)?;
        (&san[..idx], Some(promo_char))
    } else { (san, None) };

    let bytes = san_core.as_bytes();
    if bytes.is_empty() { return Err(PgnError::Malformed); }

    // Determine piece (capital letter at start except for pawn).
    let (piece_kind, mut i) = match bytes[0] {
        b'N' => (PieceKind::Knight, 1),
        b'B' => (PieceKind::Bishop, 1),
        b'R' => (PieceKind::Rook,   1),
        b'Q' => (PieceKind::Queen,  1),
        b'K' => (PieceKind::King,   1),
        b'a'..=b'h' => (PieceKind::Pawn, 0),
        _ => return Err(PgnError::Malformed),
    };

    // Optional disambiguation: file letter, rank digit, or both. Capture marker 'x'.
    let mut disamb_file: Option<u8> = None;
    let mut disamb_rank: Option<u8> = None;
    let mut _is_capture = false;

    // The destination is the LAST 2 chars (file letter + rank digit).
    if bytes.len() - i < 2 { return Err(PgnError::Malformed); }
    let dest_start = bytes.len() - 2;
    while i < dest_start {
        match bytes[i] {
            b'x' => { _is_capture = true; }
            f @ b'a'..=b'h' => { disamb_file = Some(f - b'a'); }
            r @ b'1'..=b'8' => { disamb_rank = Some(r - b'1'); }
            _ => return Err(PgnError::Malformed),
        }
        i += 1;
    }
    let to = parse_sq_bytes(&bytes[dest_start..]).ok_or(PgnError::Malformed)?;

    // Find a legal move matching criteria.
    find_move(pos, |m| {
        if m.to() != to { return false; }
        if !matches_piece(pos, m, piece_kind) { return false; }
        if let Some(df) = disamb_file { if m.from().file() as u8 != df { return false; } }
        if let Some(dr) = disamb_rank { if m.from().rank() as u8 != dr { return false; } }
        if let Some(p) = promo {
            let want = match p { b'N' => PieceKind::Knight, b'B' => PieceKind::Bishop,
                                  b'R' => PieceKind::Rook,   b'Q' => PieceKind::Queen,
                                  _ => return false };
            return promo_kind(m.flag()) == Some(want);
        } else if m.flag().is_promotion() {
            return false;
        }
        true
    }, san)
}

fn matches_piece(pos: &Position, m: Move, kind: PieceKind) -> bool {
    pos.piece_at(m.from()).map_or(false, |p| p.kind == kind)
}

fn promo_kind(flag: MoveFlag) -> Option<PieceKind> {
    Some(match flag {
        MoveFlag::PromoKnight | MoveFlag::PromoCaptureN => PieceKind::Knight,
        MoveFlag::PromoBishop | MoveFlag::PromoCaptureB => PieceKind::Bishop,
        MoveFlag::PromoRook   | MoveFlag::PromoCaptureR => PieceKind::Rook,
        MoveFlag::PromoQueen  | MoveFlag::PromoCaptureQ => PieceKind::Queen,
        _ => return None,
    })
}

fn find_move<F: Fn(Move) -> bool>(pos: &Position, predicate: F, san: &str) -> Result<Move, PgnError> {
    let candidates: Vec<Move> = legal_moves(pos).into_iter().filter(|m| predicate(*m)).collect();
    match candidates.len() {
        0 => Err(PgnError::IllegalSan(san.to_string())),
        1 => Ok(candidates[0]),
        _ => Err(PgnError::AmbiguousSan(san.to_string())),
    }
}

fn parse_sq_bytes(bytes: &[u8]) -> Option<Square> {
    if bytes.len() != 2 { return None; }
    let f = File::from_index(bytes[0].wrapping_sub(b'a'))?;
    let r = Rank::from_index(bytes[1].wrapping_sub(b'1'))?;
    Some(Square::new(f, r))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHOLARS_MATE: &str = r#"[Event "Test"]
[Site "?"]
[Date "2026.05.20"]
[White "A"]
[Black "B"]
[Result "1-0"]

1. e4 e5 2. Bc4 Nc6 3. Qh5 Nf6?? 4. Qxf7# 1-0
"#;

    #[test]
    fn parses_scholars_mate_to_completion() {
        let game = parse_pgn(SCHOLARS_MATE).unwrap();
        assert_eq!(game.tags.get("White").map(|s| s.as_str()), Some("A"));
        assert_eq!(game.result, "1-0");
        assert_eq!(game.moves.len(), 7);
        // Final position should be checkmate.
        assert_eq!(
            crate::outcome::detect_outcome(&game.final_position),
            Some(crate::outcome::Outcome::Checkmate)
        );
    }

    #[test]
    fn rejects_illegal_san() {
        let bad = "[Result \"*\"]\n\n1. e9 *\n";
        assert!(parse_pgn(bad).is_err());
    }
}
