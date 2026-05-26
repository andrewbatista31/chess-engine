<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { tauri } from "../tauri.ts";

  let fenInput = $state(gameStore.currentFen);
  let fenValid = $state(true);
  let fenChecking = $state(false);

  const canUndo = $derived(gameStore.cursor > 0);
  const canRedo = $derived(gameStore.cursor < gameStore.history.length);

  async function checkFen(s: string) {
    fenChecking = true;
    try {
      fenValid = await tauri.validateFen(s);
    } finally {
      fenChecking = false;
    }
  }

  async function loadFen() {
    if (!fenValid) return;
    try {
      await gameStore.loadFen(fenInput);
    } catch (e) {
      fenValid = false;
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "z") { e.preventDefault(); gameStore.undo(); }
    else if (e.ctrlKey && e.key === "y") { e.preventDefault(); gameStore.redo(); }
  }
</script>

<svelte:window on:keydown={onKey} />

<div class="toolbar-inner">
  <input
    type="text"
    class="fen"
    class:invalid={!fenValid}
    bind:value={fenInput}
    oninput={() => checkFen(fenInput)}
    placeholder="Paste a FEN…"
    spellcheck="false"
  />
  <button onclick={loadFen} disabled={!fenValid || fenChecking}>Load</button>
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
  .placeholder { font-style: italic; color: #777; font-size: 12px; }
  .fen {
    flex: 1;
    max-width: 600px;
    background: #2a2a2a;
    color: #eee;
    border: 1px solid #555;
    padding: 6px 8px;
    border-radius: 4px;
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 12px;
  }
  .fen.invalid { border-color: #c0392b; }
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
