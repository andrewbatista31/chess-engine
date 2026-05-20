use crate::bitboard::Bitboard;
use crate::types::{Color, Square};

const FILE_A: u64 = 0x0101010101010101;
const FILE_H: u64 = 0x8080808080808080;
const FILE_AB: u64 = FILE_A | (FILE_A << 1);
const FILE_GH: u64 = FILE_H | (FILE_H >> 1);

pub fn knight_attacks(sq: Square) -> Bitboard {
    let b = 1u64 << sq.index();
    let attacks =
        ((b << 17) & !FILE_A)
      | ((b << 15) & !FILE_H)
      | ((b << 10) & !FILE_AB)
      | ((b <<  6) & !FILE_GH)
      | ((b >> 17) & !FILE_H)
      | ((b >> 15) & !FILE_A)
      | ((b >> 10) & !FILE_GH)
      | ((b >>  6) & !FILE_AB);
    Bitboard(attacks)
}

pub fn king_attacks(sq: Square) -> Bitboard {
    let b = 1u64 << sq.index();
    let attacks =
        (b << 8)
      | (b >> 8)
      | ((b << 1) & !FILE_A)
      | ((b >> 1) & !FILE_H)
      | ((b << 9) & !FILE_A)
      | ((b << 7) & !FILE_H)
      | ((b >> 7) & !FILE_A)
      | ((b >> 9) & !FILE_H);
    Bitboard(attacks)
}

pub fn pawn_attacks(color: Color, sq: Square) -> Bitboard {
    let b = 1u64 << sq.index();
    let attacks = match color {
        Color::White => ((b << 9) & !FILE_A) | ((b << 7) & !FILE_H),
        Color::Black => ((b >> 7) & !FILE_A) | ((b >> 9) & !FILE_H),
    };
    Bitboard(attacks)
}

const ROOK_DIRS: [(i32, i32); 4] = [(0, 1), (0, -1), (1, 0), (-1, 0)];
const BISHOP_DIRS: [(i32, i32); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];

fn ray_attacks(sq: Square, occ: Bitboard, dirs: &[(i32, i32)]) -> Bitboard {
    let mut bb = 0u64;
    let f0 = (sq.index() % 8) as i32;
    let r0 = (sq.index() / 8) as i32;
    for &(df, dr) in dirs {
        let (mut f, mut r) = (f0 + df, r0 + dr);
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let idx = (r * 8 + f) as u8;
            let bit = 1u64 << idx;
            bb |= bit;
            if occ.0 & bit != 0 {
                break;
            }
            f += df;
            r += dr;
        }
    }
    Bitboard(bb)
}

pub fn rook_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    ray_attacks(sq, occ, &ROOK_DIRS)
}

pub fn bishop_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    ray_attacks(sq, occ, &BISHOP_DIRS)
}

pub fn queen_attacks(sq: Square, occ: Bitboard) -> Bitboard {
    rook_attacks(sq, occ) | bishop_attacks(sq, occ)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, File, Rank, Square};

    #[test]
    fn knight_attacks_from_d4_hits_8_squares() {
        let d4 = Square::new(File::D, Rank::Four);
        assert_eq!(knight_attacks(d4).count(), 8);
    }

    #[test]
    fn knight_attacks_from_a1_hits_2_squares() {
        let a1 = Square::new(File::A, Rank::One);
        assert_eq!(knight_attacks(a1).count(), 2);
    }

    #[test]
    fn king_attacks_from_e4_hits_8_squares() {
        let e4 = Square::new(File::E, Rank::Four);
        assert_eq!(king_attacks(e4).count(), 8);
    }

    #[test]
    fn white_pawn_attacks_from_e2_hits_d3_and_f3() {
        let e2 = Square::new(File::E, Rank::Two);
        let attacks = pawn_attacks(Color::White, e2);
        assert!(attacks.contains(Square::new(File::D, Rank::Three)));
        assert!(attacks.contains(Square::new(File::F, Rank::Three)));
        assert_eq!(attacks.count(), 2);
    }

    #[test]
    fn pawn_attack_at_corner() {
        let a2 = Square::new(File::A, Rank::Two);
        let attacks = pawn_attacks(Color::White, a2);
        assert_eq!(attacks.count(), 1);
    }

    #[test]
    fn rook_on_a1_empty_board_sees_a_file_and_rank_1() {
        let a1 = Square::new(File::A, Rank::One);
        let attacks = rook_attacks(a1, Bitboard::EMPTY);
        assert_eq!(attacks.count(), 14);
    }

    #[test]
    fn rook_attack_blocked_by_own_piece_includes_blocker() {
        let a1 = Square::new(File::A, Rank::One);
        let blocker = Bitboard::from_square(Square::new(File::A, Rank::Four));
        let attacks = rook_attacks(a1, blocker);
        assert_eq!(attacks.count(), 10);
        assert!(attacks.contains(Square::new(File::A, Rank::Four)));
        assert!(!attacks.contains(Square::new(File::A, Rank::Five)));
    }

    #[test]
    fn bishop_on_d4_empty_board_hits_13_squares() {
        let d4 = Square::new(File::D, Rank::Four);
        assert_eq!(bishop_attacks(d4, Bitboard::EMPTY).count(), 13);
    }

    #[test]
    fn queen_combines_rook_and_bishop() {
        let d4 = Square::new(File::D, Rank::Four);
        let combined = (rook_attacks(d4, Bitboard::EMPTY) | bishop_attacks(d4, Bitboard::EMPTY)).count();
        assert_eq!(queen_attacks(d4, Bitboard::EMPTY).count(), combined);
    }
}
