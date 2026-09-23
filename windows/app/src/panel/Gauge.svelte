<script lang="ts">
  // Uma janela do sensor no cartão de conta: título, valor, barra e o reset.
  // Sem teto calibrado nem saturação: aqui há uma fração e um reset, e é só
  // isso que se promete.
  import { clock } from "../lib/clock.svelte";
  import { notStarted, resetText, severity } from "../lib/format";
  import { t } from "../lib/i18n";
  import type { Reading } from "../lib/types";

  // `session`: é a janela de 5h, a única que pode não ter começado.
  let { title, reading, session = false }: { title: string; reading: Reading | null; session?: boolean } =
    $props();
</script>

<div class="gauge">
  <div class="line">
    <span>{title}</span>
    {#if reading}
      <span class="value">{reading.text}</span>
    {:else}
      <span class="value absent">{t("panel.accounts.window.absent")}</span>
    {/if}
  </div>
  <span class="bar" class:empty={!reading}>
    {#if reading}
      <span
        class="fill {severity(reading.fraction)}"
        style:width="{Math.min(100, Math.max(0, reading.fraction * 100))}%"
      ></span>
    {/if}
  </span>
  <span class="caption">
    {#if !reading}
      {t("panel.account.window.expired")}
    {:else if session && notStarted(reading)}
      {t("panel.reset.notStarted.detail")}
    {:else}
      {resetText(reading, clock.now)}
    {/if}
  </span>
</div>

<style>
  .gauge {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }
  .line {
    display: flex;
    justify-content: space-between;
    font-size: var(--font-body);
  }
  .value {
    font-variant-numeric: tabular-nums;
  }
  .absent {
    color: var(--text-tertiary);
  }
  .bar {
    position: relative;
    height: 4px;
    border-radius: 2px;
    background: var(--track);
    overflow: hidden;
  }
  .bar.empty {
    opacity: 0.5;
  }
  .fill {
    position: absolute;
    inset: 0 auto 0 0;
    border-radius: 2px;
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
  .caption {
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
</style>
