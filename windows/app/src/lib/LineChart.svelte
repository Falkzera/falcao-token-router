<script lang="ts">
  // Um gráfico de linha do Histórico (21/10/2026): uma série — a fração de uma
  // janela ao longo do tempo —, eixo vertical FIXO de 0 a 100% (as contas se
  // comparam de relance) e o limiar do grupo tracejado (é com ele que a
  // rotação compara). SVG à mão, como o `Gauge`: sem dependência.
  //
  // As marcas seguem o método de visualização do projeto: linha de 2 px, ponto
  // de 8 px com anel de 2 px da cor da superfície, grade em traço fino e
  // CONTÍNUO — o tracejado é só do limiar, que é um limite. Muitos pontos (o
  // semanal tem ~130) não ganham bolinha cada: ela fica para o último, para o
  // ponto isolado (trecho de um ponto só) e para o que o ponteiro ou o teclado
  // escolher. A dica segue o ponteiro até o ponto mais perto no tempo (←/→
  // pelo teclado), e uma tabela escondida leva todos os pontos ao leitor de tela.
  import { t } from "./i18n";
  import type { HistoryPoint, HistorySeries } from "./types";

  let { title, series, threshold }: { title: string; series: HistorySeries; threshold: number } =
    $props();

  const HEIGHT = 132;
  const PAD = { top: 10, right: 10, bottom: 22, left: 40 };
  const PLOT_BOTTOM = HEIGHT - PAD.bottom;

  let width = $state(0);
  let active = $state<number | null>(null);

  const from = $derived(Date.parse(series.from));
  const span = $derived(Math.max(1, Date.parse(series.to) - from));
  const right = $derived(Math.max(PAD.left, width - PAD.right));
  const points = $derived(series.segments.flat());
  const shown = $derived(active === null ? null : (points[active] ?? null));
  const latest = $derived(series.latest?.text ?? t("panel.accounts.window.absent"));

  function x(at: string): number {
    return PAD.left + ((Date.parse(at) - from) / span) * (right - PAD.left);
  }
  function y(fraction: number): number {
    return PAD.top + (1 - Math.min(1, Math.max(0, fraction))) * (PLOT_BOTTOM - PAD.top);
  }
  function path(segment: HistoryPoint[]): string {
    return segment
      .map((p, i) => `${i === 0 ? "M" : "L"}${x(p.at).toFixed(1)},${y(p.fraction).toFixed(1)}`)
      .join(" ");
  }
  /** O ponto mais perto no tempo de uma posição x: o ponteiro não precisa
   *  acertar a linha. */
  function nearest(px: number): number | null {
    let best: number | null = null;
    let distance = Infinity;
    points.forEach((p, i) => {
      const d = Math.abs(x(p.at) - px);
      if (d < distance) {
        distance = d;
        best = i;
      }
    });
    return best;
  }
  function describe(p: HistoryPoint): string {
    const reset = p.resetsLabel ? t("panel.reset.format", p.resetsLabel) : null;
    return [p.text, p.label, reset].filter(Boolean).join(" · ");
  }
  function onKey(event: KeyboardEvent) {
    const last = points.length - 1;
    if (last < 0) return;
    const current = active ?? last;
    const moves: Record<string, number> = {
      ArrowLeft: Math.max(0, current - 1),
      ArrowRight: Math.min(last, current + 1),
      Home: 0,
      End: last,
    };
    const next = moves[event.key];
    if (next !== undefined) {
      active = next;
      event.preventDefault();
    } else if (event.key === "Escape") {
      active = null;
    }
  }

  const limitY = $derived(y(threshold / 100));
  /** O rótulo do limiar vai acima da linha; colado no topo, abaixo dela. */
  const limitLabelY = $derived(limitY - PAD.top < 12 ? limitY + 11 : limitY - 4);
  const tipX = $derived(shown ? x(shown.at) : 0);
  /** A dica fica do lado com mais espaço. */
  const tipOnLeft = $derived(tipX > width / 2);
</script>

