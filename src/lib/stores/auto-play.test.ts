import { describe, it, expect } from "vitest";
import { shouldEnginePlay } from "./auto-play.ts";

const WHITES_TURN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
const BLACKS_TURN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
const HUMAN = { kind: "Human", skill: 10 } as const;
const SF = { kind: "Stockfish", skill: 10 } as const;

describe("shouldEnginePlay", () => {
  it("returns true when side-to-move is Stockfish", () => {
    expect(shouldEnginePlay(WHITES_TURN, SF, HUMAN)).toBe(true);
    expect(shouldEnginePlay(BLACKS_TURN, HUMAN, SF)).toBe(true);
  });

  it("returns false when side-to-move is Human", () => {
    expect(shouldEnginePlay(WHITES_TURN, HUMAN, SF)).toBe(false);
    expect(shouldEnginePlay(BLACKS_TURN, SF, HUMAN)).toBe(false);
  });

  it("returns false when both sides are Human", () => {
    expect(shouldEnginePlay(WHITES_TURN, HUMAN, HUMAN)).toBe(false);
    expect(shouldEnginePlay(BLACKS_TURN, HUMAN, HUMAN)).toBe(false);
  });

  it("returns true on both turns when both sides are Stockfish (engine-vs-engine future)", () => {
    expect(shouldEnginePlay(WHITES_TURN, SF, SF)).toBe(true);
    expect(shouldEnginePlay(BLACKS_TURN, SF, SF)).toBe(true);
  });

  it("returns false for malformed FEN", () => {
    expect(shouldEnginePlay("garbage", SF, SF)).toBe(false);
  });
});
