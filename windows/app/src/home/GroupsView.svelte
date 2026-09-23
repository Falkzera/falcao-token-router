<script lang="ts">
  // A aba do produto (≙ GroupsView.swift): cria grupos, adiciona contas pelo
  // login oficial, define a ordem de preferência e o limiar de troca — tudo
  // pela tela, sem terminal. O store faz o trabalho; aqui é apresentação.
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import type { AccountView, IntegrationState, Snapshot, TerminalView } from "../lib/types";
  import ErrorBanner from "./ErrorBanner.svelte";
  import GroupCard from "./GroupCard.svelte";
  import NewGroupDialog from "./NewGroupDialog.svelte";
  import TerminalSection from "./TerminalSection.svelte";

  let { onAddAccount, onRelogin }: {
    onAddAccount: (groupId: string) => void;
    onRelogin: (account: AccountView, groupId: string) => void;
  } = $props();

  let snapshot = $state<Snapshot | null>(null);
  /** O diálogo de grupo novo, com o login que o `~\.claude` tem (se houver). */
  let creating = $state<{ foreign: string | null } | null>(null);

  // MARK: a integração de terminal — o quadro por shell é lento (um
  // PowerShell por edição), então é pedido à parte: ao abrir, depois de cada
  // ação, e quando a janela volta ao foco (o usuário pode ter mexido no perfil
  // lá fora) — no máximo a cada 30 s.
  const RECHECK_AFTER_MS = 30_000;
  let terminal = $state<TerminalView | null>(null);
  let checking = $state(false);
  let checkedAt = 0;
  async function checkTerminal() {
    if (checking) return;
    checking = true;
    try {
      terminal = await api.terminalReport();
      checkedAt = Date.now();
    } catch {
      // Sem quadro, a seção cai no convite a ativar — o estado barato do
      // snapshot continua valendo para os cartões.
    } finally {
      checking = false;
    }
  }

  /** O que os cartões dizem da integração: com o quadro, o que ele diz; sem
   *  ele (ainda), o estado barato dos scripts. */
  const integration = $derived<IntegrationState>(
    terminal
      ? terminal.fullyInstalled
        ? "ok"
        : terminal.needsInstall
          ? "install"
          : "attention"
      : snapshot?.scripts === "current"
        ? "ok"
        : "install",
  );

  onMount(() => {
    void api.getSnapshot().then((s) => (snapshot = s));
    const unlisten = api.onSnapshotChanged(() => void api.getSnapshot().then((s) => (snapshot = s)));
    void checkTerminal();
    const onFocus = () => {
      if (Date.now() - checkedAt > RECHECK_AFTER_MS) void checkTerminal();
    };
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("focus", onFocus);
      void unlisten.then((stop) => stop());
    };
  });

  async function openNewGroup() {
    creating = { foreign: await api.foreignDefaultLogin() };
  }
</script>

{#if snapshot}
  <div class="groups">
    <div class="scroll">
      <header>
        <p class="subtitle">{t("groups.subtitle")}</p>
        <button class="secondary" onclick={openNewGroup}>
          <Icon name="plus" size={13} />{t("groups.new")}
        </button>
      </header>

      {#if snapshot.groups.length === 0}
        <div class="empty">
          <p class="empty-title">{t("groups.empty.title")}</p>
          <p class="empty-detail">{t("groups.empty.detail")}</p>
          <button class="primary" onclick={openNewGroup}>{t("groups.empty.create")}</button>
        </div>
      {:else}
        {#each snapshot.groups as group (group.id)}
          <GroupCard
            {group}
            {integration}
            measuringGroup={snapshot.measuringGroup}
            onSnapshot={(next) => (snapshot = next)}
            onAddAccount={() => onAddAccount(group.id)}
            onRelogin={(account) => onRelogin(account, group.id)}
          />
        {/each}
        <!-- Depois dos grupos: sem grupo, `claude <grupo>` não teria o que abrir. -->
        <TerminalSection
          report={terminal}
          {checking}
          onReport={(next) => {
            terminal = next;
            checkedAt = Date.now();
          }}
          onSnapshot={(next) => (snapshot = next)}
        />
      {/if}
    </div>

    {#if snapshot.lastError}
      <ErrorBanner error={snapshot.lastError} onDismiss={async () => (snapshot = await api.dismissError())} />
    {/if}
  </div>

  {#if creating}
    <NewGroupDialog
      firstGroup={snapshot.groups.length === 0}
      foreignLogin={creating.foreign}
      onCancel={() => (creating = null)}
      onCreate={async (name, asDefault) => {
        creating = null;
        snapshot = await api.addGroup(name, asDefault);
      }}
    />
  {/if}
{/if}

<style>
  .groups {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .scroll {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 16px;
    overflow: auto;
  }
  header {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .subtitle {
    flex: 1;
    font-size: var(--font-caption);
    color: var(--text-secondary);
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
  .empty .primary {
    margin-top: 8px;
  }
</style>
