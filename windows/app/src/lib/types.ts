// Os tipos que o backend manda — espelho dos `#[derive(Serialize)]` do Rust
// (camelCase). Mudou lá, muda aqui: o `svelte-check` pega o resto do front.

export type Locale = "en" | "pt-BR";

export type HomeTab = "groups" | "settings";

export interface AppInfo {
  version: string;
  locale: Locale;
  /** A aba com que a janela abre (a bandeja pode ter pedido Ajustes). */
  initialTab: HomeTab;
}
