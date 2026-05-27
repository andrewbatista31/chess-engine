import type { AnalysisInfoEvent } from "../tauri.ts";

class AnalysisStore {
  searchId: number | null = $state(null);
  lines: Map<number, AnalysisInfoEvent> = $state(new Map());

  evalPercent: number = $derived.by(() => {
    const best = this.lines.get(1);
    if (!best) return 0.5;
    if (best.score.kind === "Mate") {
      return best.score.value > 0 ? 1.0 : 0.0;
    }
    // Lichess-style sigmoid: 2/(1+exp(-cp/400)) - 1 in (-1, 1), mapped to (0, 1).
    // Score is always from White's perspective (the backend pre-flips for Black-to-move positions),
    // so positive cp → bar fills upward with white.
    const cp = best.score.value;
    const v = 2 / (1 + Math.exp(-cp / 400)) - 1; // (-1, 1)
    return (v + 1) / 2; // (0, 1)
  });

  applyInfo(info: AnalysisInfoEvent): void {
    if (info.search_id !== this.searchId) return;
    const next = new Map(this.lines);
    next.set(info.multipv_index, info);
    this.lines = next;
  }

  reset(newSearchId: number): void {
    this.searchId = newSearchId;
    this.lines = new Map();
  }
}

export const analysisStore = new AnalysisStore();
