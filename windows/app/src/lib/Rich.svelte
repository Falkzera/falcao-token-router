<script lang="ts">
  // Texto do catálogo com `código` entre crases — o que o SwiftUI fazia sozinho
  // (markdown no LocalizedStringKey). Monta elementos, não HTML: o texto nunca
  // é interpretado como marcação.
  let { text }: { text: string } = $props();

  const parts = $derived(text.split("`"));
</script>

{#each parts as part, index (index)}{#if index % 2 === 1}<code>{part}</code>{:else}{part}{/if}{/each}

<style>
  code {
    padding: 0 3px;
    border-radius: 3px;
    background: var(--row-active);
    font-family: var(--font-mono);
    font-size: 0.95em;
  }
</style>
