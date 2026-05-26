<script lang="ts">
  import wK from "./pieces/wK.svg";
  import wQ from "./pieces/wQ.svg";
  import wR from "./pieces/wR.svg";
  import wB from "./pieces/wB.svg";
  import wN from "./pieces/wN.svg";
  import wP from "./pieces/wP.svg";
  import bK from "./pieces/bK.svg";
  import bQ from "./pieces/bQ.svg";
  import bR from "./pieces/bR.svg";
  import bB from "./pieces/bB.svg";
  import bN from "./pieces/bN.svg";
  import bP from "./pieces/bP.svg";

  interface Props { piece: string; size: number; }
  let { piece, size }: Props = $props();

  const URLS: Record<string, string> = {
    K: wK, Q: wQ, R: wR, B: wB, N: wN, P: wP,
    k: bK, q: bQ, r: bR, b: bB, n: bN, p: bP,
  };
</script>

<image href={URLS[piece]} width={size} height={size} />
