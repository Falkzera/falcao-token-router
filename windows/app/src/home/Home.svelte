<script lang="ts">
  // A janela única (≙ HomeWindow.swift): Grupos (o produto) e Ajustes, em abas.
  // O tamanho é da janela (do usuário, lembrado pelo Rust), nunca do conteúdo:
  // as abas têm alturas naturais diferentes e a janela pularia a cada troca.
  // Numa janela larga, abas e páginas seguem a mesma coluna (`--page-inline`).
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { t } from "../lib/i18n";
  import type { AccountView, AppInfo, HomeTab, LoginView } from "../lib/types";
  import GroupsView from "./GroupsView.svelte";
  import LoginDialog from "./LoginDialog.svelte";
  import SettingsView from "./SettingsView.svelte";

  let { info }: { info: AppInfo } = $props();

  // A aba da abertura (a bandeja pode ter pedido Ajustes) vale ANTES do
  // primeiro desenho: montar Grupos por um instante pediria o quadro do
  // terminal — um PowerShell por edição — à toa.
  // svelte-ignore state_referenced_locally
  let tab = $state<HomeTab>(info.initialTab);

  // MARK: o login oficial (Adicionar conta / Relogar…)
  let login = $state<LoginView | null>(null);

  /** Fica a visão mais nova: a resposta de um comando pode chegar DEPOIS do
   *  evento de uma mudança posterior (o link sai em milissegundos). */
  function accept(next: LoginView | null) {
    if (next === null) login = null;
    else if (login === null || next.revision >= login.revision) login = next;
  }

  async function addAccount(groupId: string) {
    accept(await api.startLogin(groupId));
  }

  async function relogin(account: AccountView, groupId: string) {
    accept(await api.startRelogin(account.id, groupId));
  }

  onMount(() => {
    // A bandeja pode pedir outra aba com a janela já aberta.
    const unlisten = api.onNavigate((next) => (tab = next));
    // Um login em andamento continua na janela reaberta.
    void api.currentLogin().then(accept);
    const unlistenLogin = api.onLoginChanged(accept);
    return () => {
      void unlisten.then((stop) => stop());
      void unlistenLogin.then((stop) => stop());
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
      <GroupsView onAddAccount={(groupId) => void addAccount(groupId)} onRelogin={(account, groupId) => void relogin(account, groupId)} />
    {:else}
      <SettingsView />
    {/if}
  </section>
</div>

{#if login}
  <LoginDialog view={login} onClosed={() => (login = null)} />
{/if}

<style>
  .home {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  .tabs {
    display: flex;
    gap: 4px;
    padding: 8px var(--page-inline) 0;
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
