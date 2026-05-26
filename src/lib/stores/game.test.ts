import { describe, it, expect, beforeEach, vi } from "vitest";

vi.mock("../tauri.ts", () => ({
  tauri: {
    legalMoves: vi.fn(),
    makeMove: vi.fn(async (_fen: string, mv: { from: string; to: string }) => ({
      new_fen: `after-${mv.from}${mv.to}`,
      san: `${mv.from}${mv.to}`,
      outcome: null,
    })),
  },
}));

const { gameStore, STARTING_FEN } = await import("./game.svelte.ts");

describe("gameStore", () => {
  beforeEach(() => {
    gameStore.currentFen = STARTING_FEN;
    gameStore.history = [];
    gameStore.cursor = 0;
  });

  it("updates currentFen and appends to history on makeMove", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    expect(gameStore.currentFen).toBe("after-e2e4");
    expect(gameStore.history).toHaveLength(1);
    expect(gameStore.history[0].san).toBe("e2e4");
    expect(gameStore.history[0].fen_after).toBe("after-e2e4");
    expect(gameStore.cursor).toBe(1);
  });

  it("accumulates multiple moves", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    gameStore.currentFen = "after-e2e4";
    await gameStore.makeMove({ from: "e7", to: "e5", promotion: null });
    expect(gameStore.history).toHaveLength(2);
    expect(gameStore.cursor).toBe(2);
  });
});
