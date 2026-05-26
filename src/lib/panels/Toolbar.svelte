<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";

  const canUndo = $derived(gameStore.cursor > 0);
  const canRedo = $derived(gameStore.cursor < gameStore.history.length);

  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "z") {
      e.preventDefault();
      gameStore.undo();
    } else if (e.ctrlKey && e.key === "y") {
      e.preventDefault();
      gameStore.redo();
    }
  }
</script>

<svelte:window on:keydown={onKey} />

<div class="toolbar-inner">
  <div class="placeholder">FEN input arrives in Task 8</div>
  <div class="spacer"></div>
  <button onclick={() => gameStore.undo()} disabled={!canUndo}>← Undo</button>
  <button onclick={() => gameStore.redo()} disabled={!canRedo}>Redo →</button>
  <div class="placeholder pgn-slot">PGN buttons arrive in Task 9</div>
</div>

<style>
  .toolbar-inner {
    height: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    color: #ddd;
  }
  .spacer { flex: 1; }
  .placeholder {
    font-style: italic;
    color: #777;
    font-size: 12px;
  }
  button {
    background: #333;
    border: 1px solid #555;
    color: #eee;
    padding: 6px 12px;
    border-radius: 4px;
    cursor: pointer;
  }
  button:hover:not(:disabled) { background: #3d3d3d; }
  button:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
