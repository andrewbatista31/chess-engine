<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import Board from "./lib/board/Board.svelte";
  import HistoryPanel from "./lib/panels/HistoryPanel.svelte";
  import Toolbar from "./lib/panels/Toolbar.svelte";
  import GameOverBanner from "./lib/panels/GameOverBanner.svelte";
  import { gameStore } from "./lib/stores/game.svelte.ts";
  import { enginesStore } from "./lib/stores/engines.svelte.ts";
  import { analysisStore } from "./lib/stores/analysis.svelte.ts";
  import { tauri, type AnalysisInfoEvent, type EngineBestMoveEvent } from "./lib/tauri.ts";

  let unlistenInfo: UnlistenFn | null = null;
  let unlistenBest: UnlistenFn | null = null;

  onMount(async () => {
    unlistenInfo = await listen<AnalysisInfoEvent>("analysis_info", (e) => {
      analysisStore.applyInfo(e.payload);
    });
    unlistenBest = await listen<EngineBestMoveEvent>("engine_bestmove", (e) => {
      // Auto-play wiring lands in Task 10. For now just console-log.
      console.log("[engine_bestmove]", e.payload);
    });
  });

  onDestroy(() => {
    unlistenInfo?.();
    unlistenBest?.();
  });

  // Reactive analysis trigger: any FEN change, if any engine is active, kick off a new search.
  $effect(() => {
    const fen = gameStore.currentFen;
    if (!enginesStore.anyEngine) return;
    const skill =
      enginesStore.white.kind === "Stockfish" ? enginesStore.white.skill : enginesStore.black.skill;
    tauri
      .startAnalysis(fen, skill, 1000, 3)
      .then((id) => analysisStore.reset(id))
      .catch((err) => console.error("[startAnalysis]", err));
  });
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
<GameOverBanner />

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
