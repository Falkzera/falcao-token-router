// O backend simulado: responde aos mesmos comandos do Rust quando a página roda
// no navegador (`npm run dev`), com o quadro em memória — as ações mudam o
// quadro e avisam a tela, como o backend de verdade. Cenário, idioma, janela,
// aba e seleção vêm da URL:
//   ?view=home|flyout  &state=uso|vazio|pronta|critico|erro  &lang=en|pt-BR
//   &tab=groups|settings  &select=<conta>  &foreign=<e-mail>  &scripts=current|missing|stale
// Dados só de exemplo (@exemplo.com, Acme).

import type { AppInfo, GroupView, HomeTab, Locale, ScriptsState, Snapshot, UsageView } from "./types";

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

const commandFor = (name: string) =>
  name.includes(" ") ? `claude "${name}"` : `claude ${name.toLowerCase()}`;

function group(partial: Partial<GroupView> & Pick<GroupView, "id" | "name" | "accounts">): GroupView {
  return {
    isDefault: false,
    autoRotate: true,
    thresholdPercent: 90,
    command: commandFor(partial.name),
    activeAccountId: null,
    exclusiveCount: partial.accounts.length,
    sessions: { count: 0, engaged: 0 },
    ...partial,
  };
}

function work(): GroupView {
  return group({
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
}

function personal(): GroupView {
  return group({
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
}

const scenarios: Record<string, () => GroupView[]> = {
  uso: () => [work(), personal()],
  vazio: () => [],
  pronta: () => [
    group({
      id: "G1",
      name: "Trabalho",
      activeAccountId: "A1",
      accounts: [{ id: "A1", label: "equipe-1", email: "equipe-1@exemplo.com", organization: "Acme", usage: null }],
    }),
  ],
  critico: () => [
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
  erro: () => [work(), personal()],
};

const scenario = param("state") ?? "uso";
const state: Snapshot = {
  groups: (scenarios[scenario] ?? scenarios.uso!)(),
  measuringGroup: null,
  lastError: scenario === "erro" ? { code: "probeFailures", count: 2 } : null,
  scripts: (param("scripts") as ScriptsState | null) ?? "current",
};

// MARK: - Eventos (o `snapshot-changed` do backend)

const listeners = new Set<() => void>();
export function mockListen(handler: () => void): () => void {
  listeners.add(handler);
  return () => listeners.delete(handler);
}
function changed(): Snapshot {
  setTimeout(() => listeners.forEach((l) => l()), 0);
  return structuredClone(state);
}

function findGroup(id: unknown): GroupView | undefined {
  return state.groups.find((g) => g.id === id);
}

let nextId = 100;

const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
  app_info: (): AppInfo => ({ version: "0.1.0-mock", locale: mockLocale(), initialTab: mockTab() }),
  get_snapshot: () => structuredClone(state),
  fit_flyout: () => undefined,
  open_home: (args) => console.info("mock: abrir a janela na aba", args.tab),
  quit_app: () => console.info("mock: sair"),
  copy_text: async (args) => {
    await navigator.clipboard?.writeText(String(args.text)).catch(() => undefined);
  },
  foreign_default_login: () => param("foreign"),
  add_group: (args) => {
    const name = String(args.name).trim();
    if (!name) return changed();
    if (args.asDefault) state.groups.forEach((g) => (g.isDefault = false));
    state.groups.push(group({ id: `G${nextId++}`, name, isDefault: Boolean(args.asDefault), accounts: [] }));
    return changed();
  },
  rename_group: (args) => {
    const g = findGroup(args.groupId);
    if (g) {
      g.name = String(args.name);
      g.command = commandFor(g.name);
    }
    return changed();
  },
  set_auto_rotate: (args) => {
    const g = findGroup(args.groupId);
    if (g) g.autoRotate = Boolean(args.on);
    return changed();
  },
  set_threshold: (args) => {
    const g = findGroup(args.groupId);
    if (g) g.thresholdPercent = Math.min(100, Math.max(50, Number(args.percent)));
    console.info("mock: limiar gravado", args.percent);
    return changed();
  },
  reorder_accounts: (args) => {
    const g = findGroup(args.groupId);
    const ids = args.accountIds as string[];
    if (g) g.accounts.sort((a, b) => ids.indexOf(a.id) - ids.indexOf(b.id));
    return changed();
  },
  remove_group: (args) => {
    state.groups = state.groups.filter((g) => g.id !== args.groupId);
    return changed();
  },
  make_default: (args) => {
    state.groups.forEach((g) => (g.isDefault = g.id === args.groupId));
    return changed();
  },
  clear_default: () => {
    state.groups.forEach((g) => (g.isDefault = false));
    return changed();
  },
  activate_account: (args) => {
    const g = findGroup(args.groupId);
    if (g) g.activeAccountId = String(args.accountId);
    state.lastError = null;
    return changed();
  },
  remove_account: (args) => {
    for (const g of state.groups) {
      g.accounts = g.accounts.filter((a) => a.id !== args.accountId);
      if (g.activeAccountId === args.accountId) g.activeAccountId = null;
    }
    return changed();
  },
  dismiss_error: () => {
    state.lastError = null;
    return changed();
  },
  measure_group: (args) => {
    state.measuringGroup = String(args.groupId);
    setTimeout(() => {
      state.measuringGroup = null;
      changed();
    }, 2500);
    return changed();
  },
};

/** As chamadas feitas ao backend simulado, para a conferência no navegador
 *  contar (ex.: o limiar grava UMA vez, ao soltar o controle). */
const calls: { command: string; args: Record<string, unknown> }[] = [];
(window as unknown as { __mockCalls: typeof calls }).__mockCalls = calls;

export async function mockInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const handler = handlers[command];
  if (!handler) throw new Error(`mock: comando desconhecido ${command}`);
  calls.push({ command, args: args ?? {} });
  return (await handler(args ?? {})) as T;
}
