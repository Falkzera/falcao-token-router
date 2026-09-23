// Formatação que a tela divide (≙ Panel/Formatters.swift + UsageColor +
// UsageAge). O PERCENTUAL não mora aqui: ele chega pronto do backend, do
// `UsagePercent` do núcleo, para as telas nunca discordarem por arredondamento.

import { locale, t } from "./i18n";

/** Acima disto o número esmaece: pode já descrever outra realidade. */
export const STALE_SECONDS = 3600;
/** Acima disto ganha marca explícita: meio dia sem medir, em conta
 *  compartilhada, é tempo de sobra para o valor ficar otimista. */
export const VERY_STALE_SECONDS = 12 * 3600;

/** "1h 12m" / "3m" (≙ `Format.duration`). */
export function duration(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  return hours > 0
    ? t("format.duration.hoursMinutes", hours, minutes)
    : t("format.duration.minutes", minutes);
}

/** Segundos desde uma data ISO até `now` (ms). */
export function ageSeconds(iso: string, now: number): number {
  return (now - Date.parse(iso)) / 1000;
}

/** "13:20" — a hora do reset, no formato do idioma. */
export function clockTime(iso: string): string {
  return new Intl.DateTimeFormat(locale(), { hour: "2-digit", minute: "2-digit" }).format(
    new Date(iso),
  );
}

/** "reseta 13:20 · em 4h 6m" — o tempo restante contado de agora: a amostra
 *  pode ser de horas atrás, mas o RESET é uma hora do relógio e não envelhece
 *  junto com ela. */
export function resetText(resetsAt: string | null, now: number): string {
  if (!resetsAt) return "";
  const base = t("panel.reset.format", clockTime(resetsAt));
  const remaining = (Date.parse(resetsAt) - now) / 1000;
  return remaining > 0 ? t("panel.reset.remaining.format", base, duration(remaining)) : base;
}

/** O semáforo (≙ `UsageColor.bar`): os mesmos limiares da bandeja. */
export type Severity = "calm" | "warning" | "critical";

export function severity(fraction: number): Severity {
  if (fraction < 0.66) return "calm";
  if (fraction < 0.9) return "warning";
  return "critical";
}
