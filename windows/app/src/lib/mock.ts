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
// A status line (aba Ajustes):
//   &statusline=itens|vazia|comando  &runner=powershell|nenhum
//   &teste=ok|colorido|vazio|falha|prazo|naosubiu|semshell (o que o "Testar" responde)
// O login (Adicionar conta / Relogar):
//   &login=ok|codigo|duplicada|errada|recusado|encerrado|timeout|semclaude
// Dados só de exemplo (@exemplo.com, Acme, C:\Users\exemplo).

import type {
  AppInfo,
  GroupView,
  HomeTab,
  InstallResult,
  Locale,
  LoginPhase,
  LoginView,
  ScriptsState,
  SettingsView,
  ShellName,
  ShellView,
  Snapshot,
  Span,
  StatusLineChoice,
  StatusLineItem,
  StatusLineTest,
  StatusLineView,
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

// MARK: - Status line

const ITEMS: StatusLineItem[] = [
  "group",
  "model",
  "effort",
  "place",
  "context",
  "fiveHour",
  "sevenDay",
  "resets",
  "cost",
  "email",
];

function initialChoice(): StatusLineChoice {
  switch (param("statusline")) {
    case "itens":
      return { mode: "app", hidden: ["context", "cost"], command: "" };
    case "vazia":
      return { mode: "app", hidden: [...ITEMS], command: "" };
    case "comando":
      return { mode: "command", hidden: [], command: "node C:/Users/exemplo/linha.js" };
    default:
      return { mode: "app", hidden: [], command: "" };
  }
}

let statusChoice = initialChoice();

// As cores que o `render` do núcleo usa (paleta Campbell + truecolor).
const CYAN = "#3a96dd";
const BLUE = "#3b78ff";
const GREEN = "#13a10e";
const GRAY = "#999999";

/** Uma imitação do `render` do núcleo para a sessão de exemplo — só para o
 *  navegador; no app, a prévia vem do Rust. */
function mockPreview(choice: StatusLineChoice): Span[] {
  const shows = (item: StatusLineItem) => !choice.hidden.includes(item);
  const s = (text: string, color: string | null = null, bold = false): Span => ({ text, color, bold });
  const portuguese = mockLocale() === "pt-BR";
  const first = state.groups[0];
  const email =
    first?.accounts.find((a) => a.id === first.activeAccountId)?.email ??
    (portuguese ? "voce@exemplo.com" : "you@example.com");
  const now = Date.now();
  const five = new Date(now + (2 * 60 + 13) * 60_000);
  const seven = new Date(now + (4 * 24 + 5) * 3_600_000);
  const days = portuguese
    ? ["dom", "seg", "ter", "qua", "qui", "sex", "sáb"]
    : ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
  const hhmm = (d: Date) => `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  const dayTime = (d: Date) => `${days[d.getDay()]} (${d.getDate()}) ${d.getHours()}:${String(d.getMinutes()).padStart(2, "0")}`;

  const segments: Span[][] = [];
  if (shows("group")) {
    const name = first?.name ?? (portuguese ? "Trabalho" : "Work");
    segments.push([s("●", CYAN, true), s(" "), s(name, CYAN)]);
  }
  const effort = s("high", "#b1b9f9", true);
  if (shows("model")) segments.push(shows("effort") ? [s("Opus 5.5", BLUE, true), s(" "), effort] : [s("Opus 5.5", BLUE, true)]);
  else if (shows("effort")) segments.push([effort]);
  if (shows("place")) segments.push([s("~/app", GRAY)]);
  if (shows("context")) segments.push([s("███░░░░░░░", GREEN), s(" "), s("26%", GREEN), s(" "), s("51k/200k", GRAY)]);
  const windows: Span[] = [];
  if (shows("fiveHour")) {
    windows.push(s("5h", GRAY), s(" "), s("█░░░░ 29%", GREEN));
    if (shows("resets")) windows.push(s(" "), s(`↻ ${hhmm(five)}`, GRAY));
  }
  if (shows("sevenDay")) {
    if (windows.length) windows.push(s("  "));
    windows.push(s("7d", GRAY), s(" "), s("██░░░ 33%", GREEN));
    if (shows("resets")) windows.push(s(" "), s(`↻ ${dayTime(seven)}`, GRAY));
  }
  if (windows.length) segments.push(windows);
  if (shows("cost")) segments.push([s("$1.87", GRAY)]);
  if (shows("email")) segments.push([s(email, GRAY)]);
  return segments.flatMap((segment, i) => (i === 0 ? segment : [s(" │ ", GRAY), ...segment]));
}

function statusLineView(): StatusLineView {
  const runner = param("runner");
  return {
    choice: structuredClone(statusChoice),
    preview: mockPreview(statusChoice),
    runner: runner === "nenhum" ? null : runner === "powershell" ? "powerShell" : "gitBash",
    deadlineSeconds: 5,
  };
}

function mockTest(command: string): StatusLineTest {
  const line = (text: string): Span[] => [{ text, color: null, bold: false }];
  switch (param("teste")) {
    case "colorido":
      return {
        outcome: "printed",
        spans: [
          { text: "~/app", color: "#61d6d6", bold: true },
          { text: " main ", color: "#16c60c", bold: false },
          { text: "Opus 5.5", color: null, bold: false },
        ],
        elapsedMs: 180,
      };
    case "vazio":
      return { outcome: "failed", code: 0, detail: "" };
    case "falha":
      return {
        outcome: "failed",
        code: 1,
        detail: "node:internal/modules/cjs/loader:1228\n  throw err;\n  ^\n\nError: Cannot find module 'C:\\Users\\exemplo\\linha.js'",
      };
    case "prazo":
      return { outcome: "timedOut", seconds: 5 };
    case "naosubiu":
      return { outcome: "notStarted", detail: "O sistema não pode encontrar o arquivo especificado. (os error 2)" };
    case "semshell":
      return { outcome: "noShell" };
    default:
      return { outcome: "printed", spans: line(`${command} → linha de exemplo`), elapsedMs: 140 };
  }
}

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

// MARK: - Login

const LOGIN_URL =
  "https://claude.com/cai/oauth/authorize?code=true&client_id=00000000-0000-4000-8000-000000000000" +
  "&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A54545%2Fcallback" +
  "&scope=org%3Acreate_api_key+user%3Aprofile+user%3Ainference&code_challenge=exemplo" +
  "&code_challenge_method=S256&state=exemplo";

let login: LoginView | null = null;
let loginTarget: { relogin: boolean; groupId: string; accountId?: string } | null = null;
let loginRevision = 0;
let loginTimers: ReturnType<typeof setTimeout>[] = [];
const loginListeners = new Set<(view: LoginView | null) => void>();

export function mockLoginListen(handler: (view: LoginView | null) => void): () => void {
  loginListeners.add(handler);
  return () => loginListeners.delete(handler);
}

function publishLogin(): LoginView | null {
  const view = login === null ? null : structuredClone(login);
  setTimeout(() => loginListeners.forEach((l) => l(view)), 0);
  return view;
}

function setPhase(phase: LoginPhase): LoginView | null {
  login = { ...phase, revision: ++loginRevision, relogin: loginTarget?.relogin ?? false, invalidCode: false };
  return publishLogin();
}

function setInvalidCode(invalidCode: boolean): void {
  if (!login) return;
  login = { ...login, invalidCode, revision: ++loginRevision };
  publishLogin();
}

function later(ms: number, action: () => void): void {
  loginTimers.push(setTimeout(action, ms));
}

function findAccount(id: string | undefined) {
  return state.groups.flatMap((g) => g.accounts).find((a) => a.id === id);
}

/** Um login do começo: iniciando → link → (o cenário). */
function runLogin(): LoginView | null {
  loginTimers.forEach(clearTimeout);
  loginTimers = [];
  const scenario = param("login") ?? "ok";
  if (scenario === "semclaude") return setPhase({ phase: "failed", reason: { code: "noClaude" } });
  const first = setPhase({ phase: "starting" });
  later(700, () => {
    setPhase({ phase: "waiting", url: LOGIN_URL });
    // "codigo": o navegador não devolveu sozinho — espera o código colado.
    if (scenario !== "codigo") later(2500, () => finishLogin(scenario));
  });
  return first;
}

function finishLogin(scenario: string): void {
  if (scenario === "recusado") {
    setPhase({ phase: "failed", reason: { code: "refused", detail: "Request failed with status code 403" } });
    return;
  }
  if (scenario === "encerrado") {
    setPhase({ phase: "failed", reason: { code: "ended" } });
    return;
  }
  setPhase({ phase: "confirming" });
  later(900, () => settleLogin(scenario));
}

function settleLogin(scenario: string): void {
  const target = loginTarget;
  if (!target) return;
  if (scenario === "timeout") {
    setPhase({ phase: "timeout" });
  } else if (target.relogin) {
    const account = findAccount(target.accountId);
    setPhase(
      scenario === "errada"
        ? { phase: "wrongAccount", expected: account?.email ?? "", got: "intrusa@exemplo.com" }
        : { phase: "renewed", label: account?.label ?? "" },
    );
  } else if (scenario === "duplicada") {
    setPhase({ phase: "duplicate", email: "equipe-1@exemplo.com" });
  } else {
    const n = nextId++;
    findGroup(target.groupId)?.accounts.push({
      id: `A${n}`,
      label: `conta${n}`,
      email: `conta${n}@exemplo.com`,
      organization: null,
      usage: null,
    });
    changed();
    setPhase({ phase: "added", label: `conta${n}` });
  }
}

const handlers: Record<string, (args: Record<string, unknown>) => unknown> = {
  start_login: (args) => {
    loginTarget = { relogin: false, groupId: String(args.groupId) };
    return runLogin();
  },
  start_relogin: (args) => {
    loginTarget = { relogin: true, groupId: String(args.groupId), accountId: String(args.accountId) };
    return runLogin();
  },
  current_login: () => (login ? structuredClone(login) : null),
  login_submit_code: (args) => {
    if (login?.phase !== "waiting") return false;
    // O `claude` quer `código#state`; outra coisa é "Invalid code".
    const valid = /^[^#\s]+#[^#\s]+$/.test(String(args.code).trim());
    setInvalidCode(false);
    later(400, () => (valid ? finishLogin("ok") : setInvalidCode(true)));
    return true;
  },
  login_retry: () => {
    const retryable =
      login !== null &&
      ["duplicate", "wrongAccount", "timeout", "failed"].includes(login.phase) &&
      !(login.phase === "failed" && login.reason.code === "noClaude");
    return retryable ? runLogin() : login;
  },
  login_recheck: () => {
    if (login?.phase !== "timeout") return login;
    const view = setPhase({ phase: "confirming" });
    later(900, () => settleLogin("ok"));
    return view;
  },
  login_close: () => {
    loginTimers.forEach(clearTimeout);
    loginTimers = [];
    loginTarget = null;
    login = null;
    publishLogin();
    return changed();
  },
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
  get_status_line: () => statusLineView(),
  // Como o Rust: a escolha que volta é a normalizada (itens na ordem da linha).
  set_status_line: (args) => {
    const choice = args.choice as StatusLineChoice;
    statusChoice = {
      mode: choice.mode === "command" ? "command" : "app",
      hidden: ITEMS.filter((item) => choice.hidden.includes(item)),
      command: choice.command,
    };
    console.info("mock: status line gravada", JSON.stringify(statusChoice));
    return statusLineView();
  },
  test_status_line: async (args) => {
    await new Promise((resolve) => setTimeout(resolve, 600));
    return mockTest(String(args.command));
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
