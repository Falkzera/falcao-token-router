<script lang="ts">
  // A janela única (≙ HomeWindow.swift): Grupos (o produto) e Ajustes, em abas.
  // Tamanho fixo — as abas têm alturas naturais diferentes e a janela pularia
  // de tamanho a cada troca.
  import { t } from "../lib/i18n";
  import type { AppInfo } from "../lib/types";

  let { info }: { info: AppInfo } = $props();

  type Tab = "groups" | "settings";
  let tab = $state<Tab>("groups");
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
      <p class="caption">{t("groups.subtitle")}</p>
    {:else}
      <p class="caption">{t("settings.version.format", info.version)}</p>
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
    padding: 12px 16px 0;
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
    overflow: auto;
    padding: 16px;
  }
  .caption {
    color: var(--text-secondary);
    font-size: var(--font-caption);
  }
</style>
