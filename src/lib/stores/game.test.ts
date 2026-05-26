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

describe("gameStore.makeMove", () => {
  beforeEach(() => {
    gameStore.currentFen = STARTING_FEN;
  });

  it("updates currentFen to the result returned by the backend", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    expect(gameStore.currentFen).toBe("after-e2e4");
  });
});
