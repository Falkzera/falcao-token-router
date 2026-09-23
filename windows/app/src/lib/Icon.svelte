<script module lang="ts">
  // Os ícones do app, desenhados aqui (traço de 1,25 em 16×16, `currentColor`)
  // — os SF Symbols do macOS não existem no Windows e uma fonte de ícones
  // inteira seria dependência para meia dúzia de desenhos.
  export type IconName = "stack" | "gear" | "close" | "clockAlert" | "sensor" | "probe";
</script>

<script lang="ts">
  let { name, size = 14 }: { name: IconName; size?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.25"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if name === "stack"}
    <rect x="2.5" y="6" width="11" height="7.5" rx="1.5" />
    <path d="M4 3.75h8" />
    <path d="M5.5 1.75h5" />
  {:else if name === "gear"}
    <path
      d="M8 1.6l1.1 1.5 1.9-.4.4 1.9 1.8.9-.7 1.8.7 1.8-1.8.9-.4 1.9-1.9-.4L8 14.4l-1.1-1.5-1.9.4-.4-1.9-1.8-.9.7-1.8-.7-1.8 1.8-.9.4-1.9 1.9.4z"
    />
    <circle cx="8" cy="8" r="2" />
  {:else if name === "close"}
    <path d="M4.5 4.5l7 7M11.5 4.5l-7 7" />
  {:else if name === "clockAlert"}
    <circle cx="7" cy="8.5" r="5" />
    <path d="M7 6v2.5l1.75 1.25" />
    <path d="M14 3.5v3.5" />
    <path d="M14 9.5v.25" />
  {:else if name === "sensor"}
    <circle cx="8" cy="8" r="1.25" fill="currentColor" stroke="none" />
    <path d="M5.25 5.25a3.9 3.9 0 0 0 0 5.5M10.75 5.25a3.9 3.9 0 0 1 0 5.5" />
    <path d="M3.25 3.25a6.7 6.7 0 0 0 0 9.5M12.75 3.25a6.7 6.7 0 0 1 0 9.5" />
  {:else if name === "probe"}
    <path d="M2.5 11.5a5.5 5.5 0 1 1 11 0" />
    <path d="M8 11.5l2.4-3.4" />
    <circle cx="8" cy="11.5" r="1" fill="currentColor" stroke="none" />
  {/if}
</svg>

<style>
  svg {
    flex: none;
    display: block;
  }
</style>
