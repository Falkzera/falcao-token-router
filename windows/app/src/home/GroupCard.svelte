<script lang="ts">
  // O cartão de um grupo (≙ GroupCard de GroupsView.swift), sem os defeitos do
  // macOS: o limiar grava ao SOLTAR (não a cada passo), reordenar funciona (por
  // arrasto e por teclado), a confirmação de apagar conta as contas que de fato
  // perdem o login (as exclusivas), e "Tornar padrão" avisa do login que o
  // `~\.claude` tem hoje.
  import { untrack } from "svelte";
  import * as api from "../lib/api";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import Menu from "../lib/Menu.svelte";
  import Switch from "../lib/Switch.svelte";
  import type { AccountView, GroupView, IntegrationState, Snapshot } from "../lib/types";
  import SessionsBadge from "../panel/SessionsBadge.svelte";
  import AccountItem from "./AccountItem.svelte";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let {
    group,
    integration,
    measuringGroup,
    onSnapshot,
    onAddAccount,
    onRelogin,
  }: {
    group: GroupView;
    /** O comando só funciona com a integração de pé — o aviso leva até ela. */
    integration: IntegrationState;
    measuringGroup: string | null;
    onSnapshot: (snapshot: Snapshot) => void;
    onAddAccount: () => void;
    onRelogin: (account: AccountView) => void;
  } = $props();

  // MARK: renomear
  let editing = $state(false);
  let draft = $state("");
  function startRename() {
    draft = group.name;
    editing = true;
  }
  async function commitRename() {
    const name = draft.trim();
    editing = false;
    if (name && name !== group.name) onSnapshot(await api.renameGroup(group.id, name));
  }

  // MARK: comando do terminal
  let copied = $state(false);
  async function copyCommand() {
    await api.copyText(group.command);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  // MARK: limiar — o número acompanha o arrasto; grava só ao soltar.
  let threshold = $state(untrack(() => group.thresholdPercent));
  $effect(() => {
    threshold = group.thresholdPercent;
  });

  // MARK: ordem de preferência (arrasto e teclado)
  let order = $state(untrack(() => group.accounts.map((a) => a.id)));
  let dragging = $state<string | null>(null);
  let list: HTMLElement | undefined = $state();
  $effect(() => {
    const ids = group.accounts.map((a) => a.id);
    // Durante o arrasto a ordem local manda; o quadro novo a alcança depois.
    if (untrack(() => dragging) === null) order = ids;
  });
  const ordered = $derived(
    order
      .map((id) => group.accounts.find((a) => a.id === id))
      .filter((a): a is AccountView => a !== undefined),
  );

  async function commitOrder(next: string[]) {
    order = next;
    onSnapshot(await api.reorderAccounts(group.id, next));
  }
  function move(id: string, delta: number) {
    const from = order.indexOf(id);
    const to = from + delta;
    if (from < 0 || to < 0 || to >= order.length) return;
    const next = [...order];
    next.splice(from, 1);
    next.splice(to, 0, id);
    void commitOrder(next);
  }
  function dragStart(id: string, event: PointerEvent) {
    if (event.button !== 0) return;
    dragging = id;
    try {
      // Os movimentos seguem para a alça mesmo com o ponteiro fora dela.
      (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
    } catch {
      // Ponteiro que já soltou: o arrasto acaba no próximo `pointerup`.
    }
  }
  function dragMove(event: PointerEvent) {
    if (!dragging || !list) return;
    const rows = [...list.querySelectorAll<HTMLElement>("[data-account]")];
    let target = rows.length - 1;
    for (const [i, row] of rows.entries()) {
      const box = row.getBoundingClientRect();
      if (event.clientY < box.top + box.height / 2) {
        target = i;
        break;
      }
    }
    const from = order.indexOf(dragging);
    if (from >= 0 && from !== target) {
      const next = [...order];
      next.splice(from, 1);
      next.splice(target, 0, dragging);
      order = next;
    }
  }
  function dragEnd() {
    if (!dragging) return;
    dragging = null;
    const before = group.accounts.map((a) => a.id).join();
    if (order.join() !== before) void commitOrder(order);
  }

  // MARK: confirmações
  type Pending =
    | { kind: "removeAccount"; account: AccountView }
    | { kind: "deleteGroup" }
    | { kind: "makeDefault"; foreign: string | null };
  let pending = $state<Pending | null>(null);

  async function askMakeDefault() {
    pending = { kind: "makeDefault", foreign: await api.foreignDefaultLogin() };
  }
  async function confirmPending() {
    const current = pending;
    pending = null;
    if (!current) return;
    if (current.kind === "removeAccount") onSnapshot(await api.removeAccount(current.account.id));
    else if (current.kind === "deleteGroup") onSnapshot(await api.removeGroup(group.id));
    else onSnapshot(await api.makeDefault(group.id));
  }

  const measuring = $derived(measuringGroup === group.id);
</script>

<article class="card">
  <header class="title">
    {#if editing}
      <form
        class="rename"
        onsubmit={(event) => {
          event.preventDefault();
          void commitRename();
        }}
      >
        <!-- svelte-ignore a11y_autofocus -->
        <input
          class="field"
          type="text"
          placeholder={t("groups.name.placeholder")}
          bind:value={draft}
          autofocus
          onkeydown={(event) => {
            if (event.key === "Escape") editing = false;
          }}
        />
        <button type="submit" class="secondary small">{t("groups.name.save")}</button>
      </form>
    {:else}
      <h3>{group.name}</h3>
      <SessionsBadge sessions={group.sessions} />
      {#if group.isDefault}
        <span class="badge">{t("groups.default.badge")}</span>
      {/if}
      <button class="icon" title={t("groups.rename")} aria-label={t("groups.rename")} onclick={startRename}>
        <Icon name="pencil" size={13} />
      </button>
      <span class="spacer"></span>
      <Menu
        title={t("groups.account.menu.help")}
        items={[
          group.isDefault
            ? { label: t("groups.clearDefault"), onSelect: async () => onSnapshot(await api.clearDefault()) }
            : { label: t("groups.makeDefault"), onSelect: () => void askMakeDefault() },
          { label: t("groups.delete"), onSelect: () => (pending = { kind: "deleteGroup" }), destructive: true },
        ]}
      />
    {/if}
  </header>

  <!-- Como abrir este grupo no terminal: a resposta para "e agora, como uso?". -->
  <div class="command">
    <Icon name="terminal" size={13} />
    <code>{group.command}</code>
    <button class="icon" title={t("groups.command.copy.help")} aria-label={t("groups.command.copy.help")} onclick={copyCommand}>
      <Icon name={copied ? "check" : "copy"} size={13} />
    </button>
    {#if integration !== "ok"}
      <!-- A seta cumpre o que promete: leva à seção, no fim da aba. -->
      <button
        class="needs-install"
        onclick={() =>
          document.getElementById("terminal-integration")?.scrollIntoView({ behavior: "smooth", block: "start" })}
      >
        {integration === "install" ? t("groups.command.needsInstall") : t("groups.command.needsAttention")}
      </button>
    {/if}
  </div>

  <div class="controls">
    <Switch
      checked={group.autoRotate}
      label={t("groups.autoRotate")}
      onChange={async (on) => onSnapshot(await api.setAutoRotate(group.id, on))}
    />
    <span class="spacer"></span>
    <label class="threshold">
      <span>{t("groups.threshold.label")}</span>
      <input
        type="range"
        min="50"
        max="100"
        step="5"
        value={threshold}
        oninput={(event) => (threshold = Number(event.currentTarget.value))}
        onchange={async (event) =>
          onSnapshot(await api.setThreshold(group.id, Number(event.currentTarget.value)))}
      />
      <span class="value">{threshold}%</span>
    </label>
  </div>

  <hr />

  {#if ordered.length === 0}
    <p class="empty">{t("groups.accounts.empty")}</p>
  {:else}
    <div class="accounts" bind:this={list}>
      {#each ordered as account, index (account.id)}
        <AccountItem
          {account}
          active={group.activeAccountId === account.id}
          dragging={dragging === account.id}
          first={index === 0}
          last={index === ordered.length - 1}
          onUse={async () => onSnapshot(await api.activateAccount(account.id, group.id))}
          onRelogin={() => onRelogin(account)}
          onRemove={() => (pending = { kind: "removeAccount", account })}
          onMove={(delta) => move(account.id, delta)}
          onDragStart={(event) => dragStart(account.id, event)}
          onDragMove={dragMove}
          onDragEnd={dragEnd}
        />
      {/each}
    </div>
  {/if}

  <footer>
    <button class="link" onclick={onAddAccount}>
      <Icon name="plus" size={13} />{t("groups.account.add")}
    </button>
    <span class="spacer"></span>
    {#if measuring}
      <span class="measuring"><span class="spinner" aria-hidden="true"></span>{t("groups.measuring")}</span>
    {:else}
      <button
        class="link"
        title={t("groups.measure.help")}
        disabled={group.accounts.length === 0 || measuringGroup !== null}
        onclick={async () => onSnapshot(await api.measureGroup(group.id))}
      >
        <Icon name="probe" size={13} />{t("groups.measure")}
      </button>
    {/if}
  </footer>
</article>

{#if pending?.kind === "removeAccount"}
  <ConfirmDialog
    title={t("groups.account.delete.title")}
    message={t("groups.account.delete.message.format", pending.account.label)}
    confirm={t("groups.account.delete.confirm")}
    onConfirm={confirmPending}
    onCancel={() => (pending = null)}
  />
{:else if pending?.kind === "deleteGroup"}
  <ConfirmDialog
    title={t("groups.delete.confirm.title")}
    message={t("groups.delete.confirm.message.format", group.name, group.exclusiveCount)}
    confirm={t("groups.delete.confirm.button")}
    onConfirm={confirmPending}
    onCancel={() => (pending = null)}
  />
{:else if pending?.kind === "makeDefault"}
  <ConfirmDialog
    title={t("groups.makeDefault.confirm.title")}
    message={pending.foreign
      ? t("groups.makeDefault.confirm.foreign.format", pending.foreign)
      : t("groups.makeDefault.confirm.message")}
    confirm={t("groups.makeDefault")}
    destructive={pending.foreign !== null}
    onConfirm={confirmPending}
    onCancel={() => (pending = null)}
  />
{/if}

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
  }
  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
  }
  .badge {
    padding: 1px 6px;
    border-radius: 999px;
    background: color-mix(in srgb, var(--calm) 25%, transparent);
    font-size: var(--font-small);
  }
  .rename {
    display: flex;
    flex: 1;
    gap: 8px;
  }
  .rename .field {
    flex: 1;
  }
  .spacer {
    flex: 1;
  }
  .icon {
    display: inline-flex;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .icon:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .command {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border-radius: 6px;
    background: var(--bg);
    color: var(--text-tertiary);
  }
  code {
    font-family: var(--font-mono);
    font-size: var(--font-caption);
    color: var(--text);
    user-select: text;
  }
  .needs-install {
    padding: 0;
    border: none;
    background: none;
    font: inherit;
    font-size: var(--font-small);
    color: var(--warning-text);
    cursor: pointer;
  }
  .needs-install:hover {
    text-decoration: underline;
  }
  .controls {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .threshold {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
  .threshold input {
    width: 120px;
    accent-color: var(--accent);
  }
  .value {
    width: 34px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }
  hr {
    margin: 0;
    border: none;
    border-top: 1px solid var(--border);
  }
  .empty {
    padding: 4px 0;
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
  .accounts {
    display: flex;
    flex-direction: column;
  }
  footer {
    display: flex;
    align-items: center;
  }
  .link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .link:disabled {
    color: var(--text-tertiary);
    cursor: default;
  }
  .measuring {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--track);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
