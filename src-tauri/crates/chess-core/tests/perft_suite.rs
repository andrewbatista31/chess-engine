//! Perft suite — the make-or-break correctness check for move generation.
//!
//! Reference: https://www.chessprogramming.org/Perft_Results

use chess_core::fen::parse_fen;
use chess_core::perft::perft;

fn run(fen: &str, depth_results: &[(u32, u64)]) {
    let pos = parse_fen(fen).expect("valid fen");
    for &(d, expected) in depth_results {
        let actual = perft(&pos, d);
        assert_eq!(
            actual, expected,
            "perft({d}) mismatch for {fen}: expected {expected}, got {actual}"
        );
    }
}

#[test]
fn perft_position_1_starting() {
    run(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &[(1, 20), (2, 400), (3, 8902), (4, 197281)],
    );
}

#[test]
fn perft_position_2_kiwipete() {
    run(
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
        &[(1, 48), (2, 2039), (3, 97862)],
    );
}

#[test]
fn perft_position_3_endgame() {
    run(
        "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
        &[(1, 14), (2, 191), (3, 2812), (4, 43238)],
    );
}

#[test]
fn perft_position_4() {
    run(
        "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
        &[(1, 6), (2, 264), (3, 9467)],
    );
}

#[test]
fn perft_position_5() {
    run(
        "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
        &[(1, 44), (2, 1486), (3, 62379)],
    );
}

#[test]
fn perft_position_6() {
    run(
        "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
        &[(1, 46), (2, 2079), (3, 89890)],
    );
}
