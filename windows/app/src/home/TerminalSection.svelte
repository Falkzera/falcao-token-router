<script lang="ts">
  // A integração de terminal (≙ TerminalIntegrationRow do macOS), no fim da aba
  // Grupos — é para ela que o "instale a integração ↓" dos cartões aponta. No
  // macOS um clique editava o `~/.zshrc` e pronto; no Windows os modos de falha
  // são vários e SILENCIOSOS (a política de execução, um perfil sem a linha, um
  // `.bash_profile` que ignora o `.bashrc`) e todos terminam em `claude <grupo>`
  // abrindo o `claude` puro, na conta errada. Por isso o quadro é por shell, e
  // cada problema aparece com a correção dele:
  // - "Ativar"/"Reinstalar" só promete o que instalar resolve (`needsInstall`);
  // - a política tem o "Permitir", com confirmação (muda um ajuste do Windows);
  // - o `.bash_profile` é do usuário: a tela mostra a linha, com copiar.
  // "Instalada ✓" só aparece quando TUDO foi gravado — o macOS mostrava o ✓
  // mesmo com a instalação falhando.
  import * as api from "../lib/api";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import Rich from "../lib/Rich.svelte";
  import type { ShellName, ShellView, Snapshot, TerminalView } from "../lib/types";
  import ConfirmDialog from "./ConfirmDialog.svelte";

  let {
    report,
    checking,
    onReport,
    onSnapshot,
  }: {
    /** `null` enquanto o primeiro quadro não chega (ou se ele falhou). */
    report: TerminalView | null;
    checking: boolean;
    onReport: (report: TerminalView) => void;
    onSnapshot: (snapshot: Snapshot) => void;
  } = $props();

  // Nomes de produto: não se traduzem.
  const SHELL_NAMES: Record<ShellName, string> = {
    powerShell7: "PowerShell 7",
    windowsPowerShell: "Windows PowerShell 5.1",
    gitBash: "Git Bash",
  };
  /** O que o `.bash_profile` gerado pelo Git for Windows tem, e o que falta
   *  num que ignora o `.bashrc`. */
  const BASHRC_LINE = "test -f ~/.bashrc && . ~/.bashrc";

  type Status = "ok" | "missing" | "blocked" | "bashIgnores";
  function status(shell: ShellView): Status {
    if (!shell.loadsIntegration) return "missing";
    if (shell.policyBlocks) return "blocked";
    if (shell.bashLogin === "ignores") return "bashIgnores";
    return "ok";
  }

  // MARK: Ativar / Reinstalar
  let installing = $state(false);
  /** O ✓ por 2 s depois de uma instalação completa: o estado da seção pode ser
   *  "pronta" antes e depois, e o clique precisa de resposta visível. */
  let installed = $state(false);
  async function install() {
    installing = true;
    try {
      const result = await api.installIntegration();
      onSnapshot(result.snapshot);
      onReport(result.report);
      if (result.ok) {
        installed = true;
        setTimeout(() => (installed = false), 2000);
      }
    } finally {
      installing = false;
    }
  }

  // MARK: Permitir (política de execução)
  let confirming = $state<ShellView | null>(null);
  let allowing = $state<ShellName | null>(null);
  /** Edições em que o "Permitir" não venceu: uma diretiva de grupo manda. */
  let stillBlocked = $state<ShellName[]>([]);
  async function allow(shell: ShellName) {
    confirming = null;
    allowing = shell;
    try {
      const next = await api.allowProfilesFor(shell);
      onReport(next);
      const blocked = next.shells.find((s) => s.shell === shell)?.policyBlocks ?? false;
      stillBlocked = blocked
        ? [...stillBlocked.filter((s) => s !== shell), shell]
        : stillBlocked.filter((s) => s !== shell);
    } finally {
      allowing = null;
    }
  }

  // MARK: copiar a linha do .bashrc
  let copied = $state(false);
  async function copyLine() {
    await api.copyText(BASHRC_LINE);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  const canInstall = $derived(report === null || report.routerFound);
</script>

<section class="card" id="terminal-integration" aria-labelledby="terminal-title">
  <header>
    <Icon name="terminal" size={14} />
    <h3 id="terminal-title">{t("groups.terminal.title")}</h3>
    {#if checking && report}
      <span class="spinner" aria-hidden="true"></span>
    {/if}
    <span class="spacer"></span>
    {#if installed}
      <span class="done">{t("groups.terminal.reinstalled")}</span>
    {:else if installing}
      <span class="busy"><span class="spinner" aria-hidden="true"></span>{t("groups.terminal.installing")}</span>
    {:else if report === null && checking}
      <!-- O botão espera o quadro: o rótulo depende do que ele disser. -->
    {:else if report === null || report.needsInstall}
      <button class="primary small" disabled={!canInstall} onclick={install}>{t("groups.terminal.activate")}</button>
    {:else}
      <button class="secondary small" onclick={install}>{t("groups.terminal.reinstall")}</button>
    {/if}
  </header>

  {#if report === null}
    {#if checking}
      <p class="status muted"><span class="spinner" aria-hidden="true"></span>{t("groups.terminal.checking")}</p>
    {:else}
      <p class="status muted"><Rich text={t("groups.terminal.pitch")} /></p>
    {/if}
  {:else if !report.routerFound}
    <p class="status warn"><Icon name="warning" size={13} /><span>{t("groups.error.routerPathUnknown")}</span></p>
  {:else if report.scripts === "missing"}
    <p class="status muted"><Rich text={t("groups.terminal.pitch")} /></p>
  {:else}
    {#if report.fullyInstalled}
      <p class="status ok"><Icon name="check" size={13} /><span><Rich text={t("groups.terminal.ready")} /></span></p>
    {:else}
      <p class="status warn"><Icon name="warning" size={13} /><span><Rich text={t("groups.terminal.attention")} /></span></p>
    {/if}
    {#if report.scripts === "stale"}
      <p class="status warn"><Icon name="warning" size={13} /><span>{t("groups.terminal.stale")}</span></p>
    {/if}

    <ul class="shells">
      {#each report.shells as shell (shell.shell)}
        {@const state = status(shell)}
        <li>
          <span class="name" title={shell.profile}>{SHELL_NAMES[shell.shell]}</span>
          <div class="facts">
            {#if state === "ok"}
              <p class="ok"><Icon name="check" size={12} /><span>{t("groups.terminal.shell.ok")}</span></p>
            {:else if state === "missing"}
              <p class="warn"><Icon name="warning" size={12} /><span>{t("groups.terminal.shell.missing")}</span></p>
            {:else if state === "blocked"}
              <p class="warn">
                <Icon name="warning" size={12} />
                <span><Rich text={t("groups.terminal.shell.blocked.format", shell.policy ?? "")} /></span>
              </p>
              {#if stillBlocked.includes(shell.shell)}
                <!-- O "Permitir" já não venceu: repetir não resolve. -->
                <p class="muted">{t("groups.terminal.allow.stillBlocked")}</p>
              {:else}
                <div class="fix">
                  {#if allowing === shell.shell}
                    <span class="busy"><span class="spinner" aria-hidden="true"></span>{t("groups.terminal.allowing")}</span>
                  {:else}
                    <button class="secondary small" disabled={allowing !== null} onclick={() => (confirming = shell)}>
                      {t("groups.terminal.allow")}
                    </button>
                  {/if}
                </div>
              {/if}
            {:else}
              <p class="warn">
                <Icon name="warning" size={12} />
                <span><Rich text={t("groups.terminal.bash.ignores.format", shell.bashLoginFile ?? "")} /></span>
              </p>
              <div class="snippet">
                <code>{BASHRC_LINE}</code>
                <button
                  class="icon"
                  title={t("groups.terminal.bash.copy.help")}
                  aria-label={t("groups.terminal.bash.copy.help")}
                  onclick={copyLine}
                >
                  <Icon name={copied ? "check" : "copy"} size={12} />
                </button>
              </div>
            {/if}
            {#if shell.chainsUserFunction && state !== "missing"}
              <p class="muted"><Icon name="info" size={12} /><span><Rich text={t("groups.terminal.shell.chains")} /></span></p>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}

  {#if report && !report.developerMode}
    <div class="hint">
      <Icon name="info" size={12} />
      <div>
        <p><Rich text={t("groups.terminal.devMode")} /></p>
        <button class="link" onclick={() => void api.openUrl("ms-settings:developers")}>
          {t("groups.terminal.devMode.open")}
        </button>
      </div>
    </div>
  {/if}
</section>

{#if confirming}
  {@const shell = confirming}
  <ConfirmDialog
    title={t("groups.terminal.allow.confirm.title.format", SHELL_NAMES[shell.shell])}
    message={t("groups.terminal.allow.confirm.message")}
    confirm={t("groups.terminal.allow.confirm.button")}
    destructive={false}
    onConfirm={() => void allow(shell.shell)}
    onCancel={() => (confirming = null)}
  />
{/if}

<style>
  .card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    scroll-margin-top: 16px;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    min-height: 28px;
    color: var(--text-secondary);
  }
  h3 {
    margin: 0;
    font-size: 14px;
    font-weight: 600;
    color: var(--text);
  }
  .spacer {
    flex: 1;
  }
  .status {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: var(--font-caption);
    line-height: 1.45;
  }
  .status :global(svg),
  .facts p :global(svg),
  .hint > :global(svg) {
    margin-top: 2px;
  }
  .ok {
    color: var(--calm);
  }
  .ok span {
    color: var(--text);
  }
  .warn {
    color: var(--warning-text);
  }
  .warn span {
    color: var(--text);
  }
  .muted {
    color: var(--text-secondary);
  }
  .shells {
    display: flex;
    flex-direction: column;
    margin: 0;
    padding: 0;
    list-style: none;
    border-top: 1px solid var(--border);
  }
  .shells li {
    display: grid;
    grid-template-columns: 150px 1fr;
    gap: 10px;
    padding: 8px 0;
    border-bottom: 1px solid var(--border);
  }
  .name {
    font-size: var(--font-caption);
    font-weight: 600;
    line-height: 1.45;
  }
  .facts {
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
  }
  .facts p {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: var(--font-caption);
    line-height: 1.45;
  }
  .fix {
    display: flex;
  }
  .snippet {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 6px;
    background: var(--bg);
  }
  code {
    flex: 1;
    font-family: var(--font-mono);
    font-size: var(--font-small);
    user-select: text;
  }
  .icon {
    display: inline-flex;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .icon:hover {
    background: var(--row-hover);
    color: var(--text);
  }
  .hint {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    color: var(--text-tertiary);
    font-size: var(--font-small);
    line-height: 1.45;
  }
  .hint p {
    color: var(--text-secondary);
  }
  .link {
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }
  .done,
  .busy {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-caption);
  }
  .done {
    color: var(--calm);
  }
  .busy {
    color: var(--text-secondary);
  }
  .spinner {
    flex: none;
    width: 12px;
    height: 12px;
    border: 2px solid var(--track);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  .status .spinner {
    margin-top: 2px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
