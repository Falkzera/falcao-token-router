// O backend simulado: responde aos mesmos comandos do Rust quando a página roda
// no navegador (`npm run dev`). Cenário, idioma, janela e aba vêm da URL —
// `?view=flyout&state=uso&lang=pt-BR` — para cada estado ser aberto de
// propósito e conferido no Chrome. Dados só de exemplo (@exemplo.com, Acme).

import type { AppInfo, GroupView, HomeTab, Locale, Snapshot, UsageView } from "./types";

function param(name: string): string | null {
  return new URLSearchParams(window.location.search).get(name);
}

function mockLocale(): Locale {
  return param("lang") === "pt-BR" ? "pt-BR" : "en";
}

function mockTab(): HomeTab {
  return param("tab") === "settings" ? "settings" : "groups";
}

const minutesAgo = (m: number) => new Date(Date.now() - m * 60_000).toISOString();
const inHours = (h: number) => new Date(Date.now() + h * 3_600_000).toISOString();

/** O % do núcleo (meio para longe do zero), para o mock mostrar o mesmo texto. */
const pct = (f: number) => `${Math.round(f * 100)}%`;

function reading(fraction: number, resetsInHours: number) {
  return { fraction, text: pct(fraction), resetsAt: inHours(resetsInHours) };
}

function usage(
  five: number | null,
  seven: number | null,
  origin: "sensor" | "probe",
  ageMinutes: number,
  model?: { name: string; fraction: number; ageMinutes: number },
): UsageView {
  const candidates: [number, UsageView["bound"]][] = [];
  if (five !== null) candidates.push([five, "fiveHour"]);
  if (seven !== null) candidates.push([seven, "sevenDay"]);
  if (model) candidates.push([model.fraction, "model"]);
  // No empate vale a janela de horizonte mais longo (a última), como no núcleo.
  const [fraction, bound] = candidates.reduce((a, b) => (b[0] >= a[0] ? b : a));
  return {
    fraction,
    text: pct(fraction),
    bound,
    fiveHour: five === null ? null : reading(five, 3),
    sevenDay: seven === null ? null : reading(seven, 50),
    model: model
      ? { name: model.name, reading: reading(model.fraction, 30), sampledAt: minutesAgo(model.ageMinutes) }
      : null,
    origin,
    sampledAt: minutesAgo(ageMinutes),
  };
}

function group(partial: Partial<GroupView> & Pick<GroupView, "id" | "name" | "accounts">): GroupView {
  return {
    isDefault: false,
    autoRotate: true,
    thresholdPercent: 90,
    command: partial.name.includes(" ") ? `claude "${partial.name}"` : `claude ${partial.name.toLowerCase()}`,
    activeAccountId: null,
    exclusiveCount: partial.accounts.length,
    sessions: { count: 0, engaged: 0 },
    ...partial,
  };
}

const work = group({
  id: "G1",
  name: "Trabalho",
  activeAccountId: "A2",
  sessions: { count: 2, engaged: 1 },
  accounts: [
    { id: "A1", label: "equipe-1", email: "equipe-1@exemplo.com", organization: "Acme", usage: usage(0.12, 0.4, "probe", 125) },
    { id: "A2", label: "equipe-2", email: "equipe-2@exemplo.com", organization: "Acme", usage: usage(0.34, 0.81, "sensor", 3) },
    { id: "A3", label: "equipe-3", email: "equipe-3@exemplo.com", organization: "Acme", usage: null },
  ],
});

const personal = group({
  id: "G2",
  name: "Pessoal",
  thresholdPercent: 85,
  activeAccountId: "A4",
  accounts: [
    {
      id: "A4",
      label: "conta1",
      email: "conta1@exemplo.com",
      organization: null,
      usage: usage(0.2, 0.45, "sensor", 8, { name: "Fable", fraction: 0.95, ageMinutes: 75 }),
    },
    { id: "A5", label: "conta2", email: "conta2@exemplo.com", organization: null, usage: usage(null, 0.3, "sensor", 14 * 60) },
  ],
});

const scenarios: Record<string, () => Snapshot> = {
  uso: () => ({ groups: [work, personal], measuringGroup: null, lastError: null }),
  vazio: () => ({ groups: [], measuringGroup: null, lastError: null }),
  pronta: () => ({
    groups: [
      group({
        id: "G1",
        name: "Trabalho",
        activeAccountId: "A1",
        accounts: [{ id: "A1", label: "equipe-1", email: "equipe-1@exemplo.com", organization: "Acme", usage: null }],
      }),
    ],
    measuringGroup: null,
    lastError: null,
  }),
  critico: () => ({
    groups: [
      group({
        id: "G1",
        name: "Trabalho",
        activeAccountId: "A2",
        sessions: { count: 1, engaged: 0 },
        accounts: [
          { id: "A1", label: "equipe-1", email: "equipe-1@exemplo.com", organization: "Acme", usage: usage(0.97, 0.7, "sensor", 1) },
          { id: "A2", label: "equipe-2", email: "equipe-2@exemplo.com", organization: "Acme", usage: usage(0.96, 0.88, "sensor", 2) },
        ],
      }),
    ],
    measuringGroup: null,
    lastError: null,
  }),
  erro: () => ({
    groups: [work, personal],
    measuringGroup: null,
    lastError: { code: "probeFailures", count: 2 },
  }),
};

const handlers: Record<string, (args?: Record<string, unknown>) => unknown> = {
  app_info: (): AppInfo => ({ version: "0.1.0-mock", locale: mockLocale(), initialTab: mockTab() }),
  get_snapshot: (): Snapshot => (scenarios[param("state") ?? "uso"] ?? scenarios.uso!)(),
  fit_flyout: () => undefined,
  open_home: (args) => console.info("mock: abrir a janela na aba", args?.tab),
  quit_app: () => console.info("mock: sair"),
};

export async function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[command];
  if (!handler) throw new Error(`mock: comando desconhecido ${command}`);
  return handler(args) as T;
}
