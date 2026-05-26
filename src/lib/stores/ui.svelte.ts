import type { MoveDto } from "../tauri.ts";

interface DragState {
  from: string;
  /** Cursor position in board-local coordinates (px from board origin). */
  x: number;
  y: number;
}

class UiStore {
  selectedSquare: string | null = $state(null);
  legalTargets: string[] = $state([]);
  legalByFrom: Map<string, MoveDto[]> = $state(new Map());
  dragging: DragState | null = $state(null);

  selectSquare(sq: string, legalFromHere: MoveDto[]): void {
    this.selectedSquare = sq;
    this.legalTargets = legalFromHere.map((m) => m.to);
    this.legalByFrom = new Map([[sq, legalFromHere]]);
  }

  clearSelection(): void {
    this.selectedSquare = null;
    this.legalTargets = [];
    this.legalByFrom = new Map();
    this.dragging = null;
  }

  startDrag(from: string, x: number, y: number): void {
    this.dragging = { from, x, y };
  }

  updateDrag(x: number, y: number): void {
    if (this.dragging) this.dragging = { ...this.dragging, x, y };
  }

  endDrag(): void {
    this.dragging = null;
  }

  findMove(to: string): MoveDto | null {
    if (!this.selectedSquare) return null;
    const candidates = this.legalByFrom.get(this.selectedSquare) ?? [];
    return candidates.find((m) => m.to === to && (m.promotion === null || m.promotion === "Q")) ?? null;
  }
}

export const uiStore = new UiStore();
