// O tooltip de uma linha de conta, num lugar só (≙ AccountHelp) — o flyout e a
// tela de Grupos contam a mesma história.
//
// Duas origens, duas idades: as janelas de 5h e 7d vêm do sensor (a cada
// mensagem) ou da sonda; a do modelo vem SÓ da sonda, que roda quando alguém
// pede. Uma idade só para as duas afirmaria que o número do Fable é tão fresco
// quanto o das 5 horas — e é justamente o do Fable que trava a conta.

import { ageSeconds, duration } from "../lib/format";
import { t } from "../lib/i18n";
import type { UsageView } from "../lib/types";

export function accountHelp(usage: UsageView | null, now: number): string {
  if (!usage) return t("panel.accounts.ready.help.probe");
  const absent = t("panel.accounts.window.absent");
  const lines = [
    t(
      "panel.accounts.usage.help.format",
      usage.fiveHour?.text ?? absent,
      usage.sevenDay?.text ?? absent,
      duration(ageSeconds(usage.sampledAt, now)),
    ),
    // QUEM mediu, e não só quando: as duas fontes são oficiais e dizem coisas
    // diferentes.
    t(usage.origin === "probe" ? "panel.accounts.origin.probe" : "panel.accounts.origin.sensor"),
  ];
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
  return lines.join("\n\n");
}

/** O tooltip do relógio de amostra muito velha. */
export function staleHelp(usage: UsageView, now: number): string {
  const age = duration(ageSeconds(usage.sampledAt, now));
  return usage.origin === "probe"
    ? t("panel.accounts.stale.help.probe.format", age)
    : t("panel.accounts.stale.help.sensor.format", age);
}
