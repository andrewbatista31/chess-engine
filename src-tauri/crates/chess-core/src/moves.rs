use crate::types::Square;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveFlag {
    Quiet           = 0,
    DoublePawnPush  = 1,
    KingCastle      = 2,
    QueenCastle     = 3,
    Capture         = 4,
    EnPassant       = 5,
    PromoKnight     = 8,
    PromoBishop     = 9,
    PromoRook       = 10,
    PromoQueen      = 11,
    PromoCaptureN   = 12,
    PromoCaptureB   = 13,
    PromoCaptureR   = 14,
    PromoCaptureQ   = 15,
}

impl MoveFlag {
    pub fn is_capture(self) -> bool {
        matches!(self,
            MoveFlag::Capture | MoveFlag::EnPassant |
            MoveFlag::PromoCaptureN | MoveFlag::PromoCaptureB |
            MoveFlag::PromoCaptureR | MoveFlag::PromoCaptureQ
        )
    }
    pub fn is_promotion(self) -> bool { (self as u8) & 0b1000 != 0 }
}

/// Packed move: bits 0-5 = from, 6-11 = to, 12-15 = flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Move(pub u16);

impl Move {
    pub fn new(from: Square, to: Square, flag: MoveFlag) -> Self {
        Move(
            (from.index() as u16) |
            ((to.index() as u16) << 6) |
            ((flag as u16) << 12)
        )
    }
    pub fn from(self) -> Square { Square::from_index((self.0 & 0x3F) as u8).unwrap() }
    pub fn to  (self) -> Square { Square::from_index(((self.0 >> 6) & 0x3F) as u8).unwrap() }
    pub fn flag(self) -> MoveFlag {
        // SAFETY: flag values 0..16 are all defined
        unsafe { std::mem::transmute::<u8, MoveFlag>(((self.0 >> 12) & 0xF) as u8) }
    }
}

pub type MoveList = arrayvec::ArrayVec<Move, 256>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{File, Rank};

    #[test]
    fn pack_and_unpack_quiet_move() {
        let from = Square::new(File::E, Rank::Two);
        let to   = Square::new(File::E, Rank::Four);
        let m = Move::new(from, to, MoveFlag::Quiet);
        assert_eq!(m.from(), from);
        assert_eq!(m.to(), to);
        assert_eq!(m.flag(), MoveFlag::Quiet);
    }

    #[test]
    fn promotion_flags_distinct() {
        let from = Square::new(File::A, Rank::Seven);
        let to   = Square::new(File::A, Rank::Eight);
        let mq = Move::new(from, to, MoveFlag::PromoQueen);
        let mn = Move::new(from, to, MoveFlag::PromoKnight);
        assert_ne!(mq.0, mn.0);
        assert_eq!(mq.flag(), MoveFlag::PromoQueen);
        assert_eq!(mn.flag(), MoveFlag::PromoKnight);
    }
}
