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

describe("gameStore undo/redo", () => {
  beforeEach(() => {
    gameStore.currentFen = STARTING_FEN;
    gameStore.history = [];
    gameStore.cursor = 0;
  });

  it("undo decrements cursor and restores prior FEN", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    expect(gameStore.cursor).toBe(1);
    gameStore.undo();
    expect(gameStore.cursor).toBe(0);
    expect(gameStore.currentFen).toBe(STARTING_FEN);
  });

  it("redo restores the next FEN after undo", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    gameStore.undo();
    gameStore.redo();
    expect(gameStore.cursor).toBe(1);
    expect(gameStore.currentFen).toBe("after-e2e4");
  });

  it("undo at cursor 0 is a no-op", () => {
    gameStore.undo();
    expect(gameStore.cursor).toBe(0);
    expect(gameStore.currentFen).toBe(STARTING_FEN);
  });

  it("redo at end-of-history is a no-op", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    gameStore.redo();
    expect(gameStore.cursor).toBe(1);
  });

  it("a new move after undo drops the redo branch", async () => {
    await gameStore.makeMove({ from: "e2", to: "e4", promotion: null });
    await gameStore.makeMove({ from: "e7", to: "e5", promotion: null });
    gameStore.undo();
    await gameStore.makeMove({ from: "d7", to: "d5", promotion: null });
    expect(gameStore.history).toHaveLength(2);
    expect(gameStore.history[1].san).toBe("d7d5");
  });
});
