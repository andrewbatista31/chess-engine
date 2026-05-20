use crate::position::{CastlingRights, Position};
use crate::types::{Color, File, Piece, PieceKind, Rank, Square};

#[derive(Debug)]
pub enum FenError {
    BadField(&'static str),
    BadChar(char),
    WrongFieldCount,
}

pub fn parse_fen(s: &str) -> Result<Position, FenError> {
    let mut fields = s.split_ascii_whitespace();
    let board    = fields.next().ok_or(FenError::WrongFieldCount)?;
    let stm      = fields.next().ok_or(FenError::WrongFieldCount)?;
    let castling = fields.next().ok_or(FenError::WrongFieldCount)?;
    let ep       = fields.next().ok_or(FenError::WrongFieldCount)?;
    let hmc      = fields.next().ok_or(FenError::WrongFieldCount)?;
    let fmn      = fields.next().ok_or(FenError::WrongFieldCount)?;

    let mut pos = Position::empty();

    let mut rank = 7i32;
    for row in board.split('/') {
        if rank < 0 { return Err(FenError::BadField("board")); }
        let mut file = 0i32;
        for ch in row.chars() {
            if ch.is_ascii_digit() {
                file += ch.to_digit(10).unwrap() as i32;
            } else {
                if file >= 8 { return Err(FenError::BadField("board")); }
                let (color, kind) = match ch {
                    'P' => (Color::White, PieceKind::Pawn),
                    'N' => (Color::White, PieceKind::Knight),
                    'B' => (Color::White, PieceKind::Bishop),
                    'R' => (Color::White, PieceKind::Rook),
                    'Q' => (Color::White, PieceKind::Queen),
                    'K' => (Color::White, PieceKind::King),
                    'p' => (Color::Black, PieceKind::Pawn),
                    'n' => (Color::Black, PieceKind::Knight),
                    'b' => (Color::Black, PieceKind::Bishop),
                    'r' => (Color::Black, PieceKind::Rook),
                    'q' => (Color::Black, PieceKind::Queen),
                    'k' => (Color::Black, PieceKind::King),
                    c => return Err(FenError::BadChar(c)),
                };
                let sq = Square::new(
                    File::from_index(file as u8).ok_or(FenError::BadField("board"))?,
                    Rank::from_index(rank as u8).ok_or(FenError::BadField("board"))?,
                );
                pos.set_piece(sq, Piece { color, kind });
                file += 1;
            }
        }
        if file != 8 { return Err(FenError::BadField("board")); }
        rank -= 1;
    }
    if rank != -1 { return Err(FenError::BadField("board")); }

    pos.side_to_move = match stm {
        "w" => Color::White,
        "b" => Color::Black,
        _ => return Err(FenError::BadField("side_to_move")),
    };

    pos.castling = CastlingRights::default();
    if castling != "-" {
        for ch in castling.chars() {
            match ch {
                'K' => pos.castling.white_king_side  = true,
                'Q' => pos.castling.white_queen_side = true,
                'k' => pos.castling.black_king_side  = true,
                'q' => pos.castling.black_queen_side = true,
                _ => return Err(FenError::BadField("castling")),
            }
        }
    }

    pos.en_passant = if ep == "-" { None } else { Some(parse_square(ep)?) };
    pos.halfmove_clock  = hmc.parse().map_err(|_| FenError::BadField("halfmove_clock"))?;
    pos.fullmove_number = fmn.parse().map_err(|_| FenError::BadField("fullmove_number"))?;

    Ok(pos)
}

fn parse_square(s: &str) -> Result<Square, FenError> {
    let bytes = s.as_bytes();
    if bytes.len() != 2 { return Err(FenError::BadField("square")); }
    let file = File::from_index(bytes[0].wrapping_sub(b'a')).ok_or(FenError::BadField("square"))?;
    let rank = Rank::from_index(bytes[1].wrapping_sub(b'1')).ok_or(FenError::BadField("square"))?;
    Ok(Square::new(file, rank))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Color;

    const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

    #[test]
    fn parses_starting_fen() {
        let pos = parse_fen(START_FEN).unwrap();
        assert_eq!(pos.side_to_move, Color::White);
        assert_eq!(pos.all_pieces().count(), 32);
        assert!(pos.castling.white_king_side);
        assert!(pos.castling.black_queen_side);
        assert_eq!(pos.halfmove_clock, 0);
        assert_eq!(pos.fullmove_number, 1);
    }

    #[test]
    fn parses_kiwipete() {
        let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
        let pos = parse_fen(fen).unwrap();
        assert_eq!(pos.all_pieces().count(), 32);
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse_fen("not a fen").is_err());
        assert!(parse_fen("8/8/8/8/8/8/8/8 w - - 0 1").is_ok()); // empty board OK
    }
}
