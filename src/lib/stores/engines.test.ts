import { describe, it, expect, beforeEach } from "vitest";
import { enginesStore } from "./engines.svelte.ts";

describe("enginesStore", () => {
  beforeEach(() => {
    enginesStore.white = { kind: "Human", skill: 10 };
    enginesStore.black = { kind: "Stockfish", skill: 10 };
  });

  it("default is white=Human, black=Stockfish(10)", () => {
    expect(enginesStore.white.kind).toBe("Human");
    expect(enginesStore.black.kind).toBe("Stockfish");
    expect(enginesStore.black.skill).toBe(10);
  });

  it("engineFor returns the slot iff Stockfish, else null", () => {
    expect(enginesStore.engineFor("w")).toBeNull();
    expect(enginesStore.engineFor("b")).toEqual({ kind: "Stockfish", skill: 10 });
  });

  it("anyEngine is true if either side is Stockfish", () => {
    expect(enginesStore.anyEngine).toBe(true);
    enginesStore.black = { kind: "Human", skill: 10 };
    expect(enginesStore.anyEngine).toBe(false);
    enginesStore.white = { kind: "Stockfish", skill: 5 };
    expect(enginesStore.anyEngine).toBe(true);
  });

  it("engineForSideToMove parses FEN side and returns the right slot", () => {
    const whitesTurn = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const blacksTurn = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1";
    expect(enginesStore.engineForSideToMove(whitesTurn)).toBeNull(); // white is Human
    expect(enginesStore.engineForSideToMove(blacksTurn)?.kind).toBe("Stockfish");
  });
});
