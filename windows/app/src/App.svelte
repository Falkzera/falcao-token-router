<script lang="ts">
  // A raiz: decide qual superfície esta janela mostra e segura a página até o
  // idioma chegar do backend (ele manda o do Windows; o front não adivinha).
  import { onMount } from "svelte";
  import { appInfo } from "./lib/api";
  import { setLocale } from "./lib/i18n";
  import type { AppInfo } from "./lib/types";
  import Home from "./home/Home.svelte";

  let info = $state<AppInfo | null>(null);

  onMount(async () => {
    const loaded = await appInfo();
    setLocale(loaded.locale);
    document.documentElement.lang = loaded.locale;
    info = loaded;
  });
</script>

{#if info}
  <Home {info} />
{/if}
