<script lang="ts">
  // O login oficial dentro do app (≙ LoginSheet do macOS). Quem autentica é o
  // `claude auth login` num ConPTY, no backend; esta tela só acompanha: o
  // link (o navegador já abriu nele), o código para o raro caso em que o
  // navegador não devolve sozinho, e o desfecho — que o backend confere NO
  // DISCO. Sem o spinner eterno do macOS: um "Login successful" cuja conta não
  // aparece vira "tempo esgotado", com "conferir de novo".
  //
  // Fechar (ou Esc) cancela o que estiver rodando; a limpeza do disco é do
  // backend (a casa reservada que não virou conta; o login estranho de um
  // relogin que voltou com outra conta).
  import * as api from "../lib/api";
  import Icon from "../lib/Icon.svelte";
  import { t } from "../lib/i18n";
  import Modal from "../lib/Modal.svelte";
  import type { LoginFailure, LoginView } from "../lib/types";

  let { view, onClosed }: { view: LoginView; onClosed: () => void } = $props();

  const LOGOUT_URL = "https://claude.ai/logout";

  let busy = $state(false);
  let copied = $state(false);
  let askCode = $state(false);
  let code = $state("");

  async function run(action: () => Promise<unknown>) {
    if (busy) return;
    busy = true;
    try {
      await action();
    } finally {
      busy = false;
    }
  }

  async function close() {
    await run(async () => {
      await api.loginClose();
      onClosed();
    });
  }

  async function copyLink(url: string) {
    await api.copyText(url);
    copied = true;
    setTimeout(() => (copied = false), 2000);
  }

  async function sendCode() {
    const value = code.trim();
    if (!value) return;
    await api.loginSubmitCode(value);
    code = "";
  }

  function failure(reason: LoginFailure): string {
    switch (reason.code) {
      case "noClaude":
        return t("groups.login.error.noClaude");
      case "pty":
        return t("groups.login.error.pty");
      case "ended":
        return t("groups.login.error.ended");
      case "refused":
        return t("groups.login.error.refused");
    }
  }

  function detail(reason: LoginFailure): string | null {
    return reason.code === "pty" || reason.code === "refused" ? reason.detail : null;
  }

  const waitingTitle = $derived(view.relogin ? t("groups.relogin.waiting.title") : t("groups.login.waiting.title"));
</script>

