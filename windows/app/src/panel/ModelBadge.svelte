<script lang="ts">
  // A marca de que quem manda no número é uma janela POR MODELO (≙ ModelBadge).
  // Sem ela a linha mentiria por omissão: nem a coluna de 5h nem a de 7d ficam
  // destacadas, e o usuário veria duas folgas sem nada explicando a troca. O
  // nome vem cru do `/usage` (Fable, Opus) — o app não mantém lista de modelos.
  import { severity } from "../lib/format";
  import type { ModelReading } from "../lib/types";

  let { model }: { model: ModelReading } = $props();
</script>

<span class="badge {severity(model.reading.fraction)}">{model.name} {model.reading.text}</span>

<style>
  .badge {
    flex: none;
    padding: 0 5px;
    border-radius: 999px;
    font-size: var(--font-small);
    font-variant-numeric: tabular-nums;
    line-height: 16px;
    white-space: nowrap;
  }
  .calm {
    color: var(--calm);
    background: color-mix(in srgb, var(--calm) 22%, transparent);
  }
  .warning {
    color: var(--warning-text);
    background: color-mix(in srgb, var(--warning) 22%, transparent);
  }
  .critical {
    color: var(--critical);
    background: color-mix(in srgb, var(--critical) 22%, transparent);
  }
</style>
