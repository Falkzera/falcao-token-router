<script module lang="ts">
  // O menu "⋯" das ações raras (≙ o `Menu` do macOS): o que é frequente fica
  // exposto, o resto atrás daqui. Fecha ao escolher, ao clicar fora e no Esc.
  export interface MenuItem {
    label: string;
    onSelect: () => void;
    destructive?: boolean;
    disabled?: boolean;
  }
</script>

<script lang="ts">
  let { items, title }: { items: MenuItem[]; title: string } = $props();

  let open = $state(false);
  let root: HTMLElement | undefined = $state();

  $effect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      if (root && !root.contains(event.target as Node)) open = false;
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") open = false;
    };
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("keydown", escape);
    };
  });

  function choose(item: MenuItem) {
    open = false;
    item.onSelect();
  }
</script>

<span class="menu" bind:this={root}>
  <button class="trigger" {title} aria-label={title} aria-haspopup="menu" aria-expanded={open} onclick={() => (open = !open)}>
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <circle cx="3.5" cy="8" r="1.25" />
      <circle cx="8" cy="8" r="1.25" />
      <circle cx="12.5" cy="8" r="1.25" />
    </svg>
  </button>
  {#if open}
    <div class="popover" role="menu">
      {#each items as item (item.label)}
        <button
          role="menuitem"
          class:destructive={item.destructive}
          disabled={item.disabled}
          onclick={() => choose(item)}
        >
          {item.label}
        </button>
      {/each}
    </div>
  {/if}
</span>

<style>
  .menu {
    position: relative;
    display: inline-flex;
  }
  .trigger {
    display: inline-flex;
    padding: 4px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }
  .trigger:hover,
  .trigger[aria-expanded="true"] {
    background: var(--row-hover);
    color: var(--text);
  }
  .popover {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    min-width: 200px;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.2);
  }
  .popover button {
    padding: 6px 10px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    font: inherit;
    font-size: var(--font-caption);
    text-align: left;
    white-space: nowrap;
    cursor: pointer;
  }
  .popover button:hover:not(:disabled) {
    background: var(--row-hover);
  }
  .popover button:disabled {
    color: var(--text-tertiary);
    cursor: default;
  }
  .popover .destructive {
    color: var(--critical);
  }
  button:focus-visible {
    outline: 2px solid var(--focus);
    outline-offset: -2px;
  }
</style>
