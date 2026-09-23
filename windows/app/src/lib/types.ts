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

export interface Snapshot {
  groups: GroupView[];
  measuringGroup: string | null;
  lastError: ErrorView | null;
  /** O estado barato da integração (o quadro por shell é pedido à parte). */
  scripts: ScriptsState;
}
