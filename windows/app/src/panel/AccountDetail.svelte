<script lang="ts">
  // O detalhamento de UMA conta, aberto ao tocar a linha (≙ accountSection):
  // 5h e 7d com barra e reset, quem mediu e há quanto tempo, e — se alguém
  // sondou — a janela POR MODELO embaixo, com carimbo próprio (ela pode ser de
  // ontem enquanto as duas de cima são de agora). Deliberadamente pobre: sem
  // tokens/min nem valor, que o porte não mede.
  import { clock } from "../lib/clock.svelte";
  import { ageSeconds, duration } from "../lib/format";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import type { AccountView } from "../lib/types";
  import Gauge from "./Gauge.svelte";

  let { account, onClose }: { account: AccountView; onClose: () => void } = $props();

  const usage = $derived(account.usage);
</script>

<section class="detail">
  <div class="head">
    <span class="title">{t("panel.section.account")}</span>
    <span class="who">· {account.label}</span>
    <button class="close" title={t("panel.account.close")} onclick={onClose}>
      <Icon name="close" size={12} />
    </button>
  </div>

  {#if usage}
    <Gauge title={t("panel.gauge.current")} reading={usage.fiveHour} session />
    <Gauge title={t("panel.gauge.weekly")} reading={usage.sevenDay} />
    <!-- Rótulo e ícone acompanham a origem: antena é o sensor (uma requisição
         que a conta atendeu), medidor é a sonda (uma consulta de propósito). -->
    <span class="stamp">
      <Icon name={usage.origin === "probe" ? "probe" : "sensor"} size={12} />
      {usage.origin === "probe"
        ? t("panel.account.sampled.probe.format", duration(ageSeconds(usage.sampledAt, clock.now)))
        : t("panel.account.sampled.sensor.format", duration(ageSeconds(usage.sampledAt, clock.now)))}
    </span>
    {#if usage.model?.sampledAt}
      <hr />
      <Gauge title={usage.model.name} reading={usage.model.reading} />
      <span class="stamp">
        <Icon name="probe" size={12} />
        {t("panel.account.probed.format", duration(ageSeconds(usage.model.sampledAt, clock.now)))}
      </span>
    {/if}
  {:else}
    <p class="ready">{t("panel.accounts.ready.help.probe")}</p>
  {/if}
</section>

<style>
  .detail {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .title {
    font-size: var(--font-small);
    font-weight: 600;
    letter-spacing: 0.02em;
    color: var(--text-secondary);
  }
  .who {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-small);
    color: var(--text-tertiary);
  }
  .close {
    display: inline-flex;
    padding: 3px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
  }
  .close:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .stamp {
    display: flex;
    align-items: flex-start;
    gap: 5px;
    font-size: var(--font-small);
    line-height: 15px;
    color: var(--text-tertiary);
  }
  /* Em texto de duas linhas o ícone acompanha a primeira. */
  .stamp :global(svg) {
    margin-top: 1.5px;
  }
  hr {
    margin: 0;
    border: none;
    border-top: 1px solid var(--border);
  }
  .ready {
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
</style>
