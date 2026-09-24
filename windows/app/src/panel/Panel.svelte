<script lang="ts">
  // O flyout da bandeja (≙ UsagePanel): a tabela de contas por grupo, o
  // detalhamento da conta tocada e o rodapé com as duas portas da MESMA janela
  // ("Grupos" e "Ajustes" são o que o usuário procura). A altura da janela
  // segue o conteúdo: o painel mede e pede ao backend.
  import { onMount } from "svelte";
  import {
    fitFlyout,
    getSnapshot,
    initialSelection,
    onSnapshotChanged,
    openHome,
    quitApp,
  } from "../lib/api";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import type { AccountView, Snapshot } from "../lib/types";
  import AccountDetail from "./AccountDetail.svelte";
  import AccountsTable from "./AccountsTable.svelte";

  let snapshot = $state<Snapshot | null>(null);
  /** A conta tocada: enquanto houver uma, o cartão de baixo fala dela. */
  let selected = $state<string | null>(initialSelection());
  let root: HTMLElement | undefined = $state();

  const selectedAccount = $derived.by((): AccountView | undefined => {
    if (!snapshot || !selected) return undefined;
    for (const group of snapshot.groups) {
      const found = group.accounts.find((a) => a.id === selected);
      if (found) return found;
    }
    return undefined;
  });

  async function reload() {
    snapshot = await getSnapshot();
  }

  onMount(() => {
    void reload();
    const unlisten = onSnapshotChanged(() => void reload());
    return () => {
      void unlisten.then((stop) => stop());
    };
  });

  $effect(() => {
    if (!root) return;
    const element = root;
    const observer = new ResizeObserver(() => {
      void fitFlyout(Math.ceil(element.getBoundingClientRect().height));
    });
    observer.observe(element);
    return () => observer.disconnect();
  });
</script>

<main class="panel" bind:this={root}>
  {#if snapshot}
    {#if snapshot.groups.length === 0}
      <section class="card empty">
        <p class="empty-title">{t("groups.empty.title")}</p>
        <p class="empty-detail">{t("groups.empty.detail")}</p>
        <button class="primary" onclick={() => openHome("groups")}>{t("groups.empty.create")}</button>
      </section>
    {:else}
      <section class="card">
        <AccountsTable groups={snapshot.groups} {selected} onSelect={(id) => (selected = id)} />
      </section>
      {#if selectedAccount}
        <div class="card">
          <AccountDetail account={selectedAccount} onClose={() => (selected = null)} />
        </div>
      {/if}
    {/if}
    <footer>
      <button class="link" onclick={() => openHome("groups")}>
        <Icon name="stack" size={13} />{t("panel.groups")}
      </button>
      <button class="link" onclick={() => openHome("settings")}>
        <Icon name="gear" size={13} />{t("panel.settings")}
      </button>
      <span class="spacer"></span>
      <button class="link" onclick={() => quitApp()}>{t("panel.quit")}</button>
    </footer>
  {/if}
</main>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 12px;
    background: var(--flyout-bg);
  }
  .card {
    padding: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--card-bg);
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
  }
  .empty-title {
    font-weight: 600;
  }
  .empty-detail {
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
  .primary {
    margin-top: 6px;
    padding: 5px 12px;
    border: none;
    border-radius: 4px;
    background: var(--accent);
    color: var(--accent-text);
    font: inherit;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 0 4px;
  }
  .spacer {
    flex: 1;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .link:hover {
    color: var(--text);
  }
  button:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
    border-radius: 3px;
  }
</style>
