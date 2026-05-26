use crate::bitboard::Bitboard;
use crate::moves::{Move, MoveFlag};
use crate::position::{CastlingRights, Position};
use crate::types::{Color, File, Piece, PieceKind, Rank, Square};

#[derive(Debug, Clone, Copy)]
pub struct MoveUndo {
    pub captured: Option<Piece>,
    pub prior_castling: CastlingRights,
    pub prior_en_passant: Option<Square>,
    pub prior_halfmove_clock: u8,
    pub prior_zobrist_hash: u64,
}

pub fn make_move(pos: &mut Position, m: Move) -> MoveUndo {
    let undo = MoveUndo {
        captured: pos.piece_at(m.to()),
        prior_castling: pos.castling,
        prior_en_passant: pos.en_passant,
        prior_halfmove_clock: pos.halfmove_clock,
        prior_zobrist_hash: pos.zobrist_hash,
    };
    let us = pos.side_to_move;
    let them = us.flip();
    let mover = pos.piece_at(m.from()).expect("piece on from square");

    // Remove mover from `from`.
    clear_square(pos, m.from(), mover);

    // Handle non-EP capture.
    if let Some(cap) = undo.captured {
        clear_square(pos, m.to(), cap);
    }

    // Special moves.
    match m.flag() {
        MoveFlag::EnPassant => {
            let cap_rank = match us { Color::White => Rank::Five, Color::Black => Rank::Four };
            let cap_sq = Square::new(m.to().file(), cap_rank);
            let cap_piece = Piece { color: them, kind: PieceKind::Pawn };
            clear_square(pos, cap_sq, cap_piece);
        }
        MoveFlag::KingCastle => {
            let rank = m.from().rank();
            let rook_from = Square::new(File::H, rank);
            let rook_to   = Square::new(File::F, rank);
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, rook_from, rook);
            set_square  (pos, rook_to,   rook);
        }
        MoveFlag::QueenCastle => {
            let rank = m.from().rank();
            let rook_from = Square::new(File::A, rank);
            let rook_to   = Square::new(File::D, rank);
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, rook_from, rook);
            set_square  (pos, rook_to,   rook);
        }
        _ => {}
    }

    // Place mover on `to`, with promotion if applicable.
    let placed = match m.flag() {
        MoveFlag::PromoKnight | MoveFlag::PromoCaptureN => Piece { color: us, kind: PieceKind::Knight },
        MoveFlag::PromoBishop | MoveFlag::PromoCaptureB => Piece { color: us, kind: PieceKind::Bishop },
        MoveFlag::PromoRook   | MoveFlag::PromoCaptureR => Piece { color: us, kind: PieceKind::Rook },
        MoveFlag::PromoQueen  | MoveFlag::PromoCaptureQ => Piece { color: us, kind: PieceKind::Queen },
        _ => mover,
    };
    set_square(pos, m.to(), placed);

    update_castling_rights(pos, m, mover);

    pos.en_passant = if m.flag() == MoveFlag::DoublePawnPush {
        let r = match us { Color::White => Rank::Three, Color::Black => Rank::Six };
        Some(Square::new(m.from().file(), r))
    } else { None };

    let is_capture = m.flag().is_capture() || undo.captured.is_some();
    if mover.kind == PieceKind::Pawn || is_capture {
        pos.halfmove_clock = 0;
    } else {
        pos.halfmove_clock = pos.halfmove_clock.saturating_add(1);
    }

    if us == Color::Black { pos.fullmove_number += 1; }
    pos.side_to_move = them;

    // TODO(perf): incremental zobrist update — recomputing each move is ~30% of search time.
    pos.zobrist_hash = crate::zobrist::compute_zobrist(pos);
    undo
}

