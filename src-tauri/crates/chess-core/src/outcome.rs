use crate::movegen::{is_in_check, legal_moves};
use crate::position::Position;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Checkmate,
    Stalemate,
    FiftyMoveRule,
    InsufficientMaterial,
}

pub fn detect_outcome(pos: &Position) -> Option<Outcome> {
    if pos.halfmove_clock >= 100 {
        return Some(Outcome::FiftyMoveRule);
    }
    if has_insufficient_material(pos) {
        return Some(Outcome::InsufficientMaterial);
    }
    if legal_moves(pos).is_empty() {
        return Some(if is_in_check(pos) {
            Outcome::Checkmate
        } else {
            Outcome::Stalemate
        });
    }
    None
}

fn has_insufficient_material(pos: &Position) -> bool {
    use crate::types::{Color, PieceKind};
    let mut total_minor = 0u32;
    let mut total_other = 0u32;
    for c in [Color::White, Color::Black] {
        for k in 0..6 {
            let n = pos.bitboards[c.index()][k].count();
            match k {
                k if k == PieceKind::King as usize => {}
                k if k == PieceKind::Knight as usize || k == PieceKind::Bishop as usize => {
                    total_minor += n;
                }
                _ => {
                    total_other += n;
                }
            }
        }
    }
    total_other == 0 && total_minor <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;

    #[test]
    fn fools_mate_is_checkmate() {
        let pos = parse_fen("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3")
            .unwrap();
        assert_eq!(detect_outcome(&pos), Some(Outcome::Checkmate));
    }

    #[test]
    fn stalemate_detected() {
        let pos =
            parse_fen("k7/2Q5/1K6/8/8/8/8/8 b - - 0 1").unwrap();
        assert_eq!(detect_outcome(&pos), Some(Outcome::Stalemate));
    }

    #[test]
    fn fifty_move_rule() {
        let mut pos = crate::position::Position::starting();
        pos.halfmove_clock = 100;
        assert_eq!(detect_outcome(&pos), Some(Outcome::FiftyMoveRule));
    }

    #[test]
    fn starting_position_is_ongoing() {
        let pos = crate::position::Position::starting();
        assert_eq!(detect_outcome(&pos), None);
    }
}
