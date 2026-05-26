import { tauri, type MoveDto, type MoveEntry } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  /** The FEN we started this game from. Undo can walk back to (but not past) this. */
  startingFen = $state(STARTING_FEN);
  currentFen = $state(STARTING_FEN);
  history: MoveEntry[] = $state([]);
  cursor = $state(0);

  private fenAt(c: number): string {
    return c === 0 ? this.startingFen : this.history[c - 1].fen_after;
  }

  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    this.history = this.history.slice(0, this.cursor);
    this.history.push({
      san: result.san,
      fen_after: result.new_fen,
      outcome: result.outcome,
    });
    this.cursor = this.history.length;
    this.currentFen = result.new_fen;
  }

  undo(): void {
    if (this.cursor === 0) return;
    this.cursor -= 1;
    this.currentFen = this.fenAt(this.cursor);
  }

  redo(): void {
    if (this.cursor >= this.history.length) return;
    this.cursor += 1;
    this.currentFen = this.fenAt(this.cursor);
  }

  async loadFen(fen: string): Promise<void> {
    const ok = await tauri.validateFen(fen);
    if (!ok) throw new Error("Invalid FEN");
    this.startingFen = fen;
    this.currentFen = fen;
    this.history = [];
    this.cursor = 0;
  }

  reset(): void {
    this.startingFen = STARTING_FEN;
    this.currentFen = STARTING_FEN;
    this.history = [];
    this.cursor = 0;
  }
}

export const gameStore = new GameStore();
