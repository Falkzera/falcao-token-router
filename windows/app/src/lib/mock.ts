// O backend simulado: responde aos mesmos comandos do Rust quando a página roda
// no navegador (`npm run dev`), com o quadro em memória — as ações mudam o
// quadro e avisam a tela, como o backend de verdade. Cenário, idioma, janela,
// aba e seleção vêm da URL:
//   ?view=home|flyout  &state=uso|vazio|pronta|critico|erro  &lang=en|pt-BR
//   &tab=groups|settings  &select=<conta>  &foreign=<e-mail>  &scripts=current|missing|stale
// A integração de terminal e os ajustes:
//   &terminal=ausente|ok|bloqueado|parcial|velha|semrouter  &devmode=1
//   &install=falha (Ativar grava só parte)  &diretiva=1 (Permitir não vence a política)
//   &autostart=falha (o Windows recusa o registro)  &taskbar=1
// Dados só de exemplo (@exemplo.com, Acme, C:\Users\exemplo).

import type {
  AppInfo,
  GroupView,
  HomeTab,
  InstallResult,
  Locale,
  ScriptsState,
  SettingsView,
  ShellName,
  ShellView,
  Snapshot,
  TerminalView,
  UsageView,
} from "./types";

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

// MARK: - Integração de terminal

const PROFILES: Record<ShellName, string> = {
  powerShell7: "C:\\Users\\exemplo\\Documents\\PowerShell\\Microsoft.PowerShell_profile.ps1",
  windowsPowerShell: "C:\\Users\\exemplo\\Documents\\WindowsPowerShell\\Microsoft.PowerShell_profile.ps1",
  gitBash: "C:\\Users\\exemplo\\.bashrc",
};

/** Um shell com a integração no lugar; o cenário estraga o que quiser. */
function shellView(shell: ShellName, partial: Partial<ShellView> = {}): ShellView {
  const bash = shell === "gitBash";
  return {
    shell,
    profile: PROFILES[shell],
    loadsIntegration: true,
    policy: bash ? null : "RemoteSigned",
    policyBlocks: false,
    chainsUserFunction: false,
    bashLogin: bash ? "loads" : null,
    bashLoginFile: bash ? ".bash_profile" : null,
    ...partial,
  };
}

/** As mesmas contas do `view` do Rust (`terminal.rs`). */
function terminalView(scripts: ScriptsState, shells: ShellView[], routerFound = true): TerminalView {
  return {
    routerFound,
    scripts,
    shells,
    developerMode: param("devmode") === "1",
    fullyInstalled:
      routerFound &&
      scripts === "current" &&
      shells.every((s) => s.loadsIntegration && !s.policyBlocks && s.bashLogin !== "ignores"),
    blockedByPolicy: shells.some((s) => s.policyBlocks),
    needsInstall: scripts !== "current" || shells.some((s) => !s.loadsIntegration),
  };
}

const notLoaded = { loadsIntegration: false, policy: null };

const terminals: Record<string, () => TerminalView> = {
  ausente: () =>
    terminalView("missing", [
      shellView("powerShell7", notLoaded),
      shellView("windowsPowerShell", notLoaded),
      shellView("gitBash", { loadsIntegration: false }),
    ]),
  ok: () =>
    terminalView("current", [shellView("powerShell7"), shellView("windowsPowerShell"), shellView("gitBash")]),
  bloqueado: () =>
    terminalView("current", [
      shellView("powerShell7", { chainsUserFunction: true }),
      shellView("windowsPowerShell", { policy: "Restricted", policyBlocks: true }),
      shellView("gitBash"),
    ]),
  parcial: () =>
    terminalView("current", [
      shellView("powerShell7"),
      shellView("windowsPowerShell", notLoaded),
      shellView("gitBash", { bashLogin: "ignores" }),
    ]),
  velha: () =>
    terminalView("stale", [shellView("powerShell7"), shellView("windowsPowerShell"), shellView("gitBash")]),
  semrouter: () =>
    terminalView(
      "missing",
      [shellView("powerShell7", notLoaded), shellView("windowsPowerShell", notLoaded)],
      false,
    ),
};

