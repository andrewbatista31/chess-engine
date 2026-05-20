use crate::make_move::{make_move, unmake_move};
use crate::movegen::legal_moves;
use crate::position::Position;

pub fn perft(pos: &Position, depth: u32) -> u64 {
    if depth == 0 { return 1; }
    let moves = legal_moves(pos);
    if depth == 1 { return moves.len() as u64; }
    let mut probe = pos.clone();
    let mut nodes = 0u64;
    for m in moves {
        let undo = make_move(&mut probe, m);
        nodes += perft(&probe, depth - 1);
        unmake_move(&mut probe, m, undo);
    }
    nodes
}

/// Divides nodes by root move — useful for debugging when totals disagree with reference.
pub fn perft_divide(pos: &Position, depth: u32) -> Vec<(crate::moves::Move, u64)> {
    let mut out = vec![];
    let mut probe = pos.clone();
    for m in legal_moves(pos) {
        let undo = make_move(&mut probe, m);
        let n = if depth <= 1 { 1 } else { perft(&probe, depth - 1) };
        unmake_move(&mut probe, m, undo);
        out.push((m, n));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn perft_starting_depth_1_is_20() {
        let pos = Position::starting();
        assert_eq!(perft(&pos, 1), 20);
    }

    #[test]
    fn perft_starting_depth_2_is_400() {
        let pos = Position::starting();
        assert_eq!(perft(&pos, 2), 400);
    }
}
