<script lang="ts">
  // A raiz: decide qual superfície esta janela mostra (a janela `home` ou o
  // flyout da bandeja) e segura a página até o idioma chegar do backend (ele
  // manda o do Windows; o front não adivinha).
  import { onMount } from "svelte";
  import { appInfo, currentView, insideTauri } from "./lib/api";
  import { setLocale } from "./lib/i18n";
  import type { AppInfo } from "./lib/types";
  import Home from "./home/Home.svelte";
  import Panel from "./panel/Panel.svelte";

  const view = currentView();
  let info = $state<AppInfo | null>(null);

  onMount(async () => {
    document.documentElement.dataset.view = view;
    // No navegador a janela é simulada numa moldura do tamanho de fábrica.
    document.documentElement.dataset.host = insideTauri ? "app" : "browser";
    const loaded = await appInfo();
    setLocale(loaded.locale);
    document.documentElement.lang = loaded.locale;
    info = loaded;
  });
</script>

{#if info}
  {#if view === "flyout"}
    <Panel />
  {:else}
    <Home {info} />
  {/if}
{/if}
