<script lang="ts">
  // Uma conta no cartão do grupo (≙ AccountRow de GroupsView.swift). A ordem é a
  // preferência de rotação: a alça arrasta, e com o foco nela as setas ↑/↓
  // movem — o reordenar do macOS (`onMove` fora de uma `List`) não funcionava.
  // "Usar" fica exposto (é a ação frequente); o resto, no menu.
  import { clock } from "../lib/clock.svelte";
  import { ageSeconds, severity, VERY_STALE_SECONDS } from "../lib/format";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import Menu from "../lib/Menu.svelte";
  import type { AccountView, Reading } from "../lib/types";
  import { accountHelp, staleHelp } from "../panel/accountHelp";
  import ModelBadge from "../panel/ModelBadge.svelte";

  let {
    account,
    active,
    dragging,
    first,
    last,
    onUse,
    onRelogin,
    onRemove,
    onMove,
    onDragStart,
    onDragMove,
    onDragEnd,
  }: {
    account: AccountView;
    active: boolean;
    dragging: boolean;
    first: boolean;
    last: boolean;
    onUse: () => void;
    onRelogin: () => void;
    onRemove: () => void;
    onMove: (delta: number) => void;
    onDragStart: (event: PointerEvent) => void;
    onDragMove: (event: PointerEvent) => void;
    onDragEnd: () => void;
  } = $props();

  const usage = $derived(account.usage);
  const veryStale = $derived(usage ? ageSeconds(usage.sampledAt, clock.now) > VERY_STALE_SECONDS : false);
  const help = $derived(accountHelp(usage, clock.now));

  function tone(reading: Reading, bound: boolean): string {
    return bound ? severity(reading.fraction) : "plain";
  }
</script>

<div class="item" class:dragging data-account={account.id}>
  <button
    class="handle"
    title={t("groups.reorder.help")}
    aria-label={t("groups.reorder.help")}
    onpointerdown={onDragStart}
    onpointermove={onDragMove}
    onpointerup={onDragEnd}
    onpointercancel={onDragEnd}
    onkeydown={(event) => {
      if (event.key === "ArrowUp" && !first) {
        event.preventDefault();
        onMove(-1);
      } else if (event.key === "ArrowDown" && !last) {
        event.preventDefault();
        onMove(1);
      }
    }}
  >
    <svg width="10" height="14" viewBox="0 0 10 14" fill="currentColor" aria-hidden="true">
      <circle cx="3" cy="3" r="1.1" /><circle cx="7" cy="3" r="1.1" />
      <circle cx="3" cy="7" r="1.1" /><circle cx="7" cy="7" r="1.1" />
      <circle cx="3" cy="11" r="1.1" /><circle cx="7" cy="11" r="1.1" />
    </svg>
  </button>

  <span class="dot {active ? severity(usage?.fraction ?? 0) : 'idle'}" aria-hidden="true"></span>

  <span class="who" title={help}>
    <span class="label" class:active>{account.label}</span>
    {#if account.organization}
      <span class="org">{account.organization}</span>
    {/if}
  </span>

  {#if usage?.bound === "model" && usage.model}
    <ModelBadge model={usage.model} />
  {/if}
  {#if veryStale && usage}
    <span class="clock" title={staleHelp(usage, clock.now)}><Icon name="clockAlert" size={12} /></span>
  {/if}

  {#if usage}
    <span class="readings" title={help}>
      {#each [{ key: "fiveHour" as const, reading: usage.fiveHour }, { key: "sevenDay" as const, reading: usage.sevenDay }] as window (window.key)}
        <span class="reading">
          <span class="window">
            {window.key === "fiveHour" ? t("panel.accounts.window.fiveHour") : t("panel.accounts.window.sevenDay")}
          </span>
          {#if window.reading}
            <span class="num {tone(window.reading, usage.bound === window.key)}" class:bound={usage.bound === window.key}>
              {window.reading.text}
            </span>
          {:else}
            <span class="num absent">{t("panel.accounts.window.absent")}</span>
          {/if}
        </span>
      {/each}
    </span>
  {:else}
    <span class="unmeasured" title={help}>{t("groups.account.unmeasured")}</span>
  {/if}

  <span class="actions">
    <!-- Na ativa o botão só esconde (não sai): o espaço dele mantém os
         números alinhados em coluna de conta para conta. -->
    <button
      class="secondary small"
      class:placeholder={active}
      disabled={active}
      aria-hidden={active}
      tabindex={active ? -1 : 0}
      onclick={onUse}
    >
      {t("groups.account.use")}
    </button>
    <Menu
      title={t("groups.account.menu.help")}
      items={[
        { label: t("groups.account.moveUp"), onSelect: () => onMove(-1), disabled: first },
        { label: t("groups.account.moveDown"), onSelect: () => onMove(1), disabled: last },
        { label: t("groups.account.relogin"), onSelect: onRelogin },
        { label: t("groups.account.remove"), onSelect: onRemove, destructive: true },
      ]}
    />
  </span>
</div>

<style>
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 36px;
    padding: 2px 0;
    border-radius: 6px;
  }
  .item.dragging {
    background: var(--row-selected);
  }
  .handle {
    display: inline-flex;
    padding: 4px 2px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-tertiary);
    cursor: grab;
    touch-action: none;
  }
  .dragging .handle {
    cursor: grabbing;
  }
  .handle:focus-visible {
    outline: 2px solid var(--focus);
  }
  .dot {
    flex: none;
    width: 7px;
    height: 7px;
    border-radius: 50%;
  }
  .dot.idle {
    border: 1px solid var(--text-quaternary);
  }
  .dot.calm {
    background: var(--calm);
  }
  .dot.warning {
    background: var(--warning);
  }
  .dot.critical {
    background: var(--critical);
  }
  .who {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .label.active {
    font-weight: 600;
  }
  .org {
    font-size: var(--font-small);
    color: var(--text-secondary);
  }
  .clock {
    display: inline-flex;
    color: var(--text-tertiary);
  }
  .readings {
    display: flex;
    gap: 10px;
  }
  .reading {
    display: inline-flex;
    gap: 3px;
    font-size: var(--font-small);
    font-variant-numeric: tabular-nums;
  }
  .window {
    color: var(--text-tertiary);
  }
  .num {
    min-width: 28px;
    text-align: right;
    color: var(--text-secondary);
  }
  .num.bound {
    font-weight: 600;
  }
  .num.calm {
    color: var(--calm);
  }
  .num.warning {
    color: var(--warning-text);
  }
  .num.critical {
    color: var(--critical);
  }
  .num.absent {
    color: var(--text-quaternary);
  }
  .unmeasured {
    font-size: var(--font-small);
    color: var(--text-tertiary);
  }
  .actions {
    display: inline-flex;
    align-items: center;
    gap: 2px;
  }
  .placeholder {
    visibility: hidden;
  }
</style>
