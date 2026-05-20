use crate::position::Position;
use crate::types::Color;

pub struct ZobristKeys {
    pub pieces: [[[u64; 64]; 6]; 2],
    pub side_to_move: u64,
    pub castling: [u64; 16],
    pub en_passant_file: [u64; 8],
}

const fn xorshift64(mut x: u64) -> u64 {
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    x
}

pub static KEYS: ZobristKeys = build_keys();

const fn build_keys() -> ZobristKeys {
    let mut seed: u64 = 0x9E3779B97F4A7C15;
    let mut pieces = [[[0u64; 64]; 6]; 2];
    let mut c = 0;
    while c < 2 {
        let mut k = 0;
        while k < 6 {
            let mut s = 0;
            while s < 64 {
                seed = xorshift64(seed);
                pieces[c][k][s] = seed;
                s += 1;
            }
            k += 1;
        }
        c += 1;
    }
    seed = xorshift64(seed);
    let side_to_move = seed;
    let mut castling = [0u64; 16];
    let mut i = 0;
    while i < 16 {
        seed = xorshift64(seed);
        castling[i] = seed;
        i += 1;
    }
    let mut en_passant_file = [0u64; 8];
    let mut f = 0;
    while f < 8 {
        seed = xorshift64(seed);
        en_passant_file[f] = seed;
        f += 1;
    }
    ZobristKeys { pieces, side_to_move, castling, en_passant_file }
}

pub fn castling_index(c: crate::position::CastlingRights) -> usize {
    (c.white_king_side as usize)
        | ((c.white_queen_side as usize) << 1)
        | ((c.black_king_side as usize) << 2)
        | ((c.black_queen_side as usize) << 3)
}

pub fn compute_zobrist(pos: &Position) -> u64 {
    let mut h: u64 = 0;
    for c in [Color::White, Color::Black] {
        for k in 0..6 {
            let mut bb = pos.bitboards[c.index()][k];
            while let Some(sq) = bb.pop_lsb() {
                h ^= KEYS.pieces[c.index()][k][sq.index() as usize];
            }
        }
    }
    if pos.side_to_move == Color::Black {
        h ^= KEYS.side_to_move;
    }
    h ^= KEYS.castling[castling_index(pos.castling)];
    if let Some(ep) = pos.en_passant {
        h ^= KEYS.en_passant_file[ep.file() as usize];
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;
    use crate::make_move::{make_move, unmake_move};
    use crate::moves::{Move, MoveFlag};
    use crate::types::{File, Rank, Square};

    #[test]
    fn equal_positions_have_equal_hash() {
        let a = crate::position::Position::starting();
        let b = parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1").unwrap();
        assert_eq!(a.zobrist_hash, b.zobrist_hash);
        assert_ne!(a.zobrist_hash, 0);
    }

    #[test]
    fn incremental_update_matches_full_recompute() {
        let mut p = crate::position::Position::starting();
        let m = Move::new(
            Square::new(File::E, Rank::Two),
            Square::new(File::E, Rank::Four),
            MoveFlag::DoublePawnPush,
        );
        let undo = make_move(&mut p, m);
        let recomputed = compute_zobrist(&p);
        assert_eq!(p.zobrist_hash, recomputed);
        unmake_move(&mut p, m, undo);
        assert_eq!(p.zobrist_hash, compute_zobrist(&p));
    }
}
