<script lang="ts">
  import { analysisStore } from "../stores/analysis.svelte.ts";

  function formatScore(score: { kind: "Cp" | "Mate"; value: number }): string {
    if (score.kind === "Mate") {
      return `M${Math.abs(score.value)}${score.value < 0 ? " (lost)" : ""}`;
    }
    const cp = score.value / 100;
    return cp >= 0 ? `+${cp.toFixed(2)}` : cp.toFixed(2);
  }

  const sorted = $derived.by(() => {
    return Array.from(analysisStore.lines.values()).sort(
      (a, b) => a.multipv_index - b.multipv_index
    );
  });
</script>

<aside class="analysis">
  <h2>Analysis</h2>
  {#if sorted.length === 0}
    <p class="empty">No analysis yet.</p>
  {:else}
    <ol>
      {#each sorted as info}
        <li>
          <div class="row-1">
            <span class="san">{info.pv_san[0] ?? "—"}</span>
            <span class="score">{formatScore(info.score)}</span>
            <span class="depth">d{info.depth}</span>
          </div>
          {#if info.pv_san.length > 1}
            <div class="pv">{info.pv_san.slice(0, 5).join(" ")}{info.pv_san.length > 5 ? "…" : ""}</div>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</aside>

<style>
  .analysis {
    background: #1a1a1a;
    color: #e8e8e8;
    padding: 12px;
    border-top: 1px solid #333;
    font-family: ui-monospace, "Cascadia Code", monospace;
    overflow-y: auto;
    min-height: 120px;
  }
  .analysis h2 {
    margin: 0 0 8px;
    font-size: 14px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #bbb;
  }
  .empty { color: #888; font-style: italic; font-size: 12px; }
  ol { list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 6px; }
  .row-1 {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 8px;
    align-items: baseline;
    font-size: 13px;
  }
  .san { color: #fff; font-weight: 500; }
  .score { color: #4caf50; font-variant-numeric: tabular-nums; }
  .depth { color: #777; font-size: 10px; }
  .pv { color: #999; font-size: 11px; padding-left: 8px; }
</style>