<Modal onClose={() => void close()}>
  <div class="login">
    {#if view.phase === "starting" || view.phase === "waiting" || view.phase === "confirming"}
      <span class="spinner large" aria-hidden="true"></span>
      <h2>{waitingTitle}</h2>

      {#if view.phase === "starting"}
        <p class="caption">{t("groups.login.inapp.starting")}</p>
      {:else if view.phase === "confirming"}
        <p class="caption">{t("groups.login.confirming")}</p>
      {:else}
        {@const url = view.phase === "waiting" ? view.url : ""}
        <p class="caption">{t("groups.login.inapp.detail")}</p>
        <div class="link">
          <code title={url}>{url}</code>
          <button
            class="icon"
            title={t("groups.login.link.copy.help")}
            aria-label={t("groups.login.link.copy.help")}
            onclick={() => void copyLink(url)}
          >
            <Icon name={copied ? "check" : "copy"} size={13} />
          </button>
        </div>
        <button class="secondary small" onclick={() => void api.openUrl(url)}>
          {t("groups.login.openBrowser")}
        </button>

        <div class="code">
          <button class="disclosure" aria-expanded={askCode} onclick={() => (askCode = !askCode)}>
            <span class="chevron" class:open={askCode} aria-hidden="true">›</span>{t("groups.login.code.disclosure")}
          </button>
          {#if askCode}
            <form
              class="code-form"
              onsubmit={(event) => {
                event.preventDefault();
                void sendCode();
              }}
            >
              <input class="field" type="text" placeholder={t("groups.login.code.placeholder")} bind:value={code} />
              <button type="submit" class="secondary small" disabled={code.trim().length === 0}>
                {t("groups.login.code.submit")}
              </button>
            </form>
            {#if view.invalidCode}
              <p class="warn">{t("groups.login.code.invalid")}</p>
            {/if}
          {/if}
        </div>
      {/if}

      <div class="actions">
        <button class="secondary" disabled={busy} onclick={() => void close()}>{t("groups.cancel")}</button>
      </div>
    {:else if view.phase === "added" || view.phase === "renewed"}
      <span class="badge calm"><Icon name="check" size={20} /></span>
      <h2>{view.phase === "renewed" ? t("groups.relogin.done") : t("groups.login.done")}</h2>
      <p class="label">{view.label}</p>
      <div class="actions">
        <button class="primary" disabled={busy} onclick={() => void close()}>{t("groups.login.close")}</button>
      </div>
    {:else if view.phase === "duplicate" || view.phase === "wrongAccount"}
      <span class="badge warning"><Icon name="warning" size={20} /></span>
      {#if view.phase === "duplicate"}
        <h2>{t("groups.login.duplicate.title")}</h2>
        <p class="caption">{t("groups.login.duplicate.detail.format", view.email)}</p>
      {:else}
        <h2>{t("groups.relogin.wrong.title")}</h2>
        <p class="caption">{t("groups.relogin.wrong.detail.format", view.got, view.expected)}</p>
      {/if}
      <button class="secondary small" onclick={() => void api.openUrl(LOGOUT_URL)}>
        {t("groups.login.duplicate.logout")}
      </button>
      <div class="actions">
        <button class="secondary" disabled={busy} onclick={() => void close()}>{t("groups.login.close")}</button>
        <button class="primary" disabled={busy} onclick={() => void run(() => api.loginRetry())}>
          {t("groups.login.duplicate.retry")}
        </button>
      </div>
    {:else if view.phase === "timeout"}
      <span class="badge warning"><Icon name="clockAlert" size={20} /></span>
      <h2>{t("groups.login.timeout.title")}</h2>
      <p class="caption">{t("groups.login.timeout.detail")}</p>
      <div class="actions">
        <button class="secondary" disabled={busy} onclick={() => void close()}>{t("groups.login.close")}</button>
        <button class="primary" disabled={busy} onclick={() => void run(() => api.loginRecheck())}>
          {t("groups.login.timeout.recheck")}
        </button>
      </div>
    {:else if view.phase === "failed"}
      <span class="badge critical"><Icon name="warning" size={20} /></span>
      <h2>{t("groups.login.failed")}</h2>
      <p class="caption">{failure(view.reason)}</p>
      {#if detail(view.reason)}
        <p class="detail">{detail(view.reason)}</p>
      {/if}
      <div class="actions">
        <button class="secondary" disabled={busy} onclick={() => void close()}>{t("groups.login.close")}</button>
        {#if view.reason.code !== "noClaude"}
          <button class="primary" disabled={busy} onclick={() => void run(() => api.loginRetry())}>
            {t("groups.login.duplicate.retry")}
          </button>
        {/if}
      </div>
    {/if}
  </div>
</Modal>

<style>
  .login {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    text-align: center;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .caption {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .label {
    font-size: var(--font-body);
    color: var(--text-secondary);
  }
  .detail {
    max-width: 100%;
    padding: 6px 8px;
    border-radius: 6px;
    background: var(--bg);
    font-family: var(--font-mono);
    font-size: var(--font-small);
    color: var(--text-secondary);
    overflow-wrap: anywhere;
    user-select: text;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 4px 4px 8px;
    border-radius: 6px;
    background: var(--bg);
  }
  .link code {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
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
  .code {
    display: flex;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
    width: 100%;
  }
  .disclosure {
    display: inline-flex;
    align-items: center;
    align-self: center;
    gap: 4px;
    padding: 2px 4px;
    border: none;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .chevron {
    display: inline-block;
    transition: transform 120ms;
  }
  .chevron.open {
    transform: rotate(90deg);
  }
  .code-form {
    display: flex;
    gap: 6px;
  }
  .code-form .field {
    flex: 1;
    min-width: 0;
  }
  .warn {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--warning-text);
  }
  .actions {
    display: flex;
    justify-content: center;
    gap: 8px;
    margin-top: 6px;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 40px;
    height: 40px;
    border-radius: 50%;
  }
  .badge.calm {
    background: color-mix(in srgb, var(--calm) 18%, transparent);
    color: var(--calm);
  }
  .badge.warning {
    background: color-mix(in srgb, var(--warning) 18%, transparent);
    color: var(--warning-text);
  }
  .badge.critical {
    background: color-mix(in srgb, var(--critical) 16%, transparent);
    color: var(--critical);
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--track);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  .spinner.large {
    width: 28px;
    height: 28px;
    border-width: 3px;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
