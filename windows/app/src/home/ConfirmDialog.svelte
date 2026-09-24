<script lang="ts">
  // Confirmação de ação destrutiva (≙ confirmationDialog). O texto tem de dizer
  // o que DE FATO acontece — o macOS afirmou por um tempo que remover a conta
  // não apagava o login, quando passou a apagar.
  import { t } from "../lib/i18n";
  import Modal from "../lib/Modal.svelte";
  import Rich from "../lib/Rich.svelte";

  let {
    title,
    message,
    confirm,
    destructive = true,
    onConfirm,
    onCancel,
  }: {
    title: string;
    message: string;
    confirm: string;
    destructive?: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  } = $props();
</script>

<Modal onClose={onCancel}>
  <div class="body">
    <h2>{title}</h2>
    <p><Rich text={message} /></p>
    <div class="actions">
      <button class="secondary" onclick={onCancel}>{t("groups.cancel")}</button>
      <button class={destructive ? "danger" : "primary"} onclick={onConfirm}>{confirm}</button>
    </div>
  </div>
</Modal>

<style>
  .body {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  p {
    font-size: var(--font-caption);
    line-height: 1.45;
    color: var(--text-secondary);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
