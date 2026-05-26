<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import { tauri } from "../tauri.ts";
  import { open, save } from "@tauri-apps/plugin-dialog";

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

  async function savePgn() {
    const path = await save({
      defaultPath: "game.pgn",
      filters: [{ name: "PGN", extensions: ["pgn"] }],
    });
    if (!path) return;
    try {
      await gameStore.savePgn(path);
    } catch (e) {
      alert(`Could not save: ${e}`);
    }
  }

  async function loadPgn() {
    const path = await open({
      filters: [{ name: "PGN", extensions: ["pgn"] }],
      multiple: false,
    });
    if (!path || Array.isArray(path)) return;
    try {
      const text = await tauri.loadPgnFile(path);
      await gameStore.loadPgn(text);
    } catch (e) {
      alert(`Could not load: ${e}`);
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.ctrlKey && e.key === "z") { e.preventDefault(); gameStore.undo(); }
    else if (e.ctrlKey && e.key === "y") { e.preventDefault(); gameStore.redo(); }
    else if (e.ctrlKey && e.key === "s") { e.preventDefault(); savePgn(); }
    else if (e.ctrlKey && e.key === "o") { e.preventDefault(); loadPgn(); }
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
  <button onclick={savePgn}>Save PGN</button>
  <button onclick={loadPgn}>Load PGN</button>
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
