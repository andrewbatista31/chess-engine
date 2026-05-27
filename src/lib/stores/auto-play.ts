import type { EngineSlot } from "./engines.svelte.ts";

/**
 * Given the current FEN and the engine slot mapping, returns true iff the side-to-move
 * is a Stockfish engine and therefore the bestmove event should be auto-played.
 */
export function shouldEnginePlay(
  fen: string,
  white: EngineSlot,
  black: EngineSlot,
): boolean {
  const side = fen.split(" ")[1];
  if (side !== "w" && side !== "b") return false;
  const slot = side === "w" ? white : black;
  return slot.kind === "Stockfish";
}
