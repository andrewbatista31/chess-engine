export type EngineSlot =
  | { kind: "Human"; skill: number }
  | { kind: "Stockfish"; skill: number };

class EnginesStore {
  white: EngineSlot = $state({ kind: "Human", skill: 10 });
  black: EngineSlot = $state({ kind: "Stockfish", skill: 10 });

  engineFor(side: "w" | "b"): EngineSlot | null {
    const slot = side === "w" ? this.white : this.black;
    return slot.kind === "Stockfish" ? slot : null;
  }

  anyEngine: boolean = $derived(
    this.white.kind === "Stockfish" || this.black.kind === "Stockfish"
  );

  engineForSideToMove(fen: string): EngineSlot | null {
    const side = fen.split(" ")[1] as "w" | "b";
    return this.engineFor(side);
  }
}

export const enginesStore = new EnginesStore();
