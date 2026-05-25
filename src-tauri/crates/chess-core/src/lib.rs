//! chess-core: pure chess rules library.
//!
//! Public surface intended for downstream crates lives in the prelude.

pub mod attacks;
pub mod bitboard;
pub mod fen;
pub mod make_move;
pub mod movegen;
pub mod moves;
pub mod outcome;
pub mod perft;
pub mod pgn;
pub mod position;
pub mod types;
pub mod zobrist;

pub mod prelude {
    pub use crate::bitboard::Bitboard;
    pub use crate::fen::{parse_fen, serialize_fen, FenError};
    pub use crate::make_move::{make_move, unmake_move, MoveUndo};
    pub use crate::movegen::{is_in_check, legal_moves, pseudo_legal_moves, square_attacked};
    pub use crate::moves::{Move, MoveFlag, MoveList};
    pub use crate::outcome::{detect_outcome, Outcome};
    pub use crate::perft::{perft, perft_divide};
    pub use crate::pgn::{parse_pgn, serialize_pgn, Game, PgnError};
    pub use crate::position::{CastlingRights, Position};
    pub use crate::types::{Color, File, Piece, PieceKind, Rank, Square};
}
