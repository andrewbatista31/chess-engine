<script lang="ts">
  import { analysisStore } from "../stores/analysis.svelte.ts";

  const whitePct = $derived(Math.max(0, Math.min(1, analysisStore.evalPercent)) * 100);

  const label = $derived.by(() => {
    const best = analysisStore.lines.get(1);
    if (!best) return "";
    if (best.score.kind === "Mate") return `M${Math.abs(best.score.value)}`;
    const cp = best.score.value / 100;
    return cp >= 0 ? `+${cp.toFixed(2)}` : cp.toFixed(2);
  });
</script>

<aside class="evalbar" aria-label="Evaluation bar">
  <div class="white-fill" style:height="{whitePct}%"></div>
  <span class="label">{label}</span>
</aside>

<style>
  .evalbar {
    width: 24px;
    height: 100%;
    background: #222;            /* black side fills the rest by default */
    border-right: 1px solid #444;
    position: relative;
    overflow: hidden;
  }
  .white-fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: #fff;
    transition: height 200ms ease-out;
  }
  .label {
    position: absolute;
    left: 50%;
    bottom: 4px;
    transform: translateX(-50%);
    font-family: ui-monospace, "Cascadia Code", monospace;
    font-size: 10px;
    color: #444;
    background: rgba(255, 255, 255, 0.85);
    padding: 1px 4px;
    border-radius: 2px;
    pointer-events: none;
    z-index: 1;
  }
</style>