pub fn unmake_move(pos: &mut Position, m: Move, undo: MoveUndo) {
    let them = pos.side_to_move;
    let us = them.flip();

    if us == Color::Black { pos.fullmove_number -= 1; }
    pos.side_to_move = us;

    let placed = pos.piece_at(m.to()).expect("piece on to-square after move");
    clear_square(pos, m.to(), placed);

    let mover = if m.flag().is_promotion() {
        Piece { color: us, kind: PieceKind::Pawn }
    } else { placed };
    set_square(pos, m.from(), mover);

    match m.flag() {
        MoveFlag::EnPassant => {
            let cap_rank = match us { Color::White => Rank::Five, Color::Black => Rank::Four };
            let cap_sq = Square::new(m.to().file(), cap_rank);
            set_square(pos, cap_sq, Piece { color: them, kind: PieceKind::Pawn });
        }
        MoveFlag::KingCastle => {
            let rank = m.from().rank();
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, Square::new(File::F, rank), rook);
            set_square  (pos, Square::new(File::H, rank), rook);
        }
        MoveFlag::QueenCastle => {
            let rank = m.from().rank();
            let rook = Piece { color: us, kind: PieceKind::Rook };
            clear_square(pos, Square::new(File::D, rank), rook);
            set_square  (pos, Square::new(File::A, rank), rook);
        }
        _ => {
            if let Some(cap) = undo.captured {
                set_square(pos, m.to(), cap);
            }
        }
    }

    pos.castling        = undo.prior_castling;
    pos.en_passant      = undo.prior_en_passant;
    pos.halfmove_clock  = undo.prior_halfmove_clock;
    pos.zobrist_hash    = undo.prior_zobrist_hash;
}

fn set_square(pos: &mut Position, sq: Square, p: Piece) {
    pos.bitboards[p.color.index()][p.kind as usize] =
        pos.bitboards[p.color.index()][p.kind as usize] | Bitboard::from_square(sq);
}
fn clear_square(pos: &mut Position, sq: Square, p: Piece) {
    pos.bitboards[p.color.index()][p.kind as usize] =
        pos.bitboards[p.color.index()][p.kind as usize] & !Bitboard::from_square(sq);
}

fn update_castling_rights(pos: &mut Position, m: Move, mover: Piece) {
    use crate::types::PieceKind::*;
    let r = m.from().rank();
    if mover.kind == King {
        match mover.color {
            Color::White => { pos.castling.white_king_side = false; pos.castling.white_queen_side = false; }
            Color::Black => { pos.castling.black_king_side = false; pos.castling.black_queen_side = false; }
        }
    }
    if mover.kind == Rook {
        let a_rank = Square::new(File::A, r);
        let h_rank = Square::new(File::H, r);
        if m.from() == a_rank {
            match mover.color {
                Color::White => pos.castling.white_queen_side = false,
                Color::Black => pos.castling.black_queen_side = false,
            }
        } else if m.from() == h_rank {
            match mover.color {
                Color::White => pos.castling.white_king_side = false,
                Color::Black => pos.castling.black_king_side = false,
            }
        }
    }
    let a1 = Square::new(File::A, Rank::One);
    let h1 = Square::new(File::H, Rank::One);
    let a8 = Square::new(File::A, Rank::Eight);
    let h8 = Square::new(File::H, Rank::Eight);
    if m.to() == a1 { pos.castling.white_queen_side = false; }
    if m.to() == h1 { pos.castling.white_king_side  = false; }
    if m.to() == a8 { pos.castling.black_queen_side = false; }
    if m.to() == h8 { pos.castling.black_king_side  = false; }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fen::parse_fen;
    use crate::moves::{Move, MoveFlag};
    use crate::types::{File, Rank, Square};

    #[test]
    fn make_then_unmake_returns_to_original() {
        let original = crate::position::Position::starting();
        let mut p = original.clone();
        let e2 = Square::new(File::E, Rank::Two);
        let e4 = Square::new(File::E, Rank::Four);
        let m = Move::new(e2, e4, MoveFlag::DoublePawnPush);
        let undo = make_move(&mut p, m);
        assert_ne!(p, original);
        unmake_move(&mut p, m, undo);
        assert_eq!(p, original);
    }

    #[test]
    fn ep_capture_removes_correct_pawn() {
        let pos = parse_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1").unwrap();
        let mut p = pos.clone();
        let e5 = Square::new(File::E, Rank::Five);
        let d6 = Square::new(File::D, Rank::Six);
        let d5 = Square::new(File::D, Rank::Five);
        let _ = make_move(&mut p, Move::new(e5, d6, MoveFlag::EnPassant));
        assert!(p.piece_at(d5).is_none(), "captured pawn should be gone");
        assert!(p.piece_at(d6).is_some(), "moving pawn now on d6");
    }
}
