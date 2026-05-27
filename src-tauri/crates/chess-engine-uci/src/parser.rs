use chess_core::prelude::{File, PieceKind, Rank, Square};
use chess_engine_api::Score;

/// A raw move parsed from UCI long algebraic notation (e.g. "e2e4", "e7e8q").
/// Position-independent — the caller resolves to a chess_core::Move against the position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

/// Information from a UCI `info` line. Score is from the side-to-move's perspective.
#[derive(Clone, Debug, PartialEq)]
pub struct RawInfo {
    pub depth: u8,
    pub score: Score,
    pub pv: Vec<RawMove>,
    pub multipv_index: u8,
    pub nodes: u64,
    pub nps: u64,
    pub time_ms: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParsedLine {
    Info(RawInfo),
    BestMove(RawMove),
    /// id, uciok, readyok, option lines etc. Caller decides what (if anything) to do with them.
    Other(String),
    /// Could not parse. Caller logs and continues.
    Malformed(String),
}

pub fn parse_uci_line(line: &str) -> ParsedLine {
    let line = line.trim();
    if line.is_empty() { return ParsedLine::Other(String::new()); }

    if let Some(rest) = line.strip_prefix("bestmove ") {
        // "bestmove e2e4" or "bestmove e7e8q" or "bestmove (none)"
        let first = rest.split_whitespace().next().unwrap_or("");
        if first == "(none)" { return ParsedLine::Malformed(line.into()); }
        return match parse_raw_move(first) {
            Some(m) => ParsedLine::BestMove(m),
            None => ParsedLine::Malformed(line.into()),
        };
    }

    if let Some(rest) = line.strip_prefix("info ") {
        return parse_info(rest).map(ParsedLine::Info).unwrap_or_else(|| ParsedLine::Other(line.into()));
    }

    ParsedLine::Other(line.into())
}

fn parse_raw_move(s: &str) -> Option<RawMove> {
    let bytes = s.as_bytes();
    if bytes.len() < 4 || bytes.len() > 5 { return None; }
    let from_file = File::from_index(bytes[0].wrapping_sub(b'a'))?;
    let from_rank = Rank::from_index(bytes[1].wrapping_sub(b'1'))?;
    let to_file   = File::from_index(bytes[2].wrapping_sub(b'a'))?;
    let to_rank   = Rank::from_index(bytes[3].wrapping_sub(b'1'))?;
    let promotion = if bytes.len() == 5 {
        Some(match bytes[4] {
            b'q' => PieceKind::Queen,
            b'r' => PieceKind::Rook,
            b'b' => PieceKind::Bishop,
            b'n' => PieceKind::Knight,
            _ => return None,
        })
    } else { None };
    Some(RawMove {
        from: Square::new(from_file, from_rank),
        to: Square::new(to_file, to_rank),
        promotion,
    })
}

fn parse_info(rest: &str) -> Option<RawInfo> {
    // Tokenize and walk: info has key/value pairs except `pv` and `score` which have multi-token values.
    let mut depth: Option<u8> = None;
    let mut score: Option<Score> = None;
    let mut multipv_index: u8 = 1; // default
    let mut nodes: u64 = 0;
    let mut nps: u64 = 0;
    let mut time_ms: u32 = 0;
    let mut pv: Vec<RawMove> = Vec::new();

    let toks: Vec<&str> = rest.split_whitespace().collect();
    let mut i = 0usize;
    while i < toks.len() {
        match toks[i] {
            "depth" => {
                depth = toks.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "multipv" => {
                multipv_index = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(1);
                i += 2;
            }
            "nodes" => {
                nodes = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "nps" => {
                nps = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "time" => {
                time_ms = toks.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(0);
                i += 2;
            }
            "score" => {
                let kind = toks.get(i + 1).copied().unwrap_or("");
                let value: i32 = toks.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(0);
                score = Some(match kind {
                    "cp"   => Score::Cp(value),
                    "mate" => Score::Mate(value as i8),
                    _ => return None,
                });
                i += 3;
                // Skip optional "lowerbound"/"upperbound" qualifiers.
                while let Some(&t) = toks.get(i) {
                    if t == "lowerbound" || t == "upperbound" { i += 1; } else { break; }
                }
            }
            "pv" => {
                i += 1;
                while let Some(&t) = toks.get(i) {
                    if let Some(m) = parse_raw_move(t) {
                        pv.push(m);
                        i += 1;
                    } else { break; }
                }
            }
            _ => { i += 1; } // Skip unknown tokens (currmove, hashfull, tbhits, etc.)
        }
    }

    Some(RawInfo {
        depth: depth?,
        score: score?,
        pv,
        multipv_index,
        nodes,
        nps,
        time_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chess_core::prelude::*;

    fn sq(file: File, rank: Rank) -> Square { Square::new(file, rank) }

    #[test]
    fn parses_bestmove_quiet() {
        let p = parse_uci_line("bestmove e2e4");
        assert_eq!(p, ParsedLine::BestMove(RawMove {
            from: sq(File::E, Rank::Two),
            to:   sq(File::E, Rank::Four),
            promotion: None,
        }));
    }

    #[test]
    fn parses_bestmove_with_promotion() {
        let p = parse_uci_line("bestmove e7e8q");
        let RawMove { promotion, .. } = match p { ParsedLine::BestMove(m) => m, _ => panic!() };
        assert_eq!(promotion, Some(PieceKind::Queen));
    }

    #[test]
    fn parses_bestmove_with_underpromotion() {
        for (ch, want) in [('r', PieceKind::Rook), ('b', PieceKind::Bishop), ('n', PieceKind::Knight)] {
            let line = format!("bestmove e7e8{ch}");
            let m = match parse_uci_line(&line) { ParsedLine::BestMove(m) => m, _ => panic!() };
            assert_eq!(m.promotion, Some(want));
        }
    }

    #[test]
    fn bestmove_none_is_malformed() {
        let p = parse_uci_line("bestmove (none)");
        assert!(matches!(p, ParsedLine::Malformed(_)));
    }

    #[test]
    fn parses_simple_info_line() {
        let line = "info depth 1 seldepth 1 multipv 1 score cp -25 nodes 21 nps 21000 time 1 pv e7e5";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, p => panic!("{p:?}") };
        assert_eq!(info.depth, 1);
        assert_eq!(info.score, Score::Cp(-25));
        assert_eq!(info.multipv_index, 1);
        assert_eq!(info.nodes, 21);
        assert_eq!(info.nps, 21000);
        assert_eq!(info.time_ms, 1);
        assert_eq!(info.pv.len(), 1);
        assert_eq!(info.pv[0].from, sq(File::E, Rank::Seven));
        assert_eq!(info.pv[0].to,   sq(File::E, Rank::Five));
    }

    #[test]
    fn parses_info_with_multipv_3() {
        let line = "info depth 14 multipv 3 score cp 28 nodes 50000 nps 100000 time 500 pv e2e4 e7e5";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.multipv_index, 3);
        assert_eq!(info.pv.len(), 2);
    }

    #[test]
    fn parses_mate_score() {
        let line = "info depth 12 multipv 1 score mate 3 nodes 1000 nps 10000 time 100 pv d1h5 g7g6";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.score, Score::Mate(3));
    }

    #[test]
    fn parses_mate_score_negative() {
        let line = "info depth 5 multipv 1 score mate -2 nodes 100 nps 1000 time 10 pv h2h3";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.score, Score::Mate(-2));
    }

    #[test]
    fn skips_lowerbound_upperbound_qualifiers() {
        let line = "info depth 8 multipv 1 score cp 50 lowerbound nodes 100 nps 1000 time 5 pv e2e4";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.score, Score::Cp(50));
        assert_eq!(info.nodes, 100);
    }

    #[test]
    fn skips_unknown_tokens_like_currmove() {
        let line = "info depth 10 currmove e2e4 currmovenumber 1 score cp 20 nodes 1000 nps 5000 time 200 pv e2e4 e7e5 g1f3";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.depth, 10);
        assert_eq!(info.pv.len(), 3);
    }

