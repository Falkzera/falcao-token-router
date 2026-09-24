<script lang="ts">
  // A falha da última ação, no idioma do usuário. O núcleo devolve FATOS com
  // código; o texto é daqui (no macOS o `lastError` vinha em pt-BR fixo do
  // core). Chaves literais — uma por código — para o verificador enxergá-las.
  import { t } from "../lib/i18n";
  import type { ErrorView } from "../lib/types";

  let { error, onDismiss }: { error: ErrorView; onDismiss: () => void } = $props();

  function text(e: ErrorView): string {
    switch (e.code) {
      case "saveFailed":
        return t("groups.error.saveFailed.format", e.detail);
      case "activateNoCredential":
        return t("groups.error.activate.noCredential");
      case "activateBusyElsewhere":
        return t("groups.error.activate.busyElsewhere.format", e.group);
      case "activateWriteFailed":
        return t("groups.error.activate.writeFailed.format", e.detail);
      case "routerPathUnknown":
        return t("groups.error.routerPathUnknown");
      case "integrationFailed":
        return t("groups.error.integrationFailed.format", e.detail);
      case "probeUnavailable":
        return t("groups.error.probeUnavailable");
      case "probeFailures":
        return t("groups.error.probeFailures.format", e.count);
    }
  }
</script>

<div class="banner" role="alert">
  <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.4" aria-hidden="true">
    <path d="M8 2.2l6.3 11.1H1.7z" stroke-linejoin="round" />
    <path d="M8 6.5v3.2M8 11.6v.2" stroke-linecap="round" />
  </svg>
  <span class="text">{text(error)}</span>
  <button class="dismiss" onclick={onDismiss}>{t("groups.error.dismiss")}</button>
</div>

<style>
  .banner {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    padding: 8px var(--page-inline);
    background: color-mix(in srgb, var(--critical) 12%, transparent);
    color: var(--critical);
    font-size: var(--font-caption);
    line-height: 1.4;
  }
  svg {
    flex: none;
    margin-top: 1px;
  }
  .text {
    flex: 1;
    /* Um caminho do Windows é uma "palavra" só: sem isto ele empurrava o
       "Dispensar" para fora da janela. */
    min-width: 0;
    overflow-wrap: anywhere;
    color: var(--text);
  }
  .dismiss {
    flex: none;
    border: none;
    background: none;
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--font-caption);
    text-decoration: underline;
    cursor: pointer;
  }
</style>
