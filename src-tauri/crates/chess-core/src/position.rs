use crate::bitboard::Bitboard;
use crate::types::{Color, Piece, PieceKind, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CastlingRights {
    pub white_king_side: bool,
    pub white_queen_side: bool,
    pub black_king_side: bool,
    pub black_queen_side: bool,
}

impl CastlingRights {
    pub const ALL: Self = CastlingRights {
        white_king_side: true, white_queen_side: true,
        black_king_side: true, black_queen_side: true,
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    /// Indexed by [color][piece_kind].
    pub bitboards: [[Bitboard; 6]; 2],
    pub side_to_move: Color,
    pub castling: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmove_clock: u8,
    pub fullmove_number: u16,
    pub zobrist_hash: u64,
}

impl Position {
    pub fn empty() -> Self {
        Self {
            bitboards: [[Bitboard::EMPTY; 6]; 2],
            side_to_move: Color::White,
            castling: CastlingRights::default(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            zobrist_hash: 0,
        }
    }

    pub fn starting() -> Self {
        crate::fen::parse_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")
            .expect("starting FEN is valid")
    }

    pub fn set_piece(&mut self, sq: Square, piece: Piece) {
        self.bitboards[piece.color.index()][piece.kind as usize] =
            self.bitboards[piece.color.index()][piece.kind as usize] | Bitboard::from_square(sq);
    }

    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        for c in [Color::White, Color::Black] {
            for k in 0..6 {
                if self.bitboards[c.index()][k].contains(sq) {
                    let kind = match k {
                        0 => PieceKind::Pawn, 1 => PieceKind::Knight, 2 => PieceKind::Bishop,
                        3 => PieceKind::Rook, 4 => PieceKind::Queen, 5 => PieceKind::King,
                        _ => unreachable!(),
                    };
                    return Some(Piece { color: c, kind });
                }
            }
        }
        None
    }

    pub fn occupied_by(&self, color: Color) -> Bitboard {
        let bbs = &self.bitboards[color.index()];
        bbs[0] | bbs[1] | bbs[2] | bbs[3] | bbs[4] | bbs[5]
    }

    pub fn all_pieces(&self) -> Bitboard {
        self.occupied_by(Color::White) | self.occupied_by(Color::Black)
    }

    pub fn to_fen(&self) -> String {
        crate::fen::serialize_fen(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Color, File, PieceKind, Rank, Square};

    #[test]
    fn starting_position_has_32_pieces() {
        let pos = Position::starting();
        assert_eq!(pos.all_pieces().count(), 32);
        assert_eq!(pos.side_to_move, Color::White);
    }

    #[test]
    fn starting_position_white_king_on_e1() {
        let pos = Position::starting();
        let sq = Square::new(File::E, Rank::One);
        let piece = pos.piece_at(sq).expect("king must exist");
        assert_eq!(piece.color, Color::White);
        assert_eq!(piece.kind, PieceKind::King);
    }
}
