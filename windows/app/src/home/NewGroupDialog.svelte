<script lang="ts">
  // Grupo novo (≙ NewGroupSheet), com a escolha que o macOS não dava: usar ou
  // não o perfil padrão (`~\.claude`). O laço de rotação ativa a primeira conta
  // de um grupo sem ativa — num grupo padrão, isso troca o login que o
  // `~\.claude` tem hoje (com backup) em até 3 minutos. Por isso a opção vem
  // DESMARCADA quando lá há um login que o router não conhece, e o aviso diz
  // qual é.
  import { t } from "../lib/i18n";
  import Modal from "../lib/Modal.svelte";
  import Rich from "../lib/Rich.svelte";

  let {
    firstGroup,
    foreignLogin,
    onCreate,
    onCancel,
  }: {
    firstGroup: boolean;
    foreignLogin: string | null;
    onCreate: (name: string, asDefault: boolean) => void;
    onCancel: () => void;
  } = $props();

  let name = $state("");
  // svelte-ignore state_referenced_locally
  let asDefault = $state(firstGroup && foreignLogin === null);
  const valid = $derived(name.trim().length > 0);

  function create() {
    if (valid) onCreate(name.trim(), asDefault);
  }
</script>

<Modal onClose={onCancel}>
  <form
    class="body"
    onsubmit={(event) => {
      event.preventDefault();
      create();
    }}
  >
    <h2>{t("groups.new.title")}</h2>
    <p class="detail">{t("groups.new.detail")}</p>
    <!-- svelte-ignore a11y_autofocus -->
    <input
      class="field"
      type="text"
      placeholder={t("groups.name.placeholder")}
      bind:value={name}
      autofocus
    />
    <label class="check">
      <input type="checkbox" bind:checked={asDefault} />
      <span>{t("groups.new.default.toggle")}</span>
    </label>
    <p class="detail"><Rich text={t("groups.new.default.detail")} /></p>
    {#if asDefault && foreignLogin}
      <p class="warning">{t("groups.new.default.foreign.format", foreignLogin)}</p>
    {/if}
    <div class="actions">
      <button type="button" class="secondary" onclick={onCancel}>{t("groups.cancel")}</button>
      <button type="submit" class="primary" disabled={!valid}>{t("groups.new.create")}</button>
    </div>
  </form>
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .detail {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 4px;
    font-size: var(--font-caption);
  }
  .warning {
    padding: 8px 10px;
    border-radius: 6px;
    background: color-mix(in srgb, var(--warning) 16%, transparent);
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
