<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { parseBoard } from "./fen-board.ts";
  import Piece from "./Piece.svelte";

  const SQUARE_SIZE = 64;

  const board = $derived(parseBoard(gameStore.currentFen));
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
      {@const x = fileIdx * SQUARE_SIZE}
      {@const y = rankIdx * SQUARE_SIZE}
      <rect
        {x} {y}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        fill={isLight ? "#f0d9b5" : "#b58863"}
      />
      {#if piece !== "."}
        <svg {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE}>
          <Piece {piece} size={SQUARE_SIZE} />
        </svg>
      {/if}
    {/each}
  {/each}
</svg>

<style>
  .board { display: block; user-select: none; }
</style>
