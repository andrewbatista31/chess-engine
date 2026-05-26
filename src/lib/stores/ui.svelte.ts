import type { MoveDto } from "../tauri.ts";

class UiStore {
  selectedSquare: string | null = $state(null);
  legalTargets: string[] = $state([]);
  legalByFrom: Map<string, MoveDto[]> = $state(new Map());

  selectSquare(sq: string, legalFromHere: MoveDto[]): void {
    this.selectedSquare = sq;
    this.legalTargets = legalFromHere.map((m) => m.to);
    this.legalByFrom = new Map([[sq, legalFromHere]]);
  }

  clearSelection(): void {
    this.selectedSquare = null;
    this.legalTargets = [];
    this.legalByFrom = new Map();
  }

  findMove(to: string): MoveDto | null {
    if (!this.selectedSquare) return null;
    const candidates = this.legalByFrom.get(this.selectedSquare) ?? [];
    const exact = candidates.find((m) => m.to === to && (m.promotion === null || m.promotion === "Q"));
    return exact ?? null;
  }
}

export const uiStore = new UiStore();
