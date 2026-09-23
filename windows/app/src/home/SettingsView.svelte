<script lang="ts">
  // A aba Ajustes (≙ a seção "Sistema" do SettingsView do macOS; o medidor não
  // entra nesta entrega). Três coisas, todas sobre o app ter por onde ser
  // aberto e estar lá quando se precisa dele:
  // - "Abrir no login", SEMPRE relido do sistema: um interruptor ligado sobre
  //   um registro que falhou é pior que nenhum — e o motivo aparece.
  // - "Mostrar na barra de tarefas" (≙ "Mostrar no Dock", pela mesma razão).
  // - a dica do ícone escondido no `^` do Windows 11, com o atalho para as
  //   Configurações onde se escolhe o que fica à vista.
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import { t } from "../lib/i18n";
  import Switch from "../lib/Switch.svelte";
  import type { SettingsView } from "../lib/types";

  let settings = $state<SettingsView | null>(null);

  onMount(() => {
    void api.getSettings().then((s) => (settings = s));
  });
</script>

{#if settings}
  <div class="settings">
    <section>
      <h3>{t("settings.section.system")}</h3>
      <div class="group">
        <div class="row">
          <Switch
            checked={settings.autostart}
            label={t("settings.system.loginItem")}
            onChange={async (on) => (settings = await api.setAutostart(on))}
          />
          {#if settings.autostartFailure}
            <p class="failure">{t("settings.system.loginItem.failure.format", settings.autostartFailure)}</p>
          {/if}
        </div>
        <div class="row">
          <Switch
            checked={settings.showInTaskbar}
            label={t("settings.system.taskbar")}
            onChange={async (on) => (settings = await api.setShowInTaskbar(on))}
          />
          <p class="detail">{t("settings.system.taskbar.detail")}</p>
        </div>
      </div>
    </section>

    <section>
      <h3>{t("settings.section.tray")}</h3>
      <div class="group">
        <div class="row">
          <p class="detail">{t("settings.tray.hidden")}</p>
          <div>
            <button class="secondary small" onclick={() => void api.openUrl("ms-settings:taskbar")}>
              {t("settings.tray.open")}
            </button>
          </div>
        </div>
      </div>
    </section>

    <p class="version">{t("settings.version.format", settings.version)}</p>
  </div>
{/if}

<style>
  .settings {
    display: flex;
    flex-direction: column;
    gap: 20px;
    height: 100%;
    padding: 16px;
    overflow: auto;
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  h3 {
    margin: 0;
    font-size: var(--font-caption);
    font-weight: 600;
    color: var(--text-secondary);
  }
  .group {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .row {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 12px 14px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .detail {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .failure {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--critical);
  }
  .version {
    margin-top: auto;
    font-size: var(--font-caption);
    color: var(--text-tertiary);
  }
</style>
