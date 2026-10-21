<script lang="ts">
  // Uma conta no Histórico: quem é, quando foi medida (e quando volta a ser,
  // com a agenda ligada) e os dois gráficos, com o limiar do grupo.
  import { clock } from "../lib/clock.svelte";
  import { ageSeconds, duration } from "../lib/format";
  import { t } from "../lib/i18n";
  import LineChart from "../lib/LineChart.svelte";
  import type { AccountHistory } from "../lib/types";

  let { account, threshold, scheduleOn }: { account: AccountHistory; threshold: number; scheduleOn: boolean } =
    $props();

  const measured = $derived(
    account.lastMeasuredAt
      ? t("history.account.measured.format", duration(ageSeconds(account.lastMeasuredAt, clock.now)))
      : t("history.account.never"),
  );
  // A hora da próxima vem escrita do Rust; vencida (a agenda ainda não chegou
  // nela), "em instantes".
  const next = $derived(
    !scheduleOn
      ? t("history.account.scheduleOff")
      : account.nextMeasureAt && account.nextMeasureLabel && Date.parse(account.nextMeasureAt) > clock.now
        ? t("history.account.next.format", account.nextMeasureLabel)
        : t("history.account.nextSoon"),
  );
</script>

<article class="card">
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
