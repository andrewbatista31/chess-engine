import { describe, it, expect, beforeEach } from "vitest";
import { analysisStore } from "./analysis.svelte.ts";

describe("analysisStore", () => {
  beforeEach(() => {
    analysisStore.reset(0);
    analysisStore.searchId = null;
  });

  it("ignores info with stale searchId", () => {
    analysisStore.reset(5);
    analysisStore.applyInfo({
      search_id: 4,
      depth: 1, score: { kind: "Cp", value: 0 }, pv_san: ["e4"],
      multipv_index: 1, nodes: 1, nps: 1, time_ms: 1,
    });
    expect(analysisStore.lines.size).toBe(0);
  });

  it("accepts info with current searchId, keyed by multipv_index", () => {
    analysisStore.reset(7);
    analysisStore.applyInfo({
      search_id: 7,
      depth: 12, score: { kind: "Cp", value: 25 }, pv_san: ["e4", "e5"],
      multipv_index: 1, nodes: 1000, nps: 5000, time_ms: 200,
    });
    analysisStore.applyInfo({
      search_id: 7,
      depth: 12, score: { kind: "Cp", value: 12 }, pv_san: ["d4", "d5"],
      multipv_index: 2, nodes: 1000, nps: 5000, time_ms: 200,
    });
    expect(analysisStore.lines.size).toBe(2);
    expect(analysisStore.lines.get(1)?.pv_san[0]).toBe("e4");
    expect(analysisStore.lines.get(2)?.pv_san[0]).toBe("d4");
  });

  it("evalPercent maps cp=0 to 0.5", () => {
    analysisStore.reset(1);
    analysisStore.applyInfo({
      search_id: 1, depth: 1, score: { kind: "Cp", value: 0 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    expect(analysisStore.evalPercent).toBeCloseTo(0.5, 2);
  });

  it("evalPercent maps cp=+400 to ~0.73 (Lichess sigmoid)", () => {
    analysisStore.reset(2);
    analysisStore.applyInfo({
      search_id: 2, depth: 1, score: { kind: "Cp", value: 400 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    // 2/(1+exp(-1)) - 1 = ~0.4621 → mapped to [0,1]: (0.4621+1)/2 = ~0.731
    expect(analysisStore.evalPercent).toBeCloseTo(0.731, 2);
  });

  it("evalPercent maps Mate(+N) to 1.0 and Mate(-N) to 0.0", () => {
    analysisStore.reset(3);
    analysisStore.applyInfo({
      search_id: 3, depth: 1, score: { kind: "Mate", value: 3 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    expect(analysisStore.evalPercent).toBeCloseTo(1.0, 4);

    analysisStore.reset(4);
    analysisStore.applyInfo({
      search_id: 4, depth: 1, score: { kind: "Mate", value: -2 }, pv_san: [],
      multipv_index: 1, nodes: 0, nps: 0, time_ms: 0,
    });
    expect(analysisStore.evalPercent).toBeCloseTo(0.0, 4);
  });

  it("evalPercent is 0.5 when there's no analysis yet", () => {
    expect(analysisStore.evalPercent).toBeCloseTo(0.5, 4);
  });
});
