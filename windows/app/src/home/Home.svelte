<script lang="ts">
  // A janela única (≙ HomeWindow.swift): Grupos (o produto) e Ajustes, em abas.
  // Tamanho fixo — as abas têm alturas naturais diferentes e a janela pularia
  // de tamanho a cada troca.
  import { onMount } from "svelte";
  import { onNavigate } from "../lib/api";
  import { t } from "../lib/i18n";
  import type { AppInfo, HomeTab } from "../lib/types";
  import GroupsView from "./GroupsView.svelte";
  import SettingsView from "./SettingsView.svelte";

  let { info }: { info: AppInfo } = $props();

  // A aba da abertura (a bandeja pode ter pedido Ajustes) vale ANTES do
  // primeiro desenho: montar Grupos por um instante pediria o quadro do
  // terminal — um PowerShell por edição — à toa.
  // svelte-ignore state_referenced_locally
  let tab = $state<HomeTab>(info.initialTab);

  onMount(() => {
    // A bandeja pode pedir outra aba com a janela já aberta.
    const unlisten = onNavigate((next) => (tab = next));
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

<div class="home">
  <div class="tabs" role="tablist">
    <button role="tab" aria-selected={tab === "groups"} onclick={() => (tab = "groups")}>
      {t("home.tab.groups")}
    </button>
    <button role="tab" aria-selected={tab === "settings"} onclick={() => (tab = "settings")}>
      {t("home.tab.settings")}
    </button>
  </div>

  <section class="page" role="tabpanel">
    {#if tab === "groups"}
      <!-- O login (adicionar e relogar) entra na fatia 5.5. -->
      <GroupsView onAddAccount={() => {}} onRelogin={() => {}} />
    {:else}
      <SettingsView />
    {/if}
  </section>
</div>

<style>
  .home {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 8px 16px 0;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    border: none;
    background: none;
    padding: 8px 12px;
    border-bottom: 2px solid transparent;
    color: var(--text-secondary);
    font: inherit;
    cursor: pointer;
  }
  .tabs button[aria-selected="true"] {
    color: var(--text);
    border-bottom-color: var(--accent);
    font-weight: 600;
  }
  .page {
    flex: 1;
    min-height: 0;
  }
</style>
