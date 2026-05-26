import { tauri, type MoveDto } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  currentFen = $state(STARTING_FEN);

  async makeMove(mv: MoveDto): Promise<void> {
    const result = await tauri.makeMove(this.currentFen, mv);
    this.currentFen = result.new_fen;
  }
}

export const gameStore = new GameStore();
