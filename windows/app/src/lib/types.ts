// Os tipos que o backend manda — espelho dos `#[derive(Serialize)]` do Rust
// (camelCase). Mudou lá, muda aqui: o `svelte-check` pega o resto do front.

export type Locale = "en" | "pt-BR";

export interface AppInfo {
  version: string;
  locale: Locale;
}
