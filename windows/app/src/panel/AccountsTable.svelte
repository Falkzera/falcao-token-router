<script lang="ts">
  // As contas de cada grupo, com quem serve o grupo e quanto sobrou (≙
  // AccountsSection). Uma TABELA, não uma lista de linhas soltas: o rótulo da
  // janela ("5h", "7d") sobe para o cabeçalho e os números descem em colunas de
  // largura fixa — "em qual conta eu estou, e quanto sobrou nas outras?" se
  // responde varrendo uma coluna. As larguras (40/34/34, respiro de 8) são as
  // mesmas no cabeçalho e nas linhas, senão nada alinha de conta para conta.
  import { t } from "../lib/i18n";
  import type { GroupView } from "../lib/types";
  import AccountRow from "./AccountRow.svelte";
  import SessionsBadge from "./SessionsBadge.svelte";

  let {
    groups,
    selected,
    onSelect,
  }: { groups: GroupView[]; selected: string | null; onSelect: (id: string | null) => void } =
    $props();
</script>

<div class="table">
  <div class="header" title={t("panel.accounts.columns.help")}>
    <span class="title">{t("panel.section.accounts")}</span>
    <span></span>
    <span class="col">{t("panel.accounts.window.fiveHour")}</span>
    <span class="col">{t("panel.accounts.window.sevenDay")}</span>
  </div>
  <hr />
  {#each groups as group (group.id)}
    {#if group.accounts.length > 0}
      <div class="group">
        <div class="group-name">
          <span>{group.name}</span>
          <SessionsBadge sessions={group.sessions} />
        </div>
        {#each group.accounts as account (account.id)}
          <AccountRow
            {account}
            active={group.activeAccountId === account.id}
            selected={selected === account.id}
            onSelect={() => onSelect(selected === account.id ? null : account.id)}
          />
        {/each}
      </div>
    {/if}
  {/each}
</div>

<style>
  .table {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .header {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 40px 34px 34px;
    column-gap: 8px;
    padding: 0 6px;
    font-size: var(--font-small);
  }
  .title {
    font-weight: 600;
    color: var(--text-secondary);
    letter-spacing: 0.02em;
  }
  .col {
    text-align: right;
    color: var(--text-tertiary);
  }
  hr {
    margin: -2px 0 0;
    border: none;
    border-top: 1px solid var(--border);
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .group-name {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 0 6px 2px;
    font-size: var(--font-small);
    font-weight: 500;
    color: var(--text-secondary);
  }
</style>