    #[test]
    fn pv_stops_at_first_non_move_token() {
        // PV must be the LAST key per UCI; non-move tokens after it shouldn't appear in real output,
        // but our parser stops at the first non-move regardless.
        let line = "info depth 4 multipv 1 score cp 0 nodes 10 nps 100 time 1 pv e2e4 notamove";
        let info = match parse_uci_line(line) { ParsedLine::Info(i) => i, _ => panic!() };
        assert_eq!(info.pv.len(), 1);
    }

    #[test]
    fn missing_depth_or_score_yields_other() {
        // Info without depth is non-actionable; we tolerate by treating it as Other.
        let p = parse_uci_line("info string Some informational text");
        assert!(matches!(p, ParsedLine::Other(_)));
    }

    #[test]
    fn uciok_and_readyok_are_other() {
        assert!(matches!(parse_uci_line("uciok"),   ParsedLine::Other(_)));
        assert!(matches!(parse_uci_line("readyok"), ParsedLine::Other(_)));
    }

    #[test]
    fn id_lines_are_other() {
        assert!(matches!(parse_uci_line("id name Stockfish 17"), ParsedLine::Other(_)));
        assert!(matches!(parse_uci_line("id author the Stockfish developers"), ParsedLine::Other(_)));
    }

    #[test]
    fn option_lines_are_other() {
        let line = "option name Hash type spin default 16 min 1 max 33554432";
        assert!(matches!(parse_uci_line(line), ParsedLine::Other(_)));
    }

    #[test]
    fn empty_line_is_other() {
        assert!(matches!(parse_uci_line(""), ParsedLine::Other(_)));
        assert!(matches!(parse_uci_line("   "), ParsedLine::Other(_)));
    }
}
