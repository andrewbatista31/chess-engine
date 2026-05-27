//! Pure types for engine abstraction. No async runtime, no I/O.

use chess_core::prelude::{Color, Move, Position};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Score {
    /// Centipawns from the side-to-move's perspective.
    Cp(i32),
    /// Forced mate in N plies (positive = side to move wins, negative = side to move loses).
    Mate(i8),
}

#[derive(Clone, Debug)]
pub struct SearchLimits {
    pub movetime_ms: u32,
    pub multipv: u8,
}

#[derive(Clone, Debug)]
pub struct AnalysisInfo {
    pub depth: u8,
    pub score: Score,
    pub pv: Vec<Move>,
    pub multipv_index: u8, // 1-based per UCI convention
    pub nodes: u64,
    pub nps: u64,
    pub time_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineKind {
    Human,
    Stockfish,
}

/// One trait, two callbacks: `on_info` per UCI info line, `on_bestmove` once at the end.
/// Callback-based to keep the trait runtime-agnostic.
pub trait Engine {
    fn name(&self) -> &str;
    fn analyze(
        &mut self,
        position: Position,
        limits: SearchLimits,
        skill_level: u8,
        on_info: Box<dyn FnMut(AnalysisInfo) + Send>,
        on_bestmove: Box<dyn FnOnce(Move) + Send>,
    );
    fn stop(&mut self);
}

/// In-process test double — emits one canned info, then one canned bestmove.
/// Use the builder methods to configure what each `analyze()` call yields.
#[cfg(any(test, feature = "mock"))]
pub mod mock {
    use super::*;

    pub struct MockEngine {
        canned_info: Option<AnalysisInfo>,
        canned_bestmove: Option<Move>,
        stop_called: bool,
    }

    impl MockEngine {
        pub fn new(canned_bestmove: Move) -> Self {
            Self {
                canned_info: None,
                canned_bestmove: Some(canned_bestmove),
                stop_called: false,
            }
        }
        pub fn with_info(mut self, info: AnalysisInfo) -> Self {
            self.canned_info = Some(info);
            self
        }
        pub fn was_stopped(&self) -> bool { self.stop_called }
    }

    impl Engine for MockEngine {
        fn name(&self) -> &str { "MockEngine" }
        fn analyze(
            &mut self,
            _position: Position,
            _limits: SearchLimits,
            _skill_level: u8,
            mut on_info: Box<dyn FnMut(AnalysisInfo) + Send>,
            on_bestmove: Box<dyn FnOnce(Move) + Send>,
        ) {
            if let Some(info) = self.canned_info.clone() {
                on_info(info);
            }
            if let Some(mv) = self.canned_bestmove {
                on_bestmove(mv);
            }
        }
        fn stop(&mut self) { self.stop_called = true; }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::prelude::*;

    #[test]
    fn score_cp_round_trips_via_serde() {
        let s = Score::Cp(42);
        let j = serde_json::to_string(&s).unwrap();
        assert_eq!(j, r#"{"kind":"Cp","value":42}"#);
        let back: Score = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn score_mate_round_trips_via_serde() {
        let s = Score::Mate(-3);
        let back: Score = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn engine_trait_is_object_safe() {
        // Compile-time check: trait can live behind a Box<dyn Engine>.
        fn _accepts(_: Box<dyn Engine + Send>) {}
    }

    #[test]
    fn mock_engine_emits_info_then_bestmove_and_stop() {
        use crate::mock::MockEngine;
        use std::sync::{Arc, Mutex};

        let m1 = Move::new(
            Square::new(File::E, Rank::Two),
            Square::new(File::E, Rank::Four),
            MoveFlag::DoublePawnPush,
        );
        let info = AnalysisInfo {
            depth: 1, score: Score::Cp(25), pv: vec![m1],
            multipv_index: 1, nodes: 100, nps: 1000, time_ms: 100,
        };
        let mut eng = MockEngine::new(m1).with_info(info.clone());

        let info_calls: Arc<Mutex<Vec<AnalysisInfo>>> = Default::default();
        let bestmove_calls: Arc<Mutex<Vec<Move>>> = Default::default();
        let ic = info_calls.clone();
        let bc = bestmove_calls.clone();

        eng.analyze(
            Position::starting(),
            SearchLimits { movetime_ms: 100, multipv: 1 },
            10,
            Box::new(move |i| ic.lock().unwrap().push(i)),
            Box::new(move |m| bc.lock().unwrap().push(m)),
        );

        assert_eq!(info_calls.lock().unwrap().len(), 1);
        assert_eq!(info_calls.lock().unwrap()[0].depth, 1);
        assert_eq!(bestmove_calls.lock().unwrap().len(), 1);
        assert_eq!(bestmove_calls.lock().unwrap()[0], m1);

        assert!(!eng.was_stopped());
        eng.stop();
        assert!(eng.was_stopped());
    }
}
