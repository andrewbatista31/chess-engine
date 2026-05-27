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
    // Lichess-style sigmoid: 2/(1+exp(-cp/400)) - 1 in (-1, 1).
    // Score is from side-to-move's POV; UI convention: positive = White advantage.
    // The store doesn't know side-to-move on its own — callers should pass cp already
    // flipped from White's perspective. For Plan 3 we accept the side-to-move convention
    // and the EvalBar inverts the bar when it's black to move.
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