const scriptsParam = param("scripts") as ScriptsState | null;
const terminalScenario =
  param("terminal") ?? { missing: "ausente", stale: "velha", current: "ok" }[scriptsParam ?? "current"];
let terminal: TerminalView = (terminals[terminalScenario] ?? terminals.ok!)();

/** "Ativar": scripts no lugar e a linha em todo perfil. A política do 5.1
 *  continua a que era — no Windows 11 cliente, `Restricted` de fábrica —, e o
 *  `.bash_profile` que ignora o `.bashrc` também: instalar não os resolve. */
function installTerminal(): boolean {
  if (!terminal.routerFound) return false;
  const factoryPolicy: Record<ShellName, string | null> = {
    powerShell7: "RemoteSigned",
    windowsPowerShell: "Restricted",
    gitBash: null,
  };
  const shells = terminal.shells.map((s) => {
    // Sem a linha no perfil a política nem era consultada; agora é.
    const policy = s.policy ?? factoryPolicy[s.shell];
    return { ...s, loadsIntegration: true, policy, policyBlocks: policy === "Restricted" };
  });
  terminal = terminalView("current", shells, true);
  return param("install") !== "falha";
}

// MARK: - Ajustes

const settings: SettingsView = {
  autostart: false,
  autostartFailure: null,
  showInTaskbar: param("taskbar") === "1",
  version: "0.1.0-mock",
};

/** A mesma lista do `allowed_url` do Rust — endereço fora dela é defeito do front. */
function allowedUrl(url: string): boolean {
  const exact = ["ms-settings:developers", "ms-settings:taskbar", "https://claude.ai/logout"];
  return exact.includes(url) || ["https://claude.com/", "https://platform.claude.com/"].some((p) => url.startsWith(p));
}

const scenario = param("state") ?? "uso";
const state: Snapshot = {
  groups: (scenarios[scenario] ?? scenarios.uso!)(),
  measuringGroup: null,
  lastError: scenario === "erro" ? { code: "probeFailures", count: 2 } : null,
  scripts: terminal.routerFound ? terminal.scripts : "missing",
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
  // O quadro de verdade abre um PowerShell por edição: leva um instante.
  terminal_report: async () => {
    await new Promise((resolve) => setTimeout(resolve, 700));
    return structuredClone(terminal);
  },
  install_integration: async (): Promise<InstallResult> => {
    await new Promise((resolve) => setTimeout(resolve, 900));
    const ok = installTerminal();
    state.scripts = terminal.routerFound ? "current" : "missing";
    state.lastError = !terminal.routerFound
      ? { code: "routerPathUnknown" }
      : ok
        ? null
        : { code: "integrationFailed", detail: `${PROFILES.windowsPowerShell}: Acesso negado. (os error 5)` };
    return { ok, snapshot: changed(), report: structuredClone(terminal) };
  },
  allow_profiles_for: async (args) => {
    await new Promise((resolve) => setTimeout(resolve, 900));
    if (param("diretiva") !== "1") {
      const shells = terminal.shells.map((s) =>
        s.shell === args.shell ? { ...s, policy: "RemoteSigned", policyBlocks: false } : s,
      );
      terminal = terminalView(terminal.scripts, shells, terminal.routerFound);
    }
    return structuredClone(terminal);
  },
  get_settings: () => ({ ...settings }),
  set_autostart: (args) => {
    if (args.on && param("autostart") === "falha") {
      settings.autostart = false;
      settings.autostartFailure = "Acesso negado. (os error 5)";
    } else {
      settings.autostart = Boolean(args.on);
      settings.autostartFailure = null;
    }
    return { ...settings };
  },
  set_show_in_taskbar: (args) => {
    settings.showInTaskbar = Boolean(args.on);
    return { ...settings };
  },
  open_url: (args) => {
    const url = String(args.url);
    if (!allowedUrl(url)) throw new Error(`mock: endereço fora da lista: ${url}`);
    console.info("mock: abrir", url);
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
