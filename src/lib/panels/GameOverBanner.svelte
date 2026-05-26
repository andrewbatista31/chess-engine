<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";
  import type { OutcomeDto } from "../tauri.ts";

  function label(o: OutcomeDto): string {
    switch (o.kind) {
      case "Checkmate": return `Checkmate — ${o.winner} wins`;
      case "Stalemate": return "Stalemate — draw";
      case "FiftyMove": return "Draw by 50-move rule";
      case "InsufficientMaterial": return "Draw by insufficient material";
    }
  }
</script>

{#if gameStore.outcome}
  <div class="overlay" role="dialog" aria-modal="true">
    <div class="card">
      <h2>{label(gameStore.outcome)}</h2>
      <button onclick={() => gameStore.reset()}>New game</button>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 50;
  }
  .card {
    background: #1f1f1f;
    border: 1px solid #555;
    border-radius: 8px;
    padding: 24px 36px;
    text-align: center;
    color: #eee;
    min-width: 320px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.6);
  }
  .card h2 {
    margin: 0 0 18px;
    font-size: 22px;
  }
  button {
    background: #2e7d32;
    border: none;
    color: white;
    padding: 8px 20px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }
  button:hover { background: #388e3c; }
</style>
