use crate::attacks::*;
use crate::bitboard::Bitboard;
use crate::moves::{Move, MoveFlag, MoveList};
use crate::position::Position;
use crate::types::{Color, PieceKind, Square};

pub fn pseudo_legal_moves(pos: &Position) -> MoveList {
    let mut out = MoveList::new();
    let us = pos.side_to_move;
    let our_pieces = pos.occupied_by(us);
    let their_pieces = pos.occupied_by(us.flip());
    let occ = our_pieces | their_pieces;

    gen_pawn_moves(pos, us, occ, their_pieces, &mut out);
    gen_piece_moves(pos, us, PieceKind::Knight, our_pieces, their_pieces, occ, &mut out, knight_attacks_wrap);
    gen_piece_moves(pos, us, PieceKind::Bishop, our_pieces, their_pieces, occ, &mut out, bishop_attacks);
    gen_piece_moves(pos, us, PieceKind::Rook, our_pieces, their_pieces, occ, &mut out, rook_attacks);
    gen_piece_moves(pos, us, PieceKind::Queen, our_pieces, their_pieces, occ, &mut out, queen_attacks);
    gen_piece_moves(pos, us, PieceKind::King, our_pieces, their_pieces, occ, &mut out, king_attacks_wrap);
    gen_castling(pos, us, occ, &mut out);

    out
}

fn knight_attacks_wrap(sq: Square, _o: Bitboard) -> Bitboard {
    knight_attacks(sq)
}
fn king_attacks_wrap(sq: Square, _o: Bitboard) -> Bitboard {
    king_attacks(sq)
}

#[allow(clippy::too_many_arguments)]
fn gen_piece_moves<F>(
    pos: &Position,
    us: Color,
    kind: PieceKind,
    our_pieces: Bitboard,
    their_pieces: Bitboard,
    occ: Bitboard,
    out: &mut MoveList,
    attacks_fn: F,
) where
    F: Fn(Square, Bitboard) -> Bitboard,
{
    let mut bb = pos.bitboards[us.index()][kind as usize];
    while let Some(from) = bb.pop_lsb() {
        let mut targets = attacks_fn(from, occ) & !our_pieces;
        while let Some(to) = targets.pop_lsb() {
            let flag = if their_pieces.contains(to) {
                MoveFlag::Capture
            } else {
                MoveFlag::Quiet
            };
            out.push(Move::new(from, to, flag));
        }
    }
}

fn gen_pawn_moves(
    pos: &Position,
    us: Color,
    occ: Bitboard,
    their_pieces: Bitboard,
    out: &mut MoveList,
) {
    use crate::types::{File, Rank};
    let mut pawns = pos.bitboards[us.index()][PieceKind::Pawn as usize];
    let (push_dir, double_rank, promo_rank) = match us {
        Color::White => (1i32, Rank::Two, Rank::Eight),
        Color::Black => (-1i32, Rank::Seven, Rank::One),
    };

    while let Some(from) = pawns.pop_lsb() {
        let r = from.rank() as i32;
        let f = from.file() as i32;

        // single push
        let nr = r + push_dir;
        if (0..8).contains(&nr) {
            let to = Square::new(
                File::from_index(f as u8).unwrap(),
                Rank::from_index(nr as u8).unwrap(),
            );
            if !occ.contains(to) {
                if to.rank() == promo_rank {
                    for promo in [
                        MoveFlag::PromoQueen,
                        MoveFlag::PromoRook,
                        MoveFlag::PromoBishop,
                        MoveFlag::PromoKnight,
                    ] {
                        out.push(Move::new(from, to, promo));
                    }
                } else {
                    out.push(Move::new(from, to, MoveFlag::Quiet));
                    if from.rank() == double_rank {
                        let nr2 = r + 2 * push_dir;
                        let to2 = Square::new(
                            File::from_index(f as u8).unwrap(),
                            Rank::from_index(nr2 as u8).unwrap(),
                        );
                        if !occ.contains(to2) {
                            out.push(Move::new(from, to2, MoveFlag::DoublePawnPush));
                        }
                    }
                }
            }
        }

        // diagonal captures
        for df in [-1i32, 1] {
            let nf = f + df;
            if !(0..8).contains(&nf) || !(0..8).contains(&nr) {
                continue;
            }
            let to = Square::new(
                File::from_index(nf as u8).unwrap(),
                Rank::from_index(nr as u8).unwrap(),
            );
            if their_pieces.contains(to) {
                if to.rank() == promo_rank {
                    for promo in [
                        MoveFlag::PromoCaptureQ,
                        MoveFlag::PromoCaptureR,
                        MoveFlag::PromoCaptureB,
                        MoveFlag::PromoCaptureN,
                    ] {
                        out.push(Move::new(from, to, promo));
                    }
                } else {
                    out.push(Move::new(from, to, MoveFlag::Capture));
                }
            }
        }

        // en passant
        if let Some(ep_sq) = pos.en_passant {
            let ep_f = ep_sq.file() as i32;
            let ep_r = ep_sq.rank() as i32;
            if ep_r == nr && (ep_f - f).abs() == 1 {
                out.push(Move::new(from, ep_sq, MoveFlag::EnPassant));
            }
        }
    }
}

fn gen_castling(pos: &Position, us: Color, occ: Bitboard, out: &mut MoveList) {
    use crate::types::{File, Rank};
    let (ks_right, qs_right, rank) = match us {
        Color::White => (pos.castling.white_king_side, pos.castling.white_queen_side, Rank::One),
        Color::Black => (pos.castling.black_king_side, pos.castling.black_queen_side, Rank::Eight),
    };
    let e = Square::new(File::E, rank);
    if ks_right {
        let f = Square::new(File::F, rank);
        let g = Square::new(File::G, rank);
        if !occ.contains(f) && !occ.contains(g) {
            out.push(Move::new(e, g, MoveFlag::KingCastle));
        }
    }
    if qs_right {
        let b = Square::new(File::B, rank);
        let c = Square::new(File::C, rank);
        let d = Square::new(File::D, rank);
        if !occ.contains(b) && !occ.contains(c) && !occ.contains(d) {
            out.push(Move::new(e, c, MoveFlag::QueenCastle));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::position::Position;

    #[test]
    fn starting_position_has_20_pseudo_legal_moves() {
        let pos = Position::starting();
        let moves = pseudo_legal_moves(&pos);
        assert_eq!(moves.len(), 20);
    }

    #[test]
    fn black_to_move_starting_position_has_20() {
        let mut pos = Position::starting();
        pos.side_to_move = crate::types::Color::Black;
        assert_eq!(pseudo_legal_moves(&pos).len(), 20);
    }

    #[test]
    fn castling_when_clear() {
        let fen = "4k3/8/8/8/8/8/8/R3K2R w KQ - 0 1";
        let pos = crate::fen::parse_fen(fen).unwrap();
        let moves = pseudo_legal_moves(&pos);
        let castles: Vec<_> = moves.into_iter()
            .filter(|m| matches!(m.flag(), crate::moves::MoveFlag::KingCastle | crate::moves::MoveFlag::QueenCastle))
            .collect();
        assert_eq!(castles.len(), 2);
    }

    #[test]
    fn en_passant_square_produces_capture() {
        let fen = "4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1";
        let pos = crate::fen::parse_fen(fen).unwrap();
        let ep_moves: Vec<_> = pseudo_legal_moves(&pos).into_iter()
            .filter(|m| m.flag() == crate::moves::MoveFlag::EnPassant)
            .collect();
        assert_eq!(ep_moves.len(), 1);
    }
}