<figure class="chart">
  <figcaption>
    <span class="title">{title}</span>
    <span class="latest">{latest}</span>
  </figcaption>
  <!-- O gráfico é navegável pelo teclado (←/→) e mostra no foco a mesma dica
       do ponteiro: um grupo focável, de propósito. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="plot"
    role="group"
    tabindex={points.length > 0 ? 0 : undefined}
    aria-label={t("history.chart.label.format", title, latest)}
    bind:clientWidth={width}
    onkeydown={onKey}
    onfocus={() => (active = points.length > 0 ? (active ?? points.length - 1) : null)}
    onblur={() => (active = null)}
    onpointermove={(event) => (active = nearest(event.clientX - event.currentTarget.getBoundingClientRect().left))}
    onpointerleave={() => (active = null)}
  >
    {#if width > 0}
      <svg {width} height={HEIGHT} aria-hidden="true">
        {#each [0, 0.5, 1] as level (level)}
          <line class="grid" class:base={level === 0} x1={PAD.left} x2={right} y1={y(level)} y2={y(level)} />
          <text class="axis" x={PAD.left - 6} y={y(level)} text-anchor="end" dominant-baseline="middle"
            >{`${level * 100}%`}</text
          >
        {/each}
        {#each series.ticks as tick (tick.at)}
          {@const tx = x(tick.at)}
          {@const atEnd = tx > right - 34}
          <line class="grid" x1={tx} x2={tx} y1={PAD.top} y2={PLOT_BOTTOM} />
          <text class="axis" x={atEnd ? tx - 3 : tx + 3} y={HEIGHT - 6} text-anchor={atEnd ? "end" : "start"}
            >{tick.label}</text
          >
        {/each}
        {#if threshold > 0 && threshold <= 100}
          <line class="limit" x1={PAD.left} x2={right} y1={limitY} y2={limitY} />
          <text class="limit-label" x={right} y={limitLabelY} text-anchor="end"
            >{t("history.chart.threshold.format", `${Math.round(threshold)}%`)}</text
          >
        {/if}
        {#each series.segments as segment, i (i)}
          {#if segment.length > 1}
            <path class="line" d={path(segment)} />
          {:else if segment[0]}
            <circle class="dot" cx={x(segment[0].at)} cy={y(segment[0].fraction)} r="5" />
          {/if}
        {/each}
        {#if series.latest}
          <circle class="dot" cx={x(series.latest.at)} cy={y(series.latest.fraction)} r="5" />
        {/if}
        {#if shown}
          <line class="crosshair" x1={tipX} x2={tipX} y1={PAD.top} y2={PLOT_BOTTOM} />
          <circle class="dot" cx={tipX} cy={y(shown.fraction)} r="5" />
        {/if}
      </svg>
    {/if}
    {#if points.length === 0}
      <p class="empty">{t("history.chart.empty")}</p>
    {/if}
    {#if shown}
      <div
        class="tip"
        style:left={tipOnLeft ? null : `${tipX + 10}px`}
        style:right={tipOnLeft ? `${width - tipX + 10}px` : null}
      >
        <strong>{shown.text}</strong>
        <span>{shown.label}</span>
        {#if shown.resetsLabel}
          <span>{t("panel.reset.format", shown.resetsLabel)}</span>
        {/if}
      </div>
    {/if}
  </div>
  <p class="sr-only" aria-live="polite">{shown ? describe(shown) : ""}</p>
  <table class="sr-only">
    <caption>{title}</caption>
    <tbody>
      {#each points as point, i (i)}
        <tr><td>{describe(point)}</td></tr>
      {/each}
    </tbody>
  </table>
</figure>

<style>
  .chart {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
  }
  figcaption {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    font-size: var(--font-caption);
  }
  .title {
    color: var(--text-secondary);
  }
  .latest {
    font-weight: 600;
    color: var(--text);
  }
  .plot {
    position: relative;
    height: 132px;
    border-radius: 4px;
    touch-action: pan-y;
  }
  .plot:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: 2px;
  }
  svg {
    display: block;
    overflow: visible;
  }
  .grid {
    stroke: var(--border);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .grid.base {
    stroke: var(--border-strong);
  }
  .axis {
    fill: var(--text-tertiary);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }
  .limit {
    stroke: var(--critical);
    stroke-width: 1;
    stroke-dasharray: 4 3;
    opacity: 0.75;
  }
  .limit-label {
    fill: var(--text-secondary);
    font-size: 10px;
  }
  .line {
    fill: none;
    stroke: var(--chart-line);
    stroke-width: 2;
    stroke-linejoin: round;
    stroke-linecap: round;
  }
  .dot {
    fill: var(--chart-line);
    stroke: var(--surface);
    stroke-width: 2;
  }
  .crosshair {
    stroke: var(--text-tertiary);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .empty {
    position: absolute;
    inset: 0 0 22px 40px;
    display: grid;
    place-items: center;
    font-size: var(--font-caption);
    color: var(--text-tertiary);
    pointer-events: none;
  }
  .tip {
    position: absolute;
    top: 4px;
    display: flex;
    flex-direction: column;
    gap: 1px;
    padding: 5px 8px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: var(--surface);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.14);
    font-size: var(--font-small);
    color: var(--text-secondary);
    white-space: nowrap;
    pointer-events: none;
  }
  .tip strong {
    font-size: var(--font-body);
    color: var(--text);
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }
</style>
