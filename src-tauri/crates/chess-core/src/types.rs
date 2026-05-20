#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Color { White = 0, Black = 1 }

impl Color {
    pub fn flip(self) -> Color {
        match self { Color::White => Color::Black, Color::Black => Color::White }
    }
    pub fn index(self) -> usize { self as usize }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PieceKind { Pawn = 0, Knight = 1, Bishop = 2, Rook = 3, Queen = 4, King = 5 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Piece { pub color: Color, pub kind: PieceKind }

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum File { A=0, B=1, C=2, D=3, E=4, F=5, G=6, H=7 }

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Rank { One=0, Two=1, Three=2, Four=3, Five=4, Six=5, Seven=6, Eight=7 }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Square(u8); // 0..64, index = rank * 8 + file

impl Square {
    pub fn new(file: File, rank: Rank) -> Self {
        Self((rank as u8) * 8 + file as u8)
    }
    pub fn from_index(i: u8) -> Option<Self> {
        if i < 64 { Some(Self(i)) } else { None }
    }
    pub fn index(self) -> u8 { self.0 }
    pub fn file(self) -> File {
        // SAFETY: 0..64 guarantees file 0..8
        unsafe { std::mem::transmute(self.0 % 8) }
    }
    pub fn rank(self) -> Rank {
        unsafe { std::mem::transmute(self.0 / 8) }
    }
}

impl File {
    pub fn from_index(i: u8) -> Option<Self> {
        if i < 8 { Some(unsafe { std::mem::transmute(i) }) } else { None }
    }
}
impl Rank {
    pub fn from_index(i: u8) -> Option<Self> {
        if i < 8 { Some(unsafe { std::mem::transmute(i) }) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_from_file_rank_and_back() {
        let sq = Square::new(File::E, Rank::Four);
        assert_eq!(sq.file(), File::E);
        assert_eq!(sq.rank(), Rank::Four);
        assert_eq!(sq.index(), 28); // (rank 4 = index 3) * 8 + (file E = index 4)
    }

    #[test]
    fn square_round_trip_for_all_64() {
        for i in 0..64u8 {
            let sq = Square::from_index(i).unwrap();
            assert_eq!(sq.index(), i);
        }
    }

    #[test]
    fn color_flip() {
        assert_eq!(Color::White.flip(), Color::Black);
        assert_eq!(Color::Black.flip(), Color::White);
    }
}
