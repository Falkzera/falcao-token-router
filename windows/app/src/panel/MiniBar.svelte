<script lang="ts">
  // A barrinha da linha de conta (≙ MiniBar): desenha a janela de 5h, a coluna
  // que ela encosta. Desenhava a que DECIDE no macOS antigo e virava uma barra
  // cheia ao lado de um "2%": o olho casa o gráfico com o número vizinho. Sem
  // medida válida a trilha fica VAZIA — uma barra mínima leria "quase zero",
  // que é afirmação.
  import { severity } from "../lib/format";

  let { fraction, dim = false }: { fraction: number | null; dim?: boolean } = $props();
</script>

<span class="track" class:dim>
  {#if fraction !== null}
    <span
      class="fill {severity(fraction)}"
      style:width="{Math.max(2, 40 * Math.min(1, Math.max(0, fraction)))}px"
    ></span>
  {/if}
</span>

<style>
  .track {
    position: relative;
    display: block;
    width: 40px;
    height: 5px;
    border-radius: 3px;
    background: var(--track);
    overflow: hidden;
    flex: none;
  }
  .dim {
    opacity: 0.45;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 3px;
  }
  .calm {
    background: var(--calm);
  }
  .warning {
    background: var(--warning);
  }
  .critical {
    background: var(--critical);
  }
</style>
