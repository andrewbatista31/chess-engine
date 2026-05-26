<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { uiStore } from "../stores/ui.svelte.ts";
  import { tauri } from "../tauri.ts";
  import { parseBoard } from "./fen-board.ts";
  import Piece from "./Piece.svelte";

  const SQUARE_SIZE = 64;
  const BOARD_PX = SQUARE_SIZE * 8;
  const board = $derived(parseBoard(gameStore.currentFen));
  const sideToMove = $derived(gameStore.currentFen.split(" ")[1]);

  let svgEl: SVGSVGElement | undefined = $state();
  let pointerDownOn: { sq: string; piece: string; x: number; y: number } | null = null;
  let didDrag = false;

  function squareName(rankIdx: number, fileIdx: number): string {
    return "abcdefgh"[fileIdx] + (8 - rankIdx).toString();
  }
  function squareFromXY(x: number, y: number): string | null {
    if (x < 0 || y < 0 || x >= BOARD_PX || y >= BOARD_PX) return null;
    const fileIdx = Math.floor(x / SQUARE_SIZE);
    const rankIdx = Math.floor(y / SQUARE_SIZE);
    return squareName(rankIdx, fileIdx);
  }
  function isOwnPiece(piece: string): boolean {
    if (piece === ".") return false;
    const isWhite = piece === piece.toUpperCase();
    return (isWhite && sideToMove === "w") || (!isWhite && sideToMove === "b");
  }
  function localXY(e: PointerEvent): { x: number; y: number } {
    if (!svgEl) return { x: 0, y: 0 };
    const r = svgEl.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  async function onPointerDown(e: PointerEvent, rankIdx: number, fileIdx: number, piece: string) {
    const sq = squareName(rankIdx, fileIdx);
    const { x, y } = localXY(e);

    if (uiStore.selectedSquare && uiStore.legalTargets.includes(sq)) {
      const mv = uiStore.findMove(sq);
      if (mv) {
        await gameStore.makeMove(mv);
        uiStore.clearSelection();
        return;
      }
    }

    if (isOwnPiece(piece)) {
      pointerDownOn = { sq, piece, x, y };
      didDrag = false;
      const all = await tauri.legalMoves(gameStore.currentFen);
      const fromHere = all.filter((m) => m.from === sq);
      if (fromHere.length > 0) {
        uiStore.selectSquare(sq, fromHere);
      }
      svgEl?.setPointerCapture(e.pointerId);
    } else {
      uiStore.clearSelection();
    }
  }

  function onPointerMove(e: PointerEvent) {
    if (!pointerDownOn) return;
    const { x, y } = localXY(e);
    const dx = x - pointerDownOn.x;
    const dy = y - pointerDownOn.y;
    if (!didDrag && Math.hypot(dx, dy) > 5) {
      didDrag = true;
      uiStore.startDrag(pointerDownOn.sq, x, y);
    } else if (didDrag) {
      uiStore.updateDrag(x, y);
    }
  }

  async function onPointerUp(e: PointerEvent) {
    if (!pointerDownOn) return;
    const { x, y } = localXY(e);
    const downSq = pointerDownOn.sq;
    pointerDownOn = null;
    if (didDrag) {
      const targetSq = squareFromXY(x, y);
      uiStore.endDrag();
      if (targetSq && targetSq !== downSq && uiStore.legalTargets.includes(targetSq)) {
        const mv = uiStore.findMove(targetSq);
        if (mv) {
          await gameStore.makeMove(mv);
        }
      }
      uiStore.clearSelection();
    }
    // If !didDrag, selection from onPointerDown stays — user will click target next.
  }
</script>

<svg
  bind:this={svgEl}
  width={BOARD_PX}
  height={BOARD_PX}
  viewBox="0 0 {BOARD_PX} {BOARD_PX}"
  class="board"
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
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
        onpointerdown={(e) => onPointerDown(e, rankIdx, fileIdx, piece)}
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
      {#if piece !== "." && !(uiStore.dragging && uiStore.dragging.from === sq)}
        <svg {x} {y} width={SQUARE_SIZE} height={SQUARE_SIZE} pointer-events="none">
          <Piece {piece} size={SQUARE_SIZE} />
        </svg>
      {/if}
    {/each}
  {/each}
  {#if uiStore.dragging}
    {@const dragPiece = board[8 - parseInt(uiStore.dragging.from[1])][("abcdefgh").indexOf(uiStore.dragging.from[0])]}
    {#if dragPiece !== "."}
      <svg
        x={uiStore.dragging.x - SQUARE_SIZE / 2}
        y={uiStore.dragging.y - SQUARE_SIZE / 2}
        width={SQUARE_SIZE}
        height={SQUARE_SIZE}
        pointer-events="none"
      >
        <Piece piece={dragPiece} size={SQUARE_SIZE} />
      </svg>
    {/if}
  {/if}
</svg>

<style>
  .board { display: block; user-select: none; touch-action: none; }
</style>
