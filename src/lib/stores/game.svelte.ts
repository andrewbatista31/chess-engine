import { tauri, type MoveDto, type MoveEntry } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  currentFen = $state(STARTING_FEN);
  history: MoveEntry[] = $state([]);
  cursor = $state(0);

  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    // Drop any pending redo branch.
    this.history = this.history.slice(0, this.cursor);
    this.history.push({
      san: result.san,
      fen_after: result.new_fen,
      outcome: result.outcome,
    });
    this.cursor = this.history.length;
    this.currentFen = result.new_fen;
  }
}

export const gameStore = new GameStore();
