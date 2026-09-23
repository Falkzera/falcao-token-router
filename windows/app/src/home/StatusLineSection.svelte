<script lang="ts">
  // A status line das sessões dos grupos (pedido de 23/09/2026). O padrão é a
  // linha COMPLETA do app; dá para tirar itens dela, ou usar o próprio comando
  // no lugar — o router mede do mesmo jeito e roda o comando depois, com o
  // mesmo JSON. A prévia é desenhada no backend pelo MESMO código do `router
  // statusline`, com uma sessão de exemplo. Grava na hora: a CLI lê a escolha
  // a cada render, e vale na próxima atualização das sessões abertas.
  import { onMount } from "svelte";
  import * as api from "../lib/api";
  import Icon from "../lib/Icon.svelte";
  import { t, type Key } from "../lib/i18n";
  import Switch from "../lib/Switch.svelte";
  import type { Runner, Span, StatusLineChoice, StatusLineItem, StatusLineTest, StatusLineView } from "../lib/types";

  const ITEMS: { item: StatusLineItem; key: Key }[] = [
    { item: "group", key: "settings.statusLine.item.group" },
    { item: "model", key: "settings.statusLine.item.model" },
    { item: "effort", key: "settings.statusLine.item.effort" },
    { item: "place", key: "settings.statusLine.item.place" },
    { item: "context", key: "settings.statusLine.item.context" },
    { item: "fiveHour", key: "settings.statusLine.item.fiveHour" },
    { item: "sevenDay", key: "settings.statusLine.item.sevenDay" },
    { item: "resets", key: "settings.statusLine.item.resets" },
    { item: "cost", key: "settings.statusLine.item.cost" },
    { item: "email", key: "settings.statusLine.item.email" },
  ];

  // Nomes de produto: não se traduzem.
  const RUNNERS: Record<Runner, string> = { gitBash: "Git Bash", powerShell: "PowerShell" };

  let view = $state<StatusLineView | null>(null);
  /** O campo do comando; grava ao sair dele (ou Enter), não a cada tecla. */
  let draft = $state("");
  let failure = $state<string | null>(null);
  let testing = $state(false);
  let test = $state<StatusLineTest | null>(null);

  onMount(() => {
    void api.getStatusLine().then((next) => {
      view = next;
      draft = next.choice.command;
    });
  });

  /** As gravações vão em fila, na ordem dos cliques: sair do campo clicando na
   *  chave grava o comando e, logo depois, o modo — a última tem de vencer. */
  let queue: Promise<void> = Promise.resolve();
  function save(choice: StatusLineChoice) {
    queue = queue.then(async () => {
      try {
        view = await api.setStatusLine(choice);
        failure = null;
      } catch (error) {
        failure = String(error);
      }
    });
  }

  function toggle(current: StatusLineChoice, item: StatusLineItem, shown: boolean) {
    const hidden = shown ? current.hidden.filter((i) => i !== item) : [...current.hidden, item];
    save({ ...current, hidden });
  }

  function commitCommand(current: StatusLineChoice) {
    if (draft !== current.command) save({ ...current, command: draft });
  }

  /** O resultado vale para o comando testado: editar o campo ou trocar de modo o
   *  apaga, e um resultado que chega depois de uma edição não aparece. */
  async function runTest() {
    const command = draft;
    testing = true;
    test = null;
    try {
      const result = await api.testStatusLine(command);
      if (draft === command) test = result;
    } finally {
      testing = false;
    }
  }
</script>

