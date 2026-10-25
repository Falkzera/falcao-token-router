// O tooltip de uma linha de conta, num lugar só (≙ AccountHelp) — o flyout e a
// tela de Grupos contam a mesma história.
//
// Duas origens, duas idades: as janelas de 5h e 7d vêm do sensor (a cada
// mensagem) ou da sonda; a do modelo vem SÓ da sonda, que roda quando alguém
// pede. Uma idade só para as duas afirmaria que o número do Fable é tão fresco
// quanto o das 5 horas — e é justamente o do Fable que trava a conta.

import { ageSeconds, duration, notStarted, resetText } from "../lib/format";
import { t } from "../lib/i18n";
import type { Reading, UsageView } from "../lib/types";

/** Quando cada janela reseta, uma por linha ("5h reseta 22:30 · em 1h 12m") —
 *  o "quando" como a status line o escreve; o que falta, contado de agora. A
 *  de 5h que não começou diz isso, em vez de sumir da lista. */
function resets(usage: UsageView, now: number): string {
  const five = t("panel.accounts.window.fiveHour");
  const lines = notStarted(usage.fiveHour) ? [t("panel.accounts.reset.notStarted.help.format", five)] : [];
  const windows: [string, Reading | null][] = [
    [five, usage.fiveHour],
    [t("panel.accounts.window.sevenDay"), usage.sevenDay],
    [usage.model?.name ?? "", usage.model?.reading ?? null],
  ];
  for (const [name, reading] of windows) {
    if (reading?.resetsAt) lines.push(t("panel.accounts.reset.help.format", name, resetText(reading, now)));
  }
  return lines.join("\n");
}

/** `signedOut`: a conta se mostrou sem login — isso vem primeiro, e sem
 *  amostra ela NÃO está "pronta" (o texto de pronta mandaria medir, e medir
 *  dá no mesmo "sem login"). */
export function accountHelp(usage: UsageView | null, now: number, signedOut = false): string {
  const signedOutLine = signedOut ? t("panel.accounts.signedOut.help") : "";
  if (!usage) return signedOutLine || t("panel.accounts.ready.help.probe");
  const absent = t("panel.accounts.window.absent");
  const lines = [
    signedOutLine,
    t(
      "panel.accounts.usage.help.format",
      usage.fiveHour?.text ?? absent,
      usage.sevenDay?.text ?? absent,
      duration(ageSeconds(usage.sampledAt, now)),
    ),
    resets(usage, now),
    // QUEM mediu, e não só quando: as duas fontes são oficiais e dizem coisas
    // diferentes.
    t(usage.origin === "probe" ? "panel.accounts.origin.probe" : "panel.accounts.origin.sensor"),
  ].filter((line) => line !== "");
  if (usage.model?.sampledAt) {
    lines.push(
      t(
        "panel.accounts.model.help.format",
        usage.model.name,
        usage.model.reading.text,
        duration(ageSeconds(usage.model.sampledAt, now)),
      ),
    );
  }
  // Tooltip nativo não formata nada: a crase do catálogo (código) sai.
  return lines.join("\n\n").replaceAll("`", "");
}

/** O tooltip do relógio de amostra muito velha. */
export function staleHelp(usage: UsageView, now: number): string {
  const age = duration(ageSeconds(usage.sampledAt, now));
  return usage.origin === "probe"
    ? t("panel.accounts.stale.help.probe.format", age)
    : t("panel.accounts.stale.help.sensor.format", age);
}
