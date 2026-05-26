import { tauri, type MoveDto, type MoveEntry } from "../tauri.ts";

export const STARTING_FEN =
  "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

class GameStore {
  /** The FEN we started this game from. Undo can walk back to (but not past) this. */
  startingFen = $state(STARTING_FEN);
  currentFen = $state(STARTING_FEN);
  history: MoveEntry[] = $state([]);
  cursor = $state(0);
  outcome = $derived(
    this.cursor > 0 ? this.history[this.cursor - 1].outcome : null
  );
  tags: Record<string, string> = $state({});

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
      mv,
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
    this.tags = {};
  }

  async savePgn(path: string): Promise<void> {
    const moves = this.history.slice(0, this.cursor).map((e) => e.mv);
    const text = await tauri.serializePgn(moves, this.tags);
    await tauri.savePgnFile(path, text);
  }

  async loadPgn(text: string): Promise<void> {
    const game = await tauri.parsePgn(text);
    this.startingFen = STARTING_FEN;
    this.currentFen = game.final_fen;
    this.tags = game.tags;
    this.history = game.moves.map((m) => ({
      san: m.san,
      fen_after: m.fen_after,
      outcome: m.outcome,
      mv: m.mv,
    }));
    this.cursor = this.history.length;
  }
}

export const gameStore = new GameStore();
