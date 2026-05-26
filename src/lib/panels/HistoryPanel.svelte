<script lang="ts">
  import { gameStore } from "../stores/game.svelte.ts";

  const pairs = $derived.by(() => {
    const out: Array<{ num: number; white: string; black: string | null }> = [];
    for (let i = 0; i < gameStore.history.length; i += 2) {
      out.push({
        num: i / 2 + 1,
        white: gameStore.history[i].san,
        black: gameStore.history[i + 1]?.san ?? null,
      });
    }
    return out;
  });

  let listEl: HTMLElement | undefined = $state();

  $effect(() => {
    void gameStore.history.length;
    if (listEl) listEl.scrollTop = listEl.scrollHeight;
  });
</script>

<aside class="history" bind:this={listEl}>
  <h2>Moves</h2>
  {#if pairs.length === 0}
    <p class="empty">No moves yet.</p>
  {:else}
    <ol>
      {#each pairs as p}
        <li>
          <span class="num">{p.num}.</span>
          <span class="san">{p.white}</span>
          {#if p.black}<span class="san">{p.black}</span>{/if}
        </li>
      {/each}
    </ol>
  {/if}
</aside>

<style>
  .history {
    background: #1a1a1a;
    color: #e8e8e8;
    padding: 12px;
    overflow-y: auto;
    font-family: ui-monospace, "Cascadia Code", monospace;
    border-left: 1px solid #444;
  }
  .history h2 {
    margin: 0 0 8px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #bbb;
  }
  .empty { color: #888; font-style: italic; }
  ol { list-style: none; padding: 0; margin: 0; }
  li {
    display: grid;
    grid-template-columns: 32px 1fr 1fr;
    gap: 6px;
    padding: 2px 0;
  }
  .num { color: #888; text-align: right; }
  .san { padding: 0 4px; }
</style>
