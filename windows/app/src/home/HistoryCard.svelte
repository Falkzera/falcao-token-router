<script lang="ts">
  // Uma conta no Histórico: quem é, quando foi medida (e quando volta a ser,
  // com a agenda ligada) e os dois gráficos, com o limiar do grupo. Os
  // `data-*` são o endereço do atalho de cada conta; `highlight`, o destaque de
  // quem chegou por ele.
  import { clock } from "../lib/clock.svelte";
  import { ageSeconds, duration } from "../lib/format";
  import { t } from "../lib/i18n";
  import LineChart from "../lib/LineChart.svelte";
  import type { AccountHistory } from "../lib/types";

  let {
    account,
    group,
    threshold,
    scheduleOn,
    highlight = false,
  }: { account: AccountHistory; group: string; threshold: number; scheduleOn: boolean; highlight?: boolean } =
    $props();

  const measured = $derived(
    account.lastMeasuredAt
      ? t("history.account.measured.format", duration(ageSeconds(account.lastMeasuredAt, clock.now)))
      : t("history.account.never"),
  );
  // A hora da próxima vem escrita do Rust. Vencida, ou a menos de uma volta da
  // agenda (ela olha a cada 3 min — e o relógio da tela anda de 30 em 30 s,
  // atrás da hora do quadro), "em instantes": senão uma conta recém-zerada
  // diria "próxima às" a hora de agora.
  const SOON_MS = 3 * 60_000;
  const next = $derived(
    !scheduleOn
      ? t("history.account.scheduleOff")
      : account.nextMeasureAt && account.nextMeasureLabel && Date.parse(account.nextMeasureAt) > clock.now + SOON_MS
        ? t("history.account.next.format", account.nextMeasureLabel)
        : t("history.account.nextSoon"),
  );
</script>

<article class="card" class:highlight data-account={account.id} data-group={group}>
  <header>
    <span class="label">{account.label}</span>
    <span class="email">{account.email}</span>
  </header>
  <p class="status">{[measured, next].join(" · ")}</p>
  <LineChart title={t("history.chart.fiveHour")} series={account.fiveHour} {threshold} />
  <LineChart title={t("history.chart.sevenDay")} series={account.sevenDay} {threshold} />
</article>

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    scroll-margin-top: 12px;
  }
  /* Quem chegou pelo atalho: um contorno que some sozinho. */
  .card.highlight {
    animation: flash 1.8s ease-out;
  }
  @keyframes flash {
    from {
      box-shadow: 0 0 0 2px var(--accent);
    }
    to {
      box-shadow: 0 0 0 2px transparent;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .card.highlight {
      animation: none;
      box-shadow: 0 0 0 2px var(--accent);
    }
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 8px;
    min-width: 0;
  }
  .label {
    font-weight: 600;
  }
  .email {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--font-caption);
    color: var(--text-secondary);
  }
  .status {
    margin-top: -8px;
    font-size: var(--font-caption);
    color: var(--text-tertiary);
  }
</style>
