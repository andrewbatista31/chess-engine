<script lang="ts">
  import Board from "./lib/board/Board.svelte";
  import HistoryPanel from "./lib/panels/HistoryPanel.svelte";
  import Toolbar from "./lib/panels/Toolbar.svelte";
</script>

<div class="app">
  <header class="toolbar">
    <Toolbar />
  </header>
  <section class="board-area">
    <Board />
  </section>
  <HistoryPanel />
</div>

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    grid-template-columns: 1fr 280px;
    height: 100vh;
  }
  .toolbar {
    grid-column: 1 / span 2;
    background: #1f1f1f;
    border-bottom: 1px solid #444;
    height: 48px;
  }
  .board-area {
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: auto;
  }
</style>
