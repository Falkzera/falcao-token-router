// O histórico do backend simulado (`get_history` no navegador, 21/10/2026): um
// ponto de 5 em 5 horas nas últimas 4 semanas para cada conta do quadro, com a
// forma que a sonda veria — a janela de 5h alta no horário de trabalho e sem
// janela aberta de madrugada; a semanal subindo até o reset de segunda 9:00 e
// voltando perto de zero. Nada sorteado: a mesma conta, no mesmo bloco de 5
// horas, dá o mesmo gráfico a cada releitura. As séries saem como no Rust
// (`history.rs`): recorte de 7 e 28 dias, mais de 7h30 sem ponto quebra a
// linha, marcas por dia e por semana.
// Cenários pela URL: `&historico=cheio|vazio|lacuna|desligado` (cheio = padrão).

import type {
  AccountHistory,
  GroupView,
  HistoryPoint,
  HistorySeries,
  HistoryTick,
  HistoryView,
  Locale,
} from "./types";

const HOUR = 3_600_000;
const DAY = 24 * HOUR;
const EVERY = 5 * HOUR;
const MAX_GAP = 7.5 * HOUR;

const WEEKDAYS: Record<Locale, string[]> = {
  "pt-BR": ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"],
  en: ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
};
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

const iso = (ms: number) => new Date(ms).toISOString().replace(/\.\d{3}Z$/, "Z");
const pct = (f: number) => `${Math.round(f * 100)}%`;
const two = (n: number) => String(n).padStart(2, "0");

/** O "quando" como o núcleo escreve (`reset_when`): só a hora, ou com o dia. */
function when(ms: number, withDay: boolean, locale: Locale): string {
  const at = new Date(ms);
  return withDay
    ? `${WEEKDAYS[locale][at.getDay()] ?? ""} (${at.getDate()}) ${at.getHours()}:${two(at.getMinutes())}`
    : `${two(at.getHours())}:${two(at.getMinutes())}`;
}

/** Um número estável entre 0 e 1 por conta e por ponto. */
function noise(seed: number, k: number): number {
  const x = Math.sin(seed * 97.13 + k * 12.9898) * 43758.5453;
  return x - Math.floor(x);
}

/** O último reset semanal do mock antes de `ms`: segunda, 9:00. */
function weeklyReset(ms: number): number {
  const at = new Date(ms);
  at.setDate(at.getDate() - ((at.getDay() + 6) % 7));
  at.setHours(9, 0, 0, 0);
  if (at.getTime() > ms) at.setDate(at.getDate() - 7);
  return at.getTime();
}

interface Raw {
  at: number;
  five: number;
  fiveReset: number | null;
  seven: number;
  sevenReset: number;
}

function raw(id: string, now: number, scenario: string | null): Raw[] {
  if (scenario === "vazio") return [];
  const seed = [...id].reduce((sum, ch) => sum + ch.charCodeAt(0), 0);
  const intensity = 0.55 + 0.4 * noise(seed, 0);
  const anchor = Math.floor(now / EVERY) * EVERY - 20 * 60_000;
  const points: Raw[] = [];
  for (let k = Math.floor((28 * DAY) / EVERY); k >= 0; k--) {
    const at = anchor - k * EVERY;
    if (scenario === "lacuna" && at > now - 60 * HOUR && at < now - 30 * HOUR) continue;
    const hour = new Date(at).getHours();
    const working = hour >= 9 && hour < 20;
    const sinceReset = (at - weeklyReset(at)) / (7 * DAY);
    points.push({
      at,
      // De madrugada a janela de 5h não começou: 0% e sem reset, como o `/usage`.
      five: working ? Math.min(1, intensity * (0.3 + 0.7 * noise(seed, k))) : 0,
      fiveReset: working ? at + (1 + 3 * noise(seed, k + 500)) * HOUR : null,
      seven: Math.min(1, 0.02 + sinceReset * intensity * (0.85 + 0.3 * noise(seed, k + 900))),
      sevenReset: weeklyReset(at) + 7 * DAY,
    });
  }
  return points;
}

function ticks(from: number, now: number, weekly: boolean, locale: Locale): HistoryTick[] {
  const today = new Date(now);
  today.setHours(0, 0, 0, 0);
  const out: HistoryTick[] = [];
  for (let back = 0; back <= (weekly ? 28 : 7); back += weekly ? 7 : 1) {
    const day = new Date(today);
    day.setDate(today.getDate() - back);
    if (day.getTime() <= from) continue;
    const label = !weekly
      ? (WEEKDAYS[locale][day.getDay()] ?? "")
      : locale === "pt-BR"
        ? `${two(day.getDate())}/${two(day.getMonth() + 1)}`
        : `${MONTHS[day.getMonth()] ?? ""} ${day.getDate()}`;
    out.unshift({ at: iso(day.getTime()), label });
  }
  return out;
}

function series(points: Raw[], weekly: boolean, now: number, locale: Locale): HistorySeries {
  const from = now - (weekly ? 28 : 7) * DAY;
  const segments: HistoryPoint[][] = [];
  let previous: number | null = null;
  for (const p of points) {
    if (p.at < from) continue;
    const fraction = weekly ? p.seven : p.five;
    const reset = weekly ? p.sevenReset : p.fiveReset;
    const drawn: HistoryPoint = {
      at: iso(p.at),
      fraction,
      text: pct(fraction),
      label: when(p.at, true, locale),
      resetsLabel: reset === null ? null : when(reset, weekly, locale),
    };
    const current = segments.at(-1);
    if (current && previous !== null && p.at - previous <= MAX_GAP) current.push(drawn);
    else segments.push([drawn]);
    previous = p.at;
  }
  return {
    from: iso(from),
    to: iso(now),
    ticks: ticks(from, now, weekly, locale),
    segments,
    latest: segments.at(-1)?.at(-1) ?? null,
  };
}

/** A conta da UI não carrega e-mail; como no `mock.ts`, ele sai do rótulo. */
function account(
  a: { id: string; label: string },
  scenario: string | null,
  scheduleOn: boolean,
  now: number,
  locale: Locale,
): AccountHistory {
  const points = raw(a.id, now, scenario);
  const last = points.at(-1)?.at ?? null;
  const next = scheduleOn ? (last === null ? now : last + EVERY) : null;
  return {
    id: a.id,
    label: a.label,
    email: `${a.label}@exemplo.com`,
    lastMeasuredAt: last === null ? null : iso(last),
    nextMeasureAt: next === null ? null : iso(next),
    nextMeasureLabel: next === null ? null : when(next, false, locale),
    fiveHour: series(points, false, now, locale),
    sevenDay: series(points, true, now, locale),
  };
}

export function mockHistory(
  groups: GroupView[],
  locale: Locale,
  scenario: string | null,
  scheduleOn: boolean,
): HistoryView {
  const now = Date.now();
  return {
    scheduleOn,
    groups: groups.map((g) => ({
      id: g.id,
      name: g.name,
      thresholdPercent: g.thresholdPercent,
      accounts: g.accounts.map((a) => account(a, scenario, scheduleOn, now, locale)),
    })),
  };
}
