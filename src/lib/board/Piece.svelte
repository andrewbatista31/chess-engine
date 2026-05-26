<script lang="ts">
  interface Props { piece: string; size: number; }
  let { piece, size }: Props = $props();

  const SYMBOLS: Record<string, string> = {
    K: "♔", Q: "♕", R: "♖", B: "♗", N: "♘", P: "♙",
    k: "♚", q: "♛", r: "♜", b: "♝", n: "♞", p: "♟",
  };
  const isWhite = $derived(piece === piece.toUpperCase());
</script>

<text
  x="50%"
  y="50%"
  font-size={size * 0.75}
  text-anchor="middle"
  dominant-baseline="central"
  font-family='"Segoe UI Symbol", "DejaVu Sans", sans-serif'
  fill={isWhite ? "#fff" : "#000"}
  stroke={isWhite ? "#000" : "none"}
  stroke-width={isWhite ? 1 : 0}
>{SYMBOLS[piece]}</text>
