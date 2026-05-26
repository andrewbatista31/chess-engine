use crate::types::Square;
use std::ops::{BitAnd, BitOr, BitXor, Not, Shl, Shr};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0);
    pub const FULL:  Bitboard = Bitboard(u64::MAX);

    pub fn from_square(sq: Square) -> Self { Bitboard(1u64 << sq.index()) }
    pub fn contains(self, sq: Square) -> bool { self.0 & (1u64 << sq.index()) != 0 }
    pub fn count(self) -> u32 { self.0.count_ones() }
    pub fn is_empty(self) -> bool { self.0 == 0 }

    /// Pop and return the least significant bit's square, or None if empty.
    pub fn pop_lsb(&mut self) -> Option<Square> {
        if self.0 == 0 { return None; }
        let idx = self.0.trailing_zeros() as u8;
        self.0 &= self.0 - 1;
        Square::from_index(idx)
    }
}

impl BitAnd for Bitboard { type Output = Self; fn bitand(self, r: Self) -> Self { Bitboard(self.0 & r.0) } }
impl BitOr  for Bitboard { type Output = Self; fn bitor (self, r: Self) -> Self { Bitboard(self.0 | r.0) } }
impl BitXor for Bitboard { type Output = Self; fn bitxor(self, r: Self) -> Self { Bitboard(self.0 ^ r.0) } }
impl Not    for Bitboard { type Output = Self; fn not(self) -> Self { Bitboard(!self.0) } }
impl Shl<u32> for Bitboard { type Output = Self; fn shl(self, s: u32) -> Self { Bitboard(self.0 << s) } }
impl Shr<u32> for Bitboard { type Output = Self; fn shr(self, s: u32) -> Self { Bitboard(self.0 >> s) } }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{File, Rank, Square};

    #[test]
    fn empty_and_full() {
        assert_eq!(Bitboard::EMPTY.0, 0);
        assert_eq!(Bitboard::FULL.0, u64::MAX);
    }

    #[test]
    fn set_and_get_square() {
        let bb = Bitboard::from_square(Square::new(File::E, Rank::Four));
        assert!(bb.contains(Square::new(File::E, Rank::Four)));
        assert!(!bb.contains(Square::new(File::A, Rank::One)));
        assert_eq!(bb.count(), 1);
    }

    #[test]
    fn pop_lsb_iterates_all_squares() {
        let mut bb = Bitboard(0b1011);
        let mut squares = vec![];
        while let Some(sq) = bb.pop_lsb() {
            squares.push(sq.index());
        }
        assert_eq!(squares, vec![0, 1, 3]);
    }

    #[test]
    fn bitwise_ops() {
        let a = Bitboard(0b1100);
        let b = Bitboard(0b1010);
        assert_eq!((a & b).0, 0b1000);
        assert_eq!((a | b).0, 0b1110);
        assert_eq!((a ^ b).0, 0b0110);
        assert_eq!((!a).0, !0b1100u64);
    }
}
