<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { uiStore } from "../stores/ui.svelte.ts";
  import { tauri } from "../tauri.ts";
  import { parseBoard } from "./fen-board.ts";
  import Piece from "./Piece.svelte";

  const SQUARE_SIZE = 64;
  const board = $derived(parseBoard(gameStore.currentFen));
  const sideToMove = $derived(gameStore.currentFen.split(" ")[1]);

  function squareName(rankIdx: number, fileIdx: number): string {
    return "abcdefgh"[fileIdx] + (8 - rankIdx).toString();
  }
  function isOwnPiece(piece: string): boolean {
    if (piece === ".") return false;
    const isWhite = piece === piece.toUpperCase();
    return (isWhite && sideToMove === "w") || (!isWhite && sideToMove === "b");
  }

  async function onSquareClick(rankIdx: number, fileIdx: number, piece: string) {
    const sq = squareName(rankIdx, fileIdx);

    if (uiStore.selectedSquare && uiStore.legalTargets.includes(sq)) {
      const mv = uiStore.findMove(sq);
      if (mv) {
        await gameStore.makeMove(mv);
        uiStore.clearSelection();
        return;
      }
    }

    if (isOwnPiece(piece)) {
      const all = await tauri.legalMoves(gameStore.currentFen);
      const fromHere = all.filter((m) => m.from === sq);
      if (fromHere.length > 0) {
        uiStore.selectSquare(sq, fromHere);
        return;
      }
    }

    uiStore.clearSelection();
  }
</script>

<svg
  width={SQUARE_SIZE * 8}
  height={SQUARE_SIZE * 8}
  viewBox="0 0 {SQUARE_SIZE * 8} {SQUARE_SIZE * 8}"
  class="board"
>
  {#each board as row, rankIdx}
    {#each row as piece, fileIdx}
      {@const isLight = (rankIdx + fileIdx) % 2 === 0}
      {@const sq = squareName(rankIdx, fileIdx)}
      {@const x = fileIdx * SQUARE_SIZE}
      {@const y = rankIdx * SQUARE_SIZE}
      {@const isSelected = uiStore.selectedSquare === sq}
      {@const isTarget = uiStore.legalTargets.includes(sq)}
      <rect
        {x} {y}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        fill={isLight ? "#f0d9b5" : "#b58863"}
        onclick={() => onSquareClick(rankIdx, fileIdx, piece)}
        style:cursor="pointer"
      />
      {#if isSelected}
        <rect {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} fill="rgba(255,255,0,0.35)" pointer-events="none" />
      {/if}
      {#if isTarget}
        <circle
          cx={x + SQUARE_SIZE / 2}
          cy={y + SQUARE_SIZE / 2}
          r={SQUARE_SIZE * (piece === "." ? 0.15 : 0.45)}
          fill={piece === "." ? "rgba(0,0,0,0.35)" : "none"}
          stroke={piece === "." ? "none" : "rgba(0,0,0,0.55)"}
          stroke-width={piece === "." ? 0 : 4}
          pointer-events="none"
        />
      {/if}
      {#if piece !== "."}
        <svg {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} pointer-events="none">
          <Piece {piece} size={SQUARE_SIZE} />
        </svg>
      {/if}
    {/each}
  {/each}
</svg>

<style>
  .board { display: block; user-select: none; }
</style>
