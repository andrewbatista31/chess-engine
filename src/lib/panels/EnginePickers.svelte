<script lang="ts">
  import { enginesStore, type EngineSlot } from "../stores/engines.svelte.ts";

  let openSide: "w" | "b" | null = $state(null);

  function label(slot: EngineSlot): string {
    return slot.kind === "Human" ? "Human" : `Stockfish (${slot.skill})`;
  }

  function setKind(side: "w" | "b", kind: "Human" | "Stockfish") {
    const cur = side === "w" ? enginesStore.white : enginesStore.black;
    const next: EngineSlot = { kind, skill: cur.skill };
    if (side === "w") enginesStore.white = next; else enginesStore.black = next;
  }

  function setSkill(side: "w" | "b", skill: number) {
    const cur = side === "w" ? enginesStore.white : enginesStore.black;
    const next: EngineSlot = { kind: cur.kind, skill };
    if (side === "w") enginesStore.white = next; else enginesStore.black = next;
  }
</script>

<div class="pickers">
  {#each ["w", "b"] as const as side}
    {@const slot = side === "w" ? enginesStore.white : enginesStore.black}
    <div class="chip-wrap">
      <button
        class="chip"
        onclick={() => (openSide = openSide === side ? null : side)}
      >
        {side === "w" ? "⚪" : "⚫"} {label(slot)}
      </button>
      {#if openSide === side}
        <div class="popover" role="dialog">
          <label>
            <input
              type="radio"
              name="kind-{side}"
              checked={slot.kind === "Human"}
              onchange={() => setKind(side, "Human")}
            />
            Human
          </label>
          <label>
            <input
              type="radio"
              name="kind-{side}"
              checked={slot.kind === "Stockfish"}
              onchange={() => setKind(side, "Stockfish")}
            />
            Stockfish
          </label>
          <div class="skill-row">
            Skill
            <input
              type="range"
              min="0"
              max="20"
              value={slot.skill}
              disabled={slot.kind === "Human"}
              oninput={(e) => setSkill(side, +(e.currentTarget as HTMLInputElement).value)}
            />
            <span class="skill-val">{slot.skill}</span>
          </div>
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .pickers { display: flex; gap: 6px; }
  .chip-wrap { position: relative; }
  .chip {
    background: #2e3d4a;
    border: 1px solid #4a90d9;
    color: #eee;
    padding: 4px 10px;
    border-radius: 12px;
    cursor: pointer;
    font-size: 12px;
  }
  .chip:hover { background: #3a4a5a; }
  .popover {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: #1f1f1f;
    border: 1px solid #555;
    border-radius: 6px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    z-index: 30;
    min-width: 200px;
    color: #ddd;
    font-size: 12px;
  }
  .popover label { display: flex; align-items: center; gap: 6px; cursor: pointer; }
  .skill-row { display: flex; align-items: center; gap: 8px; margin-top: 4px; }
  .skill-row input[type="range"] { flex: 1; }
  .skill-val { width: 24px; text-align: right; color: #fff; }
</style>
