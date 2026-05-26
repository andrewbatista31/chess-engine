import { invoke } from "@tauri-apps/api/core";

export interface MoveDto {
  from: string;
  to: string;
  promotion: "Q" | "R" | "B" | "N" | null;
}

export interface OutcomeDto {
  kind: "Checkmate" | "Stalemate" | "FiftyMove" | "InsufficientMaterial";
  winner: "White" | "Black" | null;
}

export interface MakeMoveResult {
  new_fen: string;
  san: string;
  outcome: OutcomeDto | null;
}

export interface MoveEntry {
  san: string;
  fen_after: string;
  outcome: OutcomeDto | null;
}

export interface GameDto {
  tags: Record<string, string>;
  moves: MoveEntry[];
  result: string;
  final_fen: string;
}

export const tauri = {
  legalMoves(fen: string): Promise<MoveDto[]> {
    return invoke("legal_moves", { fen });
  },
  makeMove(fen: string, mv: MoveDto): Promise<MakeMoveResult> {
    return invoke("make_move", { fen, mv });
  },
};