{#snippet terminal(spans: Span[], empty: string)}
  <div class="terminal">
    {#if spans.length > 0}
      {#each spans as span, index (index)}<span class:bold={span.bold} style:color={span.color}>{span.text}</span>{/each}
    {:else}
      <span class="nothing">{empty}</span>
    {/if}
  </div>
{/snippet}

{#if view}
  {@const choice = view.choice}
  <section>
    <h3>{t("settings.section.statusLine")}</h3>
    <div class="group">
      <div class="row">
        <Switch
          checked={choice.mode === "app"}
          label={t("settings.statusLine.useApp")}
          onChange={(on) => {
            test = null;
            save({ ...choice, mode: on ? "app" : "command", command: draft });
          }}
        />
        <p class="detail">{t("settings.statusLine.detail")}</p>
      </div>

      {#if choice.mode === "app"}
        <div class="row">
          {@render terminal(view.preview, t("settings.statusLine.preview.empty"))}
          <p class="caption">{t("settings.statusLine.preview.caption")}</p>
        </div>
        <div class="row">
          <div class="items-head">
            <span class="label">{t("settings.statusLine.items")}</span>
            {#if choice.hidden.length > 0}
              <button class="link" onclick={() => save({ ...choice, hidden: [] })}>
                {t("settings.statusLine.restore")}
              </button>
            {/if}
          </div>
          <div class="items">
            {#each ITEMS as { item, key } (item)}
              <label class="check">
                <input
                  type="checkbox"
                  checked={!choice.hidden.includes(item)}
                  onchange={(event) => toggle(choice, item, event.currentTarget.checked)}
                />
                <span>{t(key)}</span>
              </label>
            {/each}
          </div>
        </div>
      {:else}
        <div class="row">
          <label class="label" for="status-line-command">{t("settings.statusLine.command.label")}</label>
          <div class="command">
            <input
              id="status-line-command"
              class="field"
              spellcheck="false"
              autocomplete="off"
              placeholder={t("settings.statusLine.command.placeholder")}
              bind:value={draft}
              oninput={() => (test = null)}
              onblur={() => commitCommand(choice)}
              onkeydown={(event) => {
                if (event.key === "Enter") commitCommand(choice);
              }}
            />
            <button
              class="secondary"
              disabled={testing || draft.trim() === "" || view.runner === null}
              onclick={() => void runTest()}
            >
              {#if testing}<span class="spinner" aria-hidden="true"></span>{t("settings.statusLine.testing")}{:else}{t(
                  "settings.statusLine.test",
                )}{/if}
            </button>
          </div>
          {#if view.runner}
            <p class="detail">
              {t("settings.statusLine.command.detail.format", RUNNERS[view.runner], view.deadlineSeconds)}
            </p>
          {:else}
            <p class="status warn"><Icon name="warning" size={13} /><span>{t("settings.statusLine.command.noShell")}</span></p>
          {/if}
          {#if draft.trim() === ""}
            <p class="detail">{t("settings.statusLine.command.empty")}</p>
          {/if}

          {#if test}
            {#if test.outcome === "printed"}
              {@render terminal(test.spans, "")}
              <p class="status ok">
                <Icon name="check" size={13} /><span>{t("settings.statusLine.test.printed.format", test.elapsedMs)}</span>
              </p>
            {:else}
              <p class="status warn">
                <Icon name="warning" size={13} />
                <span>
                  {#if test.outcome === "failed" && test.code !== 0 && test.code !== null}
                    {t("settings.statusLine.test.failed.format", test.code)}
                  {:else if test.outcome === "failed"}
                    {t("settings.statusLine.test.silent")}
                  {:else if test.outcome === "timedOut"}
                    {t("settings.statusLine.test.timedOut.format", test.seconds)}
                  {:else if test.outcome === "notStarted"}
                    {t("settings.statusLine.test.notStarted.format", test.detail)}
                  {:else}
                    {t("settings.statusLine.command.noShell")}
                  {/if}
                </span>
              </p>
              {#if test.outcome === "failed" && test.detail}
                <pre class="stderr">{test.detail}</pre>
              {/if}
            {/if}
          {/if}
        </div>
      {/if}
    </div>
    {#if failure}
      <p class="failure">{t("settings.statusLine.saveFailed.format", failure)}</p>
    {/if}
  </section>
{/if}

<style>
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
    gap: 8px;
    padding: 12px 14px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .detail,
  .caption {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .caption {
    font-size: var(--font-small);
    color: var(--text-tertiary);
  }
  .label {
    font-size: var(--font-caption);
    font-weight: 600;
  }
  .failure {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--critical);
  }

  /* A prévia imita o terminal onde a linha aparece (o Windows Terminal, escuro
     de fábrica) nos dois temas: as cores da linha foram escolhidas para ele. */
  .terminal {
    min-height: 38px;
    padding: 9px 12px;
    border: 1px solid var(--border-strong);
    border-radius: 6px;
    background: #0c0c0c;
    color: #cccccc;
    font-family: var(--font-mono);
    font-size: var(--font-caption);
    line-height: 1.55;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .bold {
    font-weight: 700;
  }
  .nothing {
    color: #8a8a8a;
    font-family: var(--font);
    font-style: italic;
  }

  .items-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
  }
  .items {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 6px 12px;
  }
  .check {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: var(--font-caption);
    cursor: pointer;
  }
  .check input {
    margin: 0;
    accent-color: var(--accent);
  }
  .link {
    padding: 2px 0;
    border: none;
    background: none;
    color: var(--accent);
    font: inherit;
    font-size: var(--font-caption);
    cursor: pointer;
  }

  .command {
    display: flex;
    gap: 8px;
  }
  .command .field {
    flex: 1;
    min-width: 0;
    font-family: var(--font-mono);
    font-size: var(--font-caption);
  }
  .status {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: var(--font-caption);
    line-height: 1.45;
  }
  .status :global(svg) {
    flex: none;
    margin-top: 2px;
  }
  .ok {
    color: var(--calm);
  }
  .warn {
    color: var(--warning-text);
  }
  .ok span,
  .warn span {
    color: var(--text);
  }
  .stderr {
    max-height: 96px;
    margin: 0;
    padding: 6px 8px;
    overflow: auto;
    border-radius: 6px;
    background: var(--bg);
    color: var(--text-secondary);
    font-family: var(--font-mono);
    font-size: var(--font-small);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--track);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
