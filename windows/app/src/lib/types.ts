// Os tipos que o backend manda — espelho dos `#[derive(Serialize)]` do Rust
// (camelCase). Mudou lá, muda aqui: o `svelte-check` pega o resto do front.

export type Locale = "en" | "pt-BR";

export type HomeTab = "groups" | "settings";

/** Qual superfície uma janela mostra. */
export type View = "home" | "flyout";

export interface AppInfo {
  version: string;
  locale: Locale;
  /** A aba com que a janela abre (a bandeja pode ter pedido Ajustes). */
  initialTab: HomeTab;
}

/** Uma janela do `rate_limits`: fração 0–1, o % pronto (do núcleo) e o reset. */
export interface Reading {
  fraction: number;
  text: string;
  resetsAt: string | null;
}

/** A janela POR MODELO — só a sonda a vê, com carimbo próprio. */
export interface ModelReading {
  name: string;
  reading: Reading;
  sampledAt: string | null;
}

export type Bound = "fiveHour" | "sevenDay" | "model";
export type Origin = "sensor" | "probe";

export interface UsageView {
  /** O número que a rotação compara com o limiar (o maior das janelas). */
  fraction: number;
  text: string;
  /** Qual janela manda nesse número. */
  bound: Bound;
  fiveHour: Reading | null;
  sevenDay: Reading | null;
  model: ModelReading | null;
  origin: Origin;
  sampledAt: string;
}

export interface AccountView {
  id: string;
  label: string;
  email: string;
  organization: string | null;
  /** `null` = sem amostra: "pronta". */
  usage: UsageView | null;
}

export interface SessionsView {
  count: number;
  engaged: number;
}

export interface GroupView {
  id: string;
  name: string;
  isDefault: boolean;
  autoRotate: boolean;
  thresholdPercent: number;
  command: string;
  activeAccountId: string | null;
  exclusiveCount: number;
  sessions: SessionsView;
  accounts: AccountView[];
}

export type ErrorView =
  | { code: "saveFailed"; detail: string }
  | { code: "activateNoCredential" }
  | { code: "activateBusyElsewhere"; group: string }
  | { code: "activateWriteFailed"; detail: string }
  | { code: "routerPathUnknown" }
  | { code: "integrationFailed"; detail: string }
  | { code: "probeUnavailable" }
  | { code: "probeFailures"; count: number };

/** Os scripts da integração de terminal: nunca instalados, citando ESTE
 *  router.exe, ou citando outro (app movido). */
export type ScriptsState = "missing" | "current" | "stale";

export type ShellName = "powerShell7" | "windowsPowerShell" | "gitBash";

/** Quem o Git Bash lê ao abrir: nenhum perfil de login, um que carrega o
 *  `.bashrc`, ou um que o ignora (a integração nunca roda). */
export type BashLoginView = "missing" | "loads" | "ignores";

/** Um shell presente na máquina e o que se sabe dele. */
export interface ShellView {
  shell: ShellName;
  /** O arquivo que a integração edita. */
  profile: string;
  loadsIntegration: boolean;
  /** A política de execução efetiva (só PowerShell, e só com a linha no perfil). */
  policy: string | null;
  policyBlocks: boolean;
  /** O perfil já tinha uma função `claude`, que a integração encadeia. */
  chainsUserFunction: boolean;
  bashLogin: BashLoginView | null;
  /** `.bash_profile`, `.bash_login` ou `.profile` (o que existir). */
  bashLoginFile: string | null;
}

/** O quadro da integração de terminal, shell por shell (lento: pedido à parte). */
export interface TerminalView {
  routerFound: boolean;
  scripts: ScriptsState;
  shells: ShellView[];
  developerMode: boolean;
  fullyInstalled: boolean;
  blockedByPolicy: boolean;
  /** "Ativar" resolve algo (a política e o `.bash_profile` têm correção própria). */
  needsInstall: boolean;
}

export interface InstallResult {
  /** Tudo gravado — "Instalada ✓" só com isto. */
  ok: boolean;
  snapshot: Snapshot;
  report: TerminalView;
}

/** O aviso do cartão de grupo sobre a integração. */
export type IntegrationState = "ok" | "install" | "attention";

/** Por que o login não terminou. */
export type LoginFailure =
  | { code: "noClaude" }
  | { code: "pty"; detail: string }
  | { code: "ended" }
  /** O `claude` recusou (`Login failed: …`): o motivo, como ele disse. */
  | { code: "refused"; detail: string };

/** A fase do login (≙ os estados do LoginSheet do macOS, mais o tempo esgotado). */
export type LoginPhase =
  | { phase: "starting" }
  | { phase: "waiting"; url: string }
  | { phase: "confirming" }
  | { phase: "added"; label: string }
  | { phase: "renewed"; label: string }
  | { phase: "duplicate"; email: string }
  | { phase: "wrongAccount"; expected: string; got: string }
  | { phase: "failed"; reason: LoginFailure }
  | { phase: "timeout" };

export type LoginView = LoginPhase & {
  /** Cresce a cada mudança: a tela fica com a maior (evento × resposta). */
  revision: number;
  relogin: boolean;
  invalidCode: boolean;
};

/** A aba Ajustes. "Abrir no login" vem SEMPRE do sistema. */
export interface SettingsView {
  autostart: boolean;
  autostartFailure: string | null;
  showInTaskbar: boolean;
  version: string;
}

export interface Snapshot {
  groups: GroupView[];
  measuringGroup: string | null;
  lastError: ErrorView | null;
  /** O estado barato da integração (o quadro por shell é pedido à parte). */
  scripts: ScriptsState;
}
