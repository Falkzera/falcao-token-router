<script lang="ts">
  // O resumo de cada conta na aba Grupos (pedido de 23/09/2026): o usuário
  // escolhe o que vê ali — de fábrica, tudo. Um par por janela, o uso e o
  // reset dela: o reset mora embaixo do número, então sem a janela ele fica
  // desligado (o estado dele é guardado e volta com ela). O tooltip da linha
  // segue com tudo: é o detalhe. Grava na hora, em fila — dois cliques
  // rápidos não se atropelam, e a tela mostra o último.
  import * as api from "../lib/api";
  import { t, type Key } from "../lib/i18n";
  import type { SummaryItem } from "../lib/types";

  let { initial }: { initial: SummaryItem[] } = $props();

  const PAIRS: { usage: SummaryItem; usageKey: Key; reset: SummaryItem; resetKey: Key }[] = [
    {
      usage: "fiveHour",
      usageKey: "settings.summary.item.fiveHour",
      reset: "fiveHourReset",
      resetKey: "settings.summary.item.fiveHourReset",
    },
    {
      usage: "sevenDay",
      usageKey: "settings.summary.item.sevenDay",
      reset: "sevenDayReset",
      resetKey: "settings.summary.item.sevenDayReset",
    },
    {
      usage: "model",
      usageKey: "settings.summary.item.model",
      reset: "modelReset",
      resetKey: "settings.summary.item.modelReset",
    },
  ];

  /** Os itens TIRADOS (como na status line). */
  // svelte-ignore state_referenced_locally
  let hidden = $state<SummaryItem[]>(initial);
  let failure = $state<string | null>(null);
  let queue: Promise<void> = Promise.resolve();

  const shows = (item: SummaryItem) => !hidden.includes(item);

  function save(next: SummaryItem[]) {
    hidden = next;
    queue = queue.then(async () => {
      try {
        await api.setHiddenSummary(next);
        failure = null;
      } catch (error) {
        failure = String(error);
      }
    });
  }

  function toggle(item: SummaryItem, on: boolean) {
    save(on ? hidden.filter((i) => i !== item) : [...hidden, item]);
  }
</script>

<section>
  <h3>{t("settings.section.summary")}</h3>
  <div class="group">
    <div class="row">
      <div class="head">
        <p class="detail">{t("settings.summary.detail")}</p>
        {#if hidden.length > 0}
          <button class="link" onclick={() => save([])}>{t("settings.summary.restore")}</button>
        {/if}
      </div>
      <div class="items">
        {#each PAIRS as pair (pair.usage)}
          <label class="check">
            <input
              type="checkbox"
              checked={shows(pair.usage)}
              onchange={(event) => toggle(pair.usage, event.currentTarget.checked)}
            />
            <span>{t(pair.usageKey)}</span>
          </label>
          <label class="check" class:off={!shows(pair.usage)}>
            <input
              type="checkbox"
              checked={shows(pair.reset)}
              disabled={!shows(pair.usage)}
              onchange={(event) => toggle(pair.reset, event.currentTarget.checked)}
            />
            <span>{t(pair.resetKey)}</span>
          </label>
        {/each}
      </div>
      {#if failure}
        <p class="failure">{t("settings.summary.saveFailed.format", failure)}</p>
      {/if}
    </div>
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
    font-size: var(--font-caption);
    font-weight: 600;
    color: var(--text-secondary);
  }
  .group {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px 14px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 12px;
  }
  .detail {
    flex: 1;
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  /* Duas colunas: o uso à esquerda, o reset dele ao lado. */
  .items {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 6px 12px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .check.off {
    color: var(--text-tertiary);
    cursor: default;
  }
  .check input {
    margin: 0;
    accent-color: var(--accent);
  }
  .link {
    flex: none;
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .failure {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--critical);
  }
</style>
