<script lang="ts">
  // Uma conta na tabela do flyout (≙ RouterAccountRow). A linha INTEIRA é o
  // alvo do clique e dona do tooltip: o número tem 34 px, e mirar nele para
  // saber "por que 66% se a status line diz 1%?" é pedir precisão que ninguém
  // tem com o mouse.
  import { clock } from "../lib/clock.svelte";
  import { ageSeconds, severity, STALE_SECONDS, VERY_STALE_SECONDS } from "../lib/format";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import type { AccountView, Reading } from "../lib/types";
  import { accountHelp, staleHelp } from "./accountHelp";
  import MiniBar from "./MiniBar.svelte";
  import ModelBadge from "./ModelBadge.svelte";

  let {
    account,
    active,
    selected,
    onSelect,
  }: { account: AccountView; active: boolean; selected: boolean; onSelect: () => void } = $props();

  const usage = $derived(account.usage);
  const age = $derived(usage ? ageSeconds(usage.sampledAt, clock.now) : 0);
  /** Amostra com mais de 1h já pode descrever outra realidade: esmaece. */
  const stale = $derived(age > STALE_SECONDS);
  /** Meio dia: marca explícita — o risco é acreditar num número otimista. */
  const veryStale = $derived(age > VERY_STALE_SECONDS);
  const help = $derived(accountHelp(usage, clock.now));

  /** Só a janela que decide ganha peso e cor; com as duas coloridas, nada
   *  diria qual dispara a troca. Velha, perde a cor que sugere frescor. */
  function tone(reading: Reading, bound: boolean): string {
    return bound && !stale ? severity(reading.fraction) : "plain";
  }
</script>

<button
  class="row"
  class:active
  class:selected
  title={help}
  aria-pressed={selected}
  onclick={onSelect}
>
  <span class="who">
    <span
      class="dot {active ? severity(usage?.fraction ?? 0) : 'idle'}"
      aria-hidden="true"
    ></span>
    <span class="label">{account.label}</span>
    {#if usage?.bound === "model" && usage.model}
      <ModelBadge model={usage.model} />
    {/if}
    {#if veryStale && usage}
      <span class="clock" title={staleHelp(usage, clock.now)}><Icon name="clockAlert" size={12} /></span>
    {/if}
  </span>
  {#if usage}
    <MiniBar fraction={usage.fiveHour?.fraction ?? null} dim={stale} />
    {#each [{ reading: usage.fiveHour, bound: usage.bound === "fiveHour" }, { reading: usage.sevenDay, bound: usage.bound === "sevenDay" }] as column}
      {#if column.reading}
        <span class="num {tone(column.reading, column.bound)}" class:bound={column.bound}>
          {column.reading.text}
        </span>
      {:else}
        <!-- Janela sem dado ou expirada: um traço, mais apagado que número. -->
        <span class="num absent">{t("panel.accounts.window.absent")}</span>
      {/if}
    {/each}
  {:else}
    <!-- Sem amostra a conta está PRONTA (entra no rodízio sozinha): um traço
         leria como "quebrada". Ocupa a largura das três colunas. -->
    <span class="ready">{t("panel.accounts.ready")}</span>
  {/if}
</button>

<style>
  .row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 40px 34px 34px;
    align-items: center;
    column-gap: 8px;
    width: 100%;
    padding: 3px 6px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--font-caption);
    text-align: left;
    cursor: pointer;
  }
  .row:hover {
    background: var(--row-hover);
  }
  .row.active {
    background: var(--row-active);
  }
  .row.selected {
    background: var(--row-selected);
  }
  .row:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
  .who {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .active .label {
    font-weight: 600;
  }
  .dot {
    flex: none;
    width: 6px;
    height: 6px;
    border-radius: 50%;
  }
  .dot.idle {
    border: 1px solid var(--text-quaternary);
  }
  .dot.calm {
    background: var(--calm);
  }
  .dot.warning {
    background: var(--warning);
  }
  .dot.critical {
    background: var(--critical);
  }
  .clock {
    display: inline-flex;
    color: var(--text-tertiary);
  }
  .num {
    text-align: right;
    font-size: var(--font-small);
    font-variant-numeric: tabular-nums;
    color: var(--text-secondary);
  }
  .num.bound {
    font-weight: 600;
  }
  .num.calm {
    color: var(--calm);
  }
  .num.warning {
    color: var(--warning-text);
  }
  .num.critical {
    color: var(--critical);
  }
  .num.absent {
    color: var(--text-quaternary);
  }
  .ready {
    grid-column: 2 / span 3;
    text-align: right;
    font-size: var(--font-small);
    color: var(--text-tertiary);
  }
</style>
