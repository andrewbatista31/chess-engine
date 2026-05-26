import { describe, it, expect } from "vitest";
import { parseBoard } from "./fen-board.ts";

describe("parseBoard", () => {
  it("parses the starting position", () => {
    const fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    const b = parseBoard(fen);
    expect(b).toHaveLength(8);
    expect(b[0]).toEqual(["r","n","b","q","k","b","n","r"]);
    expect(b[1].every((p) => p === "p")).toBe(true);
    expect(b[2].every((p) => p === ".")).toBe(true);
    expect(b[6].every((p) => p === "P")).toBe(true);
    expect(b[7]).toEqual(["R","N","B","Q","K","B","N","R"]);
  });

  it("handles empty squares encoded as digits", () => {
    const b = parseBoard("8/8/8/3k4/3K4/8/8/8 w - - 0 1");
    expect(b[3]).toEqual([".",".",".","k",".",".",".","."]);
    expect(b[4]).toEqual([".",".",".","K",".",".",".","."]);
  });
});
