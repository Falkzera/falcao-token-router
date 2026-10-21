<script module lang="ts">
  /** O atalho de uma conta: qual mostrar — no grupo do clique, se veio de um.
   *  `seq` muda a cada clique, para o mesmo atalho rolar de novo. */
  export interface HistoryFocus {
    account: string;
    group: string | null;
    seq: number;
  }
</script>

<script lang="ts">
  // A aba Histórico (21/10/2026): para cada conta, a janela de 5h nos últimos
  // 7 dias e a semanal nas últimas 4 semanas — cada ponto, uma medição da
  // sonda (a de 5 em 5 horas, o Medir contas, o `router measure`). Grupos na
  // ordem da lista; a conta em dois grupos aparece nos dois. Relê ao abrir e a
  // cada `snapshot-changed` — o laço avisa depois de cada grupo medido. O
  // atalho de cada conta (`focus`) rola até o cartão dela e o destaca.
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { t } from "../lib/i18n";
  import Rich from "../lib/Rich.svelte";
  import type { HistoryView } from "../lib/types";
  import HistoryCard from "./HistoryCard.svelte";

  let { focus = null, onFocused = () => {} }: { focus?: HistoryFocus | null; onFocused?: () => void } =
    $props();

  let history = $state<HistoryView | null>(null);
  let list: HTMLElement | undefined = $state();
  /** O cartão em destaque agora (`grupo:conta`), por um instante. */
  let highlighted = $state<string | null>(null);

  onMount(() => {
    const load = () => void api.getHistory().then((next) => (history = next));
    load();
    const unlisten = api.onSnapshotChanged(load);
    return () => void unlisten.then((stop) => stop());
  });

  // O atalho: com o quadro na tela, rola até o cartão da conta — no grupo do
  // clique; senão, o primeiro em que ela aparece — e o destaca. Atendido, o
  // pedido some (a releitura de 30 s não rola a página de novo).
  $effect(() => {
    if (!history || !focus || !list) return;
    const wanted = focus;
    const cards = [...list.querySelectorAll<HTMLElement>("[data-account]")];
    const card =
      cards.find((c) => c.dataset.account === wanted.account && c.dataset.group === wanted.group) ??
      cards.find((c) => c.dataset.account === wanted.account);
    if (card) {
      const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      card.scrollIntoView({ block: "start", behavior: reduce ? "auto" : "smooth" });
      const key = `${card.dataset.group}:${wanted.account}`;
      highlighted = key;
      setTimeout(() => {
        if (highlighted === key) highlighted = null;
      }, 1800);
    }
    onFocused();
  });
</script>

{#if history}
  <div class="history" bind:this={list}>
    {#if history.groups.length === 0}
      <div class="empty">
        <p class="empty-title">{t("history.empty.title")}</p>
        <p class="empty-detail">{t("history.empty.detail")}</p>
      </div>
    {:else}
      <p class="intro"><Rich text={t("history.intro")} /></p>
      {#each history.groups as group (group.id)}
        <section>
          <h3>{group.name}</h3>
          {#each group.accounts as account (account.id)}
            <HistoryCard
              {account}
              group={group.id}
              threshold={group.thresholdPercent}
              scheduleOn={history.scheduleOn}
              highlight={highlighted === `${group.id}:${account.id}`}
            />
          {:else}
            <p class="none">{t("history.group.empty")}</p>
          {/each}
        </section>
      {/each}
    {/if}
  </div>
{/if}

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
    padding: 16px var(--page-inline);
    overflow: auto;
  }
  .intro {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h3 {
    margin: 0;
    font-size: var(--font-caption);
    font-weight: 600;
    color: var(--text-secondary);
  }
  .none {
    font-size: var(--font-caption);
    color: var(--text-tertiary);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    padding: 48px 32px;
    text-align: center;
  }
  .empty-title {
    font-weight: 600;
  }
  .empty-detail {
    max-width: 360px;
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
</style>
