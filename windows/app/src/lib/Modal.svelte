<script lang="ts">
  // Um diálogo modal da janela — o `<dialog>` nativo com `showModal()`: foco
  // preso dentro, Esc fecha (evento `cancel`), fundo esmaecido. Clicar fora NÃO
  // fecha, como nos diálogos do Windows: confirmação destrutiva não pode sumir
  // por um clique errado.
  import type { Snippet } from "svelte";

  let { onClose, children }: { onClose: () => void; children: Snippet } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  $effect(() => {
    if (dialog && !dialog.open) dialog.showModal();
  });
</script>

<dialog
  bind:this={dialog}
  oncancel={(event) => {
    event.preventDefault();
    onClose();
  }}
>
  {@render children()}
</dialog>

<style>
  dialog {
    width: min(400px, calc(100vw - 48px));
    padding: 20px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 16px 48px rgba(0, 0, 0, 0.28);
  }
  dialog::backdrop {
    background: rgba(0, 0, 0, 0.32);
  }
</style>
